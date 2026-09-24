//! 魔法陣を端末に画像として表示する。
//!
//! Kitty graphics protocol（Ghostty / Kitty）と iTerm2 inline images（iTerm2 / WezTerm）に対応する。
//! どちらも使えない端末では何も描かず、呼び出し側のテキスト表示だけが残る。
//!
//! tmux の中では、`allow-passthrough` が有効なら外側の端末へ素通しで送る。
//! Kitty は Unicode placeholder を使い、画像を文字のマスに結び付ける。tmux が画面を描き直しても
//! 画像がずれたり残ったりしない。iTerm2 にはその仕組みが無いので、外側の画面の位置を計算して置く。

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
/// Unicode placeholder の文字と、行・列の番号を表す結合文字（kitty の rowcolumn-diacritics.txt の先頭）。
/// 魔法陣は最大 48 行（神聖魔法）なので、それだけあれば足りる。
const PLACEHOLDER: char = '\u{10EEEE}';
const DIACRITICS: [char; 48] = [
    '\u{0305}', '\u{030D}', '\u{030E}', '\u{0310}', '\u{0312}', '\u{033D}', '\u{033E}', '\u{033F}',
    '\u{0346}', '\u{034A}', '\u{034B}', '\u{034C}', '\u{0350}', '\u{0351}', '\u{0352}', '\u{0357}',
    '\u{035B}', '\u{0363}', '\u{0364}', '\u{0365}', '\u{0366}', '\u{0367}', '\u{0368}', '\u{0369}',
    '\u{036A}', '\u{036B}', '\u{036C}', '\u{036D}', '\u{036E}', '\u{036F}', '\u{0483}', '\u{0484}',
    '\u{0485}', '\u{0486}', '\u{0487}', '\u{0592}', '\u{0593}', '\u{0594}', '\u{0595}', '\u{0597}',
    '\u{0598}', '\u{0599}', '\u{059C}', '\u{059D}', '\u{059E}', '\u{059F}', '\u{05A0}', '\u{05A1}',
];

/// Kitty に送る画像の ID。1 回の展開のあいだは同じ ID でコマを差し替え、展開ごとには変える。
/// 同じ ID で送り直すと前の画像が消えるので、固定すると、前に唱えた魔法陣や、
/// 砕ける前に展開した魔法陣まで消えてしまう。
fn fresh_id() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static CALLS: AtomicU32 = AtomicU32::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let mixed = (std::process::id().wrapping_mul(0x9e37_79b9) ^ nanos).wrapping_add(
        CALLS
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x85eb_ca6b),
    );
    mixed.max(1)
}

/// tmux の中で Unicode placeholder に使う画像の ID。文字色（256 色）で伝えるので 1〜255 に収める。
/// 前の魔法陣と重なる見込みは 255 分の 1 になる。
fn placeholder_id(id: u32) -> u32 {
    1 + id % 255
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Kitty,
    Iterm,
    None,
}

/// 描き先。tmux の中なら、外側の端末へ送るのに要るペインの情報も持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub protocol: Protocol,
    pub tmux: Option<Pane>,
}

/// tmux のペインが外側の画面のどこにあり、カーソルがどこにいるか（0 始まり）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pane {
    top: u32,
    left: u32,
    width: u32,
    height: u32,
    cursor_y: u32,
    /// 1 マスの幅と高さのピクセル数。tmux が知らなければ `None`
    cell: Option<(u32, u32)>,
}

/// tmux に 1 回だけ問い合わせる項目。`#{allow-passthrough}` のようにオプションも引ける。
pub const TMUX_FORMAT: &str = "#{allow-passthrough}\t#{client_termtype}\t#{client_termname}\t\
#{pane_top}\t#{pane_left}\t#{pane_width}\t#{pane_height}\t#{cursor_y}\t\
#{client_cell_width}\t#{client_cell_height}";

