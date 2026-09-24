//! 魔法陣を端末に画像として表示する。
//!
//! Kitty graphics protocol（Ghostty / Kitty）と iTerm2 inline images（iTerm2 / WezTerm）に対応する。
//! どちらも使えない端末では何も描かず、呼び出し側のテキスト表示だけが残る。

use std::io::{self, IsTerminal, Write};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;

/// 1 行ぶんの高さに割り当てるピクセル数。16 行なら 512px になる。
const PIXELS_PER_ROW: u32 = 32;
/// 画像の一辺の上限。これより大きく出すときは端末に引き伸ばしてもらう。
/// 1152px（36 行）を焼くと展開が 2 秒を超え、コマンドを待たせてしまう。
const MAX_PIXELS: u32 = 768;
/// Kitty に送る画像の ID。コマを差し替えるために毎回同じ値を使う。
const IMAGE_ID: u32 = 0x6d61; // "ma"

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Kitty,
    Iterm,
    None,
}

/// 環境変数から使えるプロトコルを決める。`MAHO_GRAPHICS` があればそれが勝つ。
pub fn detect(env: impl Fn(&str) -> Option<String>) -> Protocol {
    if let Some(v) = env("MAHO_GRAPHICS") {
        return match v.as_str() {
            "kitty" => Protocol::Kitty,
            "iterm" => Protocol::Iterm,
            _ => Protocol::None,
        };
    }
    // tmux はエスケープシーケンスを素通ししないので、外側の端末が対応していても描けない。
    if env("TMUX").is_some() {
        return Protocol::None;
    }
    match env("TERM_PROGRAM").as_deref() {
        Some("ghostty") => return Protocol::Kitty,
        Some("iTerm.app") | Some("WezTerm") => return Protocol::Iterm,
        _ => {}
    }
    if env("LC_TERMINAL").as_deref() == Some("iTerm2") {
        return Protocol::Iterm;
    }
    if env("KITTY_WINDOW_ID").is_some() || env("TERM").as_deref() == Some("xterm-kitty") {
        return Protocol::Kitty;
    }
    Protocol::None
}

/// SVG のコマを順に PNG にして、stderr の同じ場所へ高さ `rows` 行で描き直していく。
/// 幅は縦横比から端末が決める。コマが 1 枚なら静止画になる。描いたら `true`、描けなかったら理由を返す。
/// stderr が端末でないとき（リダイレクト中など）は何もせず `false` を返す。
pub fn play(
    frames: &[String],
    interval: Duration,
    protocol: Protocol,
    rows: u32,
) -> Result<bool, String> {
    if protocol == Protocol::None || !io::stderr().is_terminal() || frames.is_empty() {
        return Ok(false);
    }
    // 1 コマの変換に 0.1 秒ほどかかる。全コマを並列で焼きつつ、焼けた順に
    // 決まった間隔で流す。全部焼いてから流すと、その時間ぶん待たされる。
    let pngs = rasterize_in_background(frames, (rows * PIXELS_PER_ROW).min(MAX_PIXELS));

    let mut err = io::stderr().lock();
    let mut out = |bytes: &[u8]| err.write_all(bytes).and_then(|_| err.flush());
    let io = |e: io::Error| e.to_string();

    // 先に画像の高さぶん改行して場所を空け、その先頭に戻ってカーソル位置を覚える。
    // 画面の最下行で描き始めても、画像が下にはみ出さない。
    out(format!("{}\x1b[{rows}A\r\x1b7", "\n".repeat(rows as usize)).as_bytes()).map_err(io)?;
    let mut next = Instant::now();
    for rx in pngs {
        let png = rx.recv().map_err(|e| e.to_string())??;
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
        let seq = match protocol {
            Protocol::Kitty => kitty(&png, rows),
            Protocol::Iterm => iterm(&png, rows),
            Protocol::None => unreachable!(),
        };
        out(format!("\x1b8{seq}").as_bytes()).map_err(io)?;
        next = Instant::now() + interval;
    }
    // 画像の下の行へ抜ける
    out(format!("\x1b8\x1b[{rows}B\r").as_bytes()).map_err(io)?;
    Ok(true)
}

