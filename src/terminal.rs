//! 魔法陣を端末に画像として表示する。
//!
//! Kitty graphics protocol（Ghostty / Kitty）と iTerm2 inline images（iTerm2 / WezTerm）に対応する。
//! どちらも使えない端末では何も描かず、呼び出し側のテキスト表示だけが残る。

use std::io::{self, IsTerminal, Write};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;

/// 画像の高さ（端末の行数）。幅は縦横比から端末が決める。
const ROWS: u32 = 16;
/// PNG に落とすときの一辺のピクセル数
const PIXELS: u32 = 640;

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

/// SVG を PNG にして stderr に描く。描けなかったら理由を返す。
/// stderr が端末でないとき（リダイレクト中など）は黙って何もしない。
pub fn show(svg: &str, protocol: Protocol) -> Result<(), String> {
    if protocol == Protocol::None || !io::stderr().is_terminal() {
        return Ok(());
    }
    let png = rasterize(svg)?;
    let seq = match protocol {
        Protocol::Kitty => kitty(&png),
        Protocol::Iterm => iterm(&png),
        Protocol::None => unreachable!(),
    };
    let mut err = io::stderr().lock();
    err.write_all(seq.as_bytes())
        .and_then(|_| writeln!(err))
        .and_then(|_| err.flush())
        .map_err(|e| e.to_string())
}

fn rasterize(svg: &str) -> Result<Vec<u8>, String> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
        .map_err(|e| e.to_string())?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(PIXELS, PIXELS).ok_or("pixmap の確保に失敗")?;
    let scale = PIXELS as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|e| e.to_string())
}

/// Kitty graphics protocol。ペイロードは 4096 バイトずつに分けて送る決まり。
/// `q=2` で端末からの応答を止める。止めないと応答が子プロセスの stdin に混ざる。
fn kitty(png: &[u8]) -> String {
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
                "\x1b_Gf=100,a=T,q=2,r={ROWS},m={more};{chunk}\x1b\\"
            ));
        } else {
            out.push_str(&format!("\x1b_Gm={more};{chunk}\x1b\\"));
        }
    }
    out
}

/// iTerm2 inline images protocol
fn iterm(png: &[u8]) -> String {
    format!(
        "\x1b]1337;File=inline=1;size={};height={ROWS};preserveAspectRatio=1:{}\x07",
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
        let seq = kitty(&vec![0u8; 10_000]);
        let parts: Vec<&str> = seq.split("\x1b\\").filter(|p| !p.is_empty()).collect();
        assert!(parts.len() > 1);
        assert!(parts[0].starts_with("\x1b_Gf=100,a=T,q=2,"));
        assert!(parts[0].contains("m=1;"));
        assert!(parts.last().unwrap().starts_with("\x1b_Gm=0;"));
    }

    #[test]
    fn rasterizes_to_png() {
        let svg = crate::render::svg(&crate::circle::MagicCircle::from_command("git status"));
        let png = rasterize(&svg).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
}