/// 今の tmux に `TMUX_FORMAT` を問い合わせる。
pub fn ask_tmux() -> Option<String> {
    let out = std::process::Command::new("tmux")
        .args(["display-message", "-p", TMUX_FORMAT])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

/// 環境変数から使えるプロトコルを決める。`MAHOJIN_GRAPHICS` があればそれが勝つ。
/// tmux の中では `tmux` で問い合わせた結果（[`TMUX_FORMAT`] の値）から、外側の端末と
/// 素通しの可否を見る。素通しが無効なら描かない。
pub fn detect(
    env: impl Fn(&str) -> Option<String>,
    tmux: impl FnOnce() -> Option<String>,
) -> Target {
    let forced = env("MAHOJIN_GRAPHICS").map(|v| match v.as_str() {
        "kitty" => Protocol::Kitty,
        "iterm" => Protocol::Iterm,
        _ => Protocol::None,
    });
    if env("TMUX").is_none() {
        let protocol = forced.unwrap_or_else(|| outer_terminal(&env, ""));
        return Target {
            protocol,
            tmux: None,
        };
    }
    let none = Target {
        protocol: Protocol::None,
        tmux: None,
    };
    let Some(answer) = tmux() else {
        return none;
    };
    let f: Vec<&str> = answer.split('\t').collect();
    let field = |i: usize| f.get(i).copied().unwrap_or("");
    let num = |i: usize| field(i).parse::<u32>().ok();
    // 素通しが無効だと、送っても tmux が捨てる
    if forced.is_none() && !matches!(field(0), "on" | "all") {
        return none;
    }
    let (Some(top), Some(left), Some(width), Some(height), Some(cursor_y)) =
        (num(3), num(4), num(5), num(6), num(7))
    else {
        return none;
    };
    let cell = num(8).zip(num(9)).filter(|&(w, h)| w > 0 && h > 0);
    let outer = format!("{} {}", field(1), field(2));
    Target {
        protocol: forced.unwrap_or_else(|| outer_terminal(&env, &outer)),
        tmux: Some(Pane {
            top,
            left,
            width,
            height,
            cursor_y,
            cell,
        }),
    }
}

/// 外側の端末を当てる。`outer` は tmux が知っている端末の名前（tmux の外なら空）。
/// tmux の中の `TERM_PROGRAM` は tmux 自身なので、tmux の答えと、tmux の起動時から残る環境変数で見る。
fn outer_terminal(env: &impl Fn(&str) -> Option<String>, outer: &str) -> Protocol {
    let outer = outer.to_ascii_lowercase();
    if outer.contains("ghostty") || outer.contains("kitty") {
        return Protocol::Kitty;
    }
    if outer.contains("iterm") || outer.contains("wezterm") {
        return Protocol::Iterm;
    }
    match env("TERM_PROGRAM").as_deref() {
        Some("ghostty") => return Protocol::Kitty,
        Some("iTerm.app") | Some("WezTerm") => return Protocol::Iterm,
        _ => {}
    }
    if env("LC_TERMINAL").as_deref() == Some("iTerm2") {
        return Protocol::Iterm;
    }
    if env("KITTY_WINDOW_ID").is_some()
        || env("GHOSTTY_RESOURCES_DIR").is_some()
        || env("TERM").as_deref() == Some("xterm-kitty")
    {
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
    target: Target,
    rows: u32,
) -> Result<bool, String> {
    let protocol = target.protocol;
    if protocol == Protocol::None || !io::stderr().is_terminal() || frames.is_empty() {
        return Ok(false);
    }
    // tmux のペインより高いと、描いているうちに流れてしまう
    let rows = match target.tmux {
        Some(p) => rows.min(p.height.saturating_sub(1)).max(1),
        None => rows,
    };
    // 1 コマの変換に 0.1 秒ほどかかる。全コマを並列で焼きつつ、焼けた順に
    // 決まった間隔で流す。全部焼いてから流すと、その時間ぶん待たされる。
    let pngs = rasterize_in_background(frames, (rows * PIXELS_PER_ROW).min(MAX_PIXELS));
    let id = fresh_id();

    let mut err = io::stderr().lock();
    let mut out = |bytes: &[u8]| err.write_all(bytes).and_then(|_| err.flush());
    let io = |e: io::Error| e.to_string();

    // 先に画像の高さぶん改行して場所を空け、その先頭に戻ってカーソル位置を覚える。
    // 画面の最下行で描き始めても、画像が下にはみ出さない。
    out(format!("{}\x1b[{rows}A\r\x1b7", "\n".repeat(rows as usize)).as_bytes()).map_err(io)?;
    // tmux で Kitty なら、画像を載せるマスを先に文字で敷いておく
    let cols = target.tmux.map(|p| placeholder_cols(p, rows));
    if let (Some(cols), Protocol::Kitty) = (cols, protocol) {
        out(placeholders(rows, cols, placeholder_id(id)).as_bytes()).map_err(io)?;
    }
    let mut next = Instant::now();
    for rx in pngs {
        let png = rx.recv().map_err(|e| e.to_string())??;
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
        let seq = match (target.tmux, protocol) {
            (None, Protocol::Kitty) => format!("\x1b8{}", kitty(&png, rows, id).concat()),
            (None, Protocol::Iterm) => format!("\x1b8{}", iterm(&png, rows)),
            (Some(_), Protocol::Kitty) => {
                kitty_virtual(&png, rows, cols.unwrap_or(rows * 2), placeholder_id(id))
                    .iter()
                    .map(|s| passthrough(s))
                    .collect()
            }
            (Some(pane), Protocol::Iterm) => {
                // tmux は画像を知らないので、外側の画面での位置を直に指して描き、カーソルを戻す
                let (row, col) = pane.origin(rows);
                passthrough(&format!("\x1b7\x1b[{row};{col}H{}\x1b8", iterm(&png, rows)))
            }
            (_, Protocol::None) => unreachable!(),
        };
        out(seq.as_bytes()).map_err(io)?;
        next = Instant::now() + interval;
    }
    // 画像の下の行へ抜ける
    out(format!("\x1b8\x1b[{rows}B\r").as_bytes()).map_err(io)?;
    Ok(true)
}

impl Pane {
    /// 場所を空けたあとの、画像の左上のマス（外側の画面での 1 始まりの行と列）。
    /// 改行で場所を空けると、画面の下端ではペインが流れるぶん上にずれる。
    fn origin(self, rows: u32) -> (u32, u32) {
        let y = self
            .cursor_y
            .min(self.height.saturating_sub(1).saturating_sub(rows));
        (self.top + y + 1, self.left + 1)
    }
}

/// 画像を載せるマスの列数。正方形の画像が収まるように、マスの縦横比から決める。
/// tmux がマスの大きさを知らなければ、よくある縦 2 : 横 1 とみなす。
fn placeholder_cols(pane: Pane, rows: u32) -> u32 {
    let cols = match pane.cell {
        Some((w, h)) => (rows * h).div_ceil(w),
        None => rows * 2,
    };
    cols.clamp(1, pane.width.max(1))
}

/// Unicode placeholder を `rows` 行 × `cols` 列敷く。文字色で画像 ID（`id`）を伝える。
/// 各行の先頭のマスにだけ行と列の番号を付ける。後ろのマスは左隣から続きの列とみなされる。
fn placeholders(rows: u32, cols: u32, id: u32) -> String {
    let mut s = String::new();
    for r in 0..rows as usize {
        s.push_str(&format!("\x1b[38;5;{id}m"));
        s.push(PLACEHOLDER);
        s.push(DIACRITICS[r.min(DIACRITICS.len() - 1)]);
        s.push(DIACRITICS[0]);
        s.extend(std::iter::repeat_n(PLACEHOLDER, cols as usize - 1));
        s.push_str("\x1b[39m");
        if r + 1 < rows as usize {
            s.push_str("\r\n");
        }
    }
    s
}

/// tmux の素通し。中の ESC は 2 つ重ねる決まり。
fn passthrough(seq: &str) -> String {
    format!("\x1bPtmux;{}\x1b\\", seq.replace('\x1b', "\x1b\x1b"))
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
fn kitty(png: &[u8], rows: u32, id: u32) -> Vec<String> {
    kitty_chunks(png, &format!("f=100,a=T,i={id},p=1,C=1,q=2,r={rows}"))
}

/// tmux 用。画像を送ると同時に、敷いておいた placeholder に載せる仮想の置き場所を作る（`U=1`）。
/// 同じ ID で送り直すと前の置き場所は消えるので、コマごとに作り直す。
fn kitty_virtual(png: &[u8], rows: u32, cols: u32, id: u32) -> Vec<String> {
    kitty_chunks(png, &format!("f=100,a=T,U=1,i={id},q=2,c={cols},r={rows}"))
}

/// 1 つの画像を、先頭だけに `keys` を付けたエスケープシーケンスの列にする。
fn kitty_chunks(png: &[u8], keys: &str) -> Vec<String> {
    let data = B64.encode(png);
    let chunks: Vec<&str> = data
        .as_bytes()
        .chunks(4096)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect();
    chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let more = (i + 1 < chunks.len()) as u8;
            if i == 0 {
                format!("\x1b_G{keys},m={more};{chunk}\x1b\\")
            } else {
                format!("\x1b_Gm={more};{chunk}\x1b\\")
            }
        })
        .collect()
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

    /// tmux の外で決まるプロトコル
    fn plain(vars: &[(&str, &str)]) -> Protocol {
        let t = detect(env(vars), || panic!("tmux の外で tmux に問い合わせた"));
        assert_eq!(t.tmux, None);
        t.protocol
    }

    #[test]
    fn detects_terminals() {
        assert_eq!(plain(&[("TERM_PROGRAM", "ghostty")]), Protocol::Kitty);
        assert_eq!(plain(&[("TERM_PROGRAM", "iTerm.app")]), Protocol::Iterm);
        assert_eq!(plain(&[("TERM", "xterm-kitty")]), Protocol::Kitty);
        assert_eq!(plain(&[("TERM_PROGRAM", "Apple_Terminal")]), Protocol::None);
        assert_eq!(
            plain(&[("TERM_PROGRAM", "ghostty"), ("MAHOJIN_GRAPHICS", "none")]),
            Protocol::None
        );
    }

    const IN_TMUX: &[(&str, &str)] = &[("TMUX", "/tmp/x"), ("TERM_PROGRAM", "tmux")];

    fn answer(passthrough: &str, termtype: &str, termname: &str) -> Option<String> {
        Some(format!(
            "{passthrough}\t{termtype}\t{termname}\t1\t0\t80\t40\t10\t10\t20"
        ))
    }

    #[test]
    fn tmux_reaches_the_outer_terminal() {
        let t = detect(env(IN_TMUX), || {
            answer("on", "ghostty 1.2.0", "xterm-ghostty")
        });
        assert_eq!(t.protocol, Protocol::Kitty);
        let pane = t.tmux.unwrap();
        assert_eq!((pane.top, pane.height, pane.cursor_y), (1, 40, 10));
        assert_eq!(pane.cell, Some((10, 20)));
        let t = detect(env(IN_TMUX), || {
            answer("all", "iTerm2 3.5.0", "xterm-256color")
        });
        assert_eq!(t.protocol, Protocol::Iterm);
        // tmux が端末の種類を知らなくても、起動時の環境変数から当てる
        let mut vars = IN_TMUX.to_vec();
        vars.push(("LC_TERMINAL", "iTerm2"));
        let t = detect(env(&vars), || answer("on", "", "xterm-256color"));
        assert_eq!(t.protocol, Protocol::Iterm);
    }

    #[test]
    fn tmux_without_passthrough_draws_nothing() {
        let t = detect(env(IN_TMUX), || {
            answer("off", "ghostty 1.2.0", "xterm-ghostty")
        });
        assert_eq!(t.protocol, Protocol::None);
        // tmux に聞けなかったときも描かない
        assert_eq!(detect(env(IN_TMUX), || None).protocol, Protocol::None);
        // 強制すれば、素通しの設定を見ずに送る
        let mut vars = IN_TMUX.to_vec();
        vars.push(("MAHOJIN_GRAPHICS", "kitty"));
        let t = detect(env(&vars), || answer("off", "", ""));
        assert_eq!(t.protocol, Protocol::Kitty);
        assert!(t.tmux.is_some());
    }

    #[test]
    fn placeholders_cover_the_image() {
        let s = placeholders(3, 5, 42);
        assert_eq!(s.matches(PLACEHOLDER).count(), 15);
        // 行ごとに先頭のマスだけ、行番号と列 0 の結合文字が付く
        for (r, line) in s.split("\r\n").enumerate() {
            assert!(line.starts_with("\x1b[38;5;42m"));
            let mut chars = line.chars().skip_while(|&c| c != PLACEHOLDER).skip(1);
            assert_eq!(chars.next(), Some(DIACRITICS[r]));
            assert_eq!(chars.next(), Some(DIACRITICS[0]));
        }
        let pane = Pane {
            top: 0,
            left: 0,
            width: 30,
            height: 40,
            cursor_y: 0,
            cell: Some((10, 20)),
        };
        assert_eq!(placeholder_cols(pane, 8), 16);
        // ペインより広くはしない
        assert_eq!(placeholder_cols(pane, 24), 30);
        assert_eq!(placeholder_cols(Pane { cell: None, ..pane }, 8), 16);
    }

    #[test]
    fn every_row_of_the_largest_circle_has_its_own_number() {
        let mut holy = crate::circle::MagicCircle::from_command("x");
        holy.sanctify();
        assert!(DIACRITICS.len() >= holy.rows() as usize);
        let rows: std::collections::HashSet<char> = DIACRITICS.into_iter().collect();
        assert_eq!(rows.len(), DIACRITICS.len());
    }

    #[test]
    fn each_unfolding_gets_its_own_image() {
        let (a, b) = (fresh_id(), fresh_id());
        assert!(a != 0 && b != 0 && a != b);
        for id in [a, b, 0, 254, 255, u32::MAX] {
            assert!((1..=255).contains(&placeholder_id(id)), "{id}");
        }
    }

    #[test]
    fn origin_follows_scrolling() {
        let pane = Pane {
            top: 1,
            left: 40,
            width: 80,
            height: 40,
            cursor_y: 5,
            cell: None,
        };
        assert_eq!(pane.origin(16), (7, 41));
        // 下端近くでは、空けた行ぶんペインが流れて上にずれる
        assert_eq!(
            Pane {
                cursor_y: 38,
                ..pane
            }
            .origin(16),
            (1 + 23 + 1, 41)
        );
    }

    #[test]
    fn passthrough_doubles_escapes() {
        assert_eq!(
            passthrough("\x1b_Ga\x1b\\"),
            "\x1bPtmux;\x1b\x1b_Ga\x1b\x1b\\\x1b\\"
        );
        let chunks = kitty_virtual(&vec![0u8; 10_000], 16, 32, 109);
        assert!(chunks[0].starts_with("\x1b_Gf=100,a=T,U=1,i=109,q=2,c=32,r=16,m=1;"));
        assert!(chunks.iter().all(|c| c.ends_with("\x1b\\")));
    }

    #[test]
    fn kitty_chunks_are_well_formed() {
        let seq = kitty(&vec![0u8; 10_000], 16, 7).concat();
        let parts: Vec<&str> = seq.split("\x1b\\").filter(|p| !p.is_empty()).collect();
        assert!(parts.len() > 1);
        assert!(parts[0].starts_with("\x1b_Gf=100,a=T,i=7,"));
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