/// コマごとの受け口を返し、裏のスレッドで先頭のコマから順に焼いていく。
fn rasterize_in_background(
    frames: &[String],
    pixels: u32,
) -> Vec<Receiver<Result<Vec<u8>, String>>> {
    let (txs, rxs): (Vec<_>, Vec<_>) = frames.iter().map(|_| mpsc::channel()).unzip();
    let jobs: Arc<Mutex<_>> = Arc::new(Mutex::new(
        frames
            .iter()
            .cloned()
            .zip(txs)
            .collect::<std::collections::VecDeque<_>>(),
    ));
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    for _ in 0..workers.min(frames.len()) {
        let jobs = Arc::clone(&jobs);
        std::thread::spawn(move || {
            loop {
                let Some((svg, tx)) = jobs.lock().unwrap().pop_front() else {
                    return;
                };
                // 受け手が先に抜けていたら送れないが、それで困ることはない
                let _ = tx.send(rasterize(&svg, pixels));
            }
        });
    }
    rxs
}

/// SVG を一辺 `pixels` の PNG にする。
pub fn rasterize(svg: &str, pixels: u32) -> Result<Vec<u8>, String> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
        .map_err(|e| e.to_string())?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixels, pixels).ok_or("pixmap の確保に失敗")?;
    let scale = pixels as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|e| e.to_string())
}

/// Kitty graphics protocol。ペイロードは 4096 バイトずつに分けて送る決まり。
/// `q=2` で端末からの応答を止める。止めないと応答が子プロセスの stdin に混ざる。
/// 毎コマ同じ画像 ID で送ると前のコマが消えて差し替わる。`C=1` でカーソルを動かさない。
fn kitty(png: &[u8], rows: u32) -> String {
    let data = B64.encode(png);
    let chunks: Vec<&str> = data
        .as_bytes()
        .chunks(4096)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect();
    let mut out = String::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = (i + 1 < chunks.len()) as u8;
        if i == 0 {
            out.push_str(&format!(
                "\x1b_Gf=100,a=T,i={IMAGE_ID},p=1,C=1,q=2,r={rows},m={more};{chunk}\x1b\\"
            ));
        } else {
            out.push_str(&format!("\x1b_Gm={more};{chunk}\x1b\\"));
        }
    }
    out
}

/// iTerm2 inline images protocol
fn iterm(png: &[u8], rows: u32) -> String {
    format!(
        "\x1b]1337;File=inline=1;size={};height={rows};preserveAspectRatio=1:{}\x07",
        png.len(),
        B64.encode(png)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| {
            vars.iter()
                .find(|(name, _)| *name == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn detects_terminals() {
        assert_eq!(detect(env(&[("TERM_PROGRAM", "ghostty")])), Protocol::Kitty);
        assert_eq!(
            detect(env(&[("TERM_PROGRAM", "iTerm.app")])),
            Protocol::Iterm
        );
        assert_eq!(detect(env(&[("TERM", "xterm-kitty")])), Protocol::Kitty);
        assert_eq!(
            detect(env(&[("TERM_PROGRAM", "Apple_Terminal")])),
            Protocol::None
        );
    }

    #[test]
    fn tmux_and_override() {
        assert_eq!(
            detect(env(&[("TERM_PROGRAM", "ghostty"), ("TMUX", "/tmp/x")])),
            Protocol::None
        );
        assert_eq!(
            detect(env(&[("TMUX", "/tmp/x"), ("MAHO_GRAPHICS", "kitty")])),
            Protocol::Kitty
        );
        assert_eq!(
            detect(env(&[
                ("TERM_PROGRAM", "ghostty"),
                ("MAHO_GRAPHICS", "none")
            ])),
            Protocol::None
        );
    }

    #[test]
    fn kitty_chunks_are_well_formed() {
        let seq = kitty(&vec![0u8; 10_000], 16);
        let parts: Vec<&str> = seq.split("\x1b\\").filter(|p| !p.is_empty()).collect();
        assert!(parts.len() > 1);
        assert!(parts[0].starts_with("\x1b_Gf=100,a=T,"));
        assert!(parts[0].contains(",C=1,q=2,r=16,"));
        assert!(parts[0].contains("m=1;"));
        assert!(parts.last().unwrap().starts_with("\x1b_Gm=0;"));
    }

    #[test]
    fn rasterizes_to_png() {
        let svg = crate::render::svg(
            &crate::circle::MagicCircle::from_command("git status"),
            "git status",
        );
        let png = rasterize(&svg, 512).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
}
