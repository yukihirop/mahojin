//! コマンド文字列を架空の魔法文字で円周に書く。
//!
//! 字形はコマンドのハッシュではなく固定の種から引く。同じ文字はどの魔法陣でも
//! 同じ形になり、本物の文字体系のように見える。UTF-8 のバイトごとに 1 字。

use std::f32::consts::TAU;
use std::fmt::Write;

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};

/// 字母の種。変えると全部の魔法陣の文字が変わる。
const ALPHABET_SEED: [u8; 32] = *b"maho: the script of every spell.";

/// 字形を置く 2×3 の格子（横 ±1、縦 ±1）
const GRID: [(f32, f32); 6] = [
    (-1.0, -1.0),
    (1.0, -1.0),
    (-1.0, 0.0),
    (1.0, 0.0),
    (-1.0, 1.0),
    (1.0, 1.0),
];

/// 1 字の画。座標は格子の単位。
enum Stroke {
    Line((f32, f32), (f32, f32)),
    Curve((f32, f32), (f32, f32), (f32, f32)),
    Dot((f32, f32)),
}

fn glyph(byte: u8) -> Vec<Stroke> {
    let mut rng = ChaCha8Rng::from_seed(ALPHABET_SEED);
    rng.set_stream(byte as u64);
    let pick = |rng: &mut ChaCha8Rng| GRID[(rng.next_u32() % 6) as usize];
    let count = 2 + rng.next_u32() % 2;
    (0..count)
        .map(|_| match rng.next_u32() % 5 {
            0 => Stroke::Dot(pick(&mut rng)),
            1 | 2 => {
                let a = pick(&mut rng);
                let mut b = pick(&mut rng);
                if a == b {
                    b = (-a.0, -a.1);
                }
                Stroke::Curve(a, (0.0, 0.0), b)
            }
            _ => {
                let a = pick(&mut rng);
                let mut b = pick(&mut rng);
                if a == b {
                    b = (-a.0, -a.1);
                }
                Stroke::Line(a, b)
            }
        })
        .collect()
}

/// `text` を半径 `r` の円周に、字の上が外を向くように並べる。
/// 一周が埋まるまで菱形を挟んで繰り返し、先頭から `visible` の割合だけ描く。
pub fn ring(s: &mut String, text: &str, r: f32, half_height: f32, visible: f32) {
    let half_width = half_height * 0.6;
    let advance = half_width * 2.0 + half_height * 0.8;
    let slots = (TAU * r / advance).floor() as usize;
    let mut chars = text
        .bytes()
        .map(Some)
        // None は繰り返しの切れ目。菱形を置く
        .chain(std::iter::once(None))
        .cycle();
    let shown = (slots as f32 * visible).ceil() as usize;

    s.push_str("<g stroke-width=\"1.8\">\n");
    for i in 0..shown {
        let byte = chars.next().unwrap();
        let deg = 360.0 * i as f32 / slots as f32;
        let d = match byte {
            // 空白は字を置かずに間を空ける
            Some(b' ') => continue,
            Some(b) => path(&glyph(b), half_width, half_height),
            None => format!(
                "M0 {:.1}L{:.1} 0L0 {:.1}L{:.1} 0Z",
                -half_height * 0.6,
                half_width * 0.6,
                half_height * 0.6,
                -half_width * 0.6
            ),
        };
        // 回してから半径ぶん上へ出す。ルーン（translate→rotate）とは順番が逆。
        writeln!(
            s,
            r#"<path d="{d}" transform="rotate({deg:.2}) translate(0 {:.2})"/>"#,
            -r
        )
        .unwrap();
    }
    s.push_str("</g>\n");
}

/// 1 字ぶんの path の `d`。原点が字の中心で、上が -y。
pub fn glyph_d(byte: u8, half_width: f32, half_height: f32) -> String {
    path(&glyph(byte), half_width, half_height)
}

fn path(strokes: &[Stroke], hw: f32, hh: f32) -> String {
    let p = |(x, y): (f32, f32)| format!("{:.1} {:.1}", x * hw, y * hh);
    let mut d = String::new();
    for st in strokes {
        match *st {
            Stroke::Line(a, b) => write!(d, "M{}L{}", p(a), p(b)).unwrap(),
            Stroke::Curve(a, c, b) => write!(d, "M{}Q{} {}", p(a), p(c), p(b)).unwrap(),
            // 丸い線端の短い線で点にする
            Stroke::Dot(a) => write!(d, "M{}l0.1 0", p(a)).unwrap(),
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_letter_same_glyph() {
        let a = path(&glyph(b'g'), 6.0, 10.0);
        assert_eq!(a, path(&glyph(b'g'), 6.0, 10.0));
        assert_ne!(a, path(&glyph(b't'), 6.0, 10.0));
    }

    #[test]
    fn ring_repeats_text_around_the_circle() {
        let mut s = String::new();
        ring(&mut s, "ab", 200.0, 10.0, 1.0);
        let slots = (TAU * 200.0 / (12.0 + 8.0)).floor() as usize;
        assert_eq!(s.matches("<path ").count(), slots);

        let mut half = String::new();
        ring(&mut half, "ab", 200.0, 10.0, 0.5);
        assert_eq!(half.matches("<path ").count(), slots.div_ceil(2));
    }

    #[test]
    fn spaces_leave_gaps() {
        let mut s = String::new();
        ring(&mut s, "a b", 200.0, 10.0, 1.0);
        let mut t = String::new();
        ring(&mut t, "axb", 200.0, 10.0, 1.0);
        assert!(s.matches("<path ").count() < t.matches("<path ").count());
    }
}
