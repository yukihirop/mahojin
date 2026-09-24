//! コマンド文字列を架空の魔法文字で円周に書く。
//!
//! 字形はコマンドのハッシュではなく固定の種から引く。同じ書体の同じ文字は、
//! どの魔法陣でも同じ形になり、本物の文字体系のように見える。UTF-8 のバイトごとに 1 字。
//! 書体・字の大きさ・字間・区切りは魔法陣ごとに変わる（[`Hand`]）。

use std::f32::consts::TAU;
use std::fmt::Write;

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};

use crate::locale::Locale;

/// 字母の種。変えると全部の魔法陣の文字が変わる。
const ALPHABET_SEED: [u8; 32] = *b"mahojin: the runes of all spells";

/// 書体。書体ごとに別の字母を持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Style {
    /// 線・曲線・点を混ぜた、最初からある書体
    Runic,
    /// 直線だけで刻んだ角ばった書体
    Angular,
    /// 曲線と小さな輪でつないだ書体
    Flowing,
    /// 1 本の線に点を散らした書体
    Dotted,
}

impl Style {
    pub const ALL: [Style; 4] = [Style::Runic, Style::Angular, Style::Flowing, Style::Dotted];

    pub fn name(self, l: Locale) -> &'static str {
        match self {
            Style::Runic => l.pick("刻文字", "runic"),
            Style::Angular => l.pick("角文字", "angular"),
            Style::Flowing => l.pick("流文字", "flowing"),
            Style::Dotted => l.pick("点文字", "dotted"),
        }
    }
}

/// 呪文を繰り返すときの切れ目に置く印。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separator {
    Diamond,
    Dot,
    Star,
    Bar,
    None,
}

impl Separator {
    pub const ALL: [Separator; 5] = [
        Separator::Diamond,
        Separator::Dot,
        Separator::Star,
        Separator::Bar,
        Separator::None,
    ];

    pub fn name(self, l: Locale) -> &'static str {
        match self {
            Separator::Diamond => l.pick("菱形", "diamond"),
            Separator::Dot => l.pick("点", "dot"),
            Separator::Star => l.pick("星", "star"),
            Separator::Bar => l.pick("棒", "bar"),
            Separator::None => l.pick("なし", "none"),
        }
    }
}

/// 筆致。どの書体で、どのくらいの大きさと間隔で書くか。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hand {
    pub style: Style,
    /// 字の大きさの倍率 [0.7, 1.4)。小さいほど一周に多くの字が入る
    pub size: f32,
    /// 字間の倍率 [0.3, 1.6)。字の幅に対する隙間の広さ
    pub spacing: f32,
    pub separator: Separator,
}

impl Hand {
    /// 最初の版の書き方。書体・大きさ・間隔・区切りがすべて固定だった。
    #[cfg(test)]
    pub const PLAIN: Hand = Hand {
        style: Style::Runic,
        size: 1.0,
        spacing: 1.0,
        separator: Separator::Diamond,
    };
}

/// 字形を置く 2×3 の格子（横 ±1、縦 ±1）
const GRID: [(f32, f32); 6] = [
    (-1.0, -1.0),
    (1.0, -1.0),
    (-1.0, 0.0),
    (1.0, 0.0),
    (-1.0, 1.0),
    (1.0, 1.0),
];

/// 角文字と点文字に使う 3×3 の格子
const GRID9: [(f32, f32); 9] = [
    (-1.0, -1.0),
    (0.0, -1.0),
    (1.0, -1.0),
    (-1.0, 0.0),
    (0.0, 0.0),
    (1.0, 0.0),
    (-1.0, 1.0),
    (0.0, 1.0),
    (1.0, 1.0),
];

/// 1 字の画。座標は格子の単位。
enum Stroke {
    Line((f32, f32), (f32, f32)),
    Curve((f32, f32), (f32, f32), (f32, f32)),
    Dot((f32, f32)),
    /// 中心と半径（半径は字の半幅に対する割合）の小さな輪
    Loop((f32, f32), f32),
}

/// 同じ点を選んだら反対側へ向ける。長さ 0 の線を作らない。
fn distinct(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    if a == b { (-a.0, -a.1) } else { b }
}

fn glyph(style: Style, byte: u8) -> Vec<Stroke> {
    let mut rng = ChaCha8Rng::from_seed(ALPHABET_SEED);
    // 刻文字は最初の版と同じ字形のまま。ほかの書体は別の流れから引く。
    let index = Style::ALL.iter().position(|s| *s == style).unwrap() as u64;
    rng.set_stream(index << 8 | byte as u64);
    let mut below = |n: u32| rng.next_u32() % n;
    match style {
        Style::Runic => {
            let count = 2 + below(2);
            (0..count)
                .map(|_| match below(5) {
                    0 => Stroke::Dot(GRID[below(6) as usize]),
                    1 | 2 => {
                        let a = GRID[below(6) as usize];
                        let b = distinct(a, GRID[below(6) as usize]);
                        Stroke::Curve(a, (0.0, 0.0), b)
                    }
                    _ => {
                        let a = GRID[below(6) as usize];
                        let b = distinct(a, GRID[below(6) as usize]);
                        Stroke::Line(a, b)
                    }
                })
                .collect()
        }
        Style::Angular => {
            // 縦か横の背骨を 1 本通し、そこから枝を出す
            let mut strokes = vec![if below(2) == 0 {
                Stroke::Line((0.0, -1.0), (0.0, 1.0))
            } else {
                Stroke::Line((-1.0, 0.0), (1.0, 0.0))
            }];
            for _ in 0..1 + below(3) {
                let a = GRID9[below(9) as usize];
                let b = distinct(a, GRID9[below(9) as usize]);
                strokes.push(Stroke::Line(a, b));
            }
            strokes
        }
        Style::Flowing => {
            let mut strokes = Vec::new();
            for _ in 0..1 + below(2) {
                let a = GRID[below(6) as usize];
                let b = distinct(a, GRID[below(6) as usize]);
                // 中心ではなく格子の端へ引っぱって、大きくうねらせる
                let c = GRID9[below(9) as usize];
                strokes.push(Stroke::Curve(a, (c.0 * 1.6, c.1 * 1.6), b));
            }
            strokes.push(Stroke::Loop(
                GRID9[below(9) as usize],
                0.35 + below(3) as f32 * 0.15,
            ));
            strokes
        }
        Style::Dotted => {
            let a = GRID9[below(9) as usize];
            let b = distinct(a, GRID9[below(9) as usize]);
            let mut strokes = vec![Stroke::Line(a, b)];
            for _ in 0..2 + below(3) {
                strokes.push(Stroke::Dot(GRID9[below(9) as usize]));
            }
            strokes
        }
    }
}

/// `text` を半径 `r` の円周に、字の上が外を向くように並べる。
/// 一周が埋まるまで区切りを挟んで繰り返し、先頭から `visible` の割合だけ描く。
/// `half_height` は倍率 1 のときの字の半分の高さ。
pub fn ring(s: &mut String, text: &str, hand: &Hand, r: f32, half_height: f32, visible: f32) {
    let half_height = half_height * hand.size;
    let half_width = half_height * 0.6;
    let advance = half_width * 2.0 + half_height * 0.8 * hand.spacing;
    let slots = (TAU * r / advance).floor() as usize;
    let mut chars = text
        .bytes()
        .map(Some)
        // None は繰り返しの切れ目。区切りの印を置く
        .chain(std::iter::once(None))
        .cycle();
    let shown = (slots as f32 * visible).ceil() as usize;

    // 点は線の太さで大きさが決まるので、字が大きいほど線も太くする
    writeln!(s, r#"<g stroke-width="{:.2}">"#, 1.2 + 0.6 * hand.size).unwrap();
    for i in 0..shown {
        let byte = chars.next().unwrap();
        let deg = 360.0 * i as f32 / slots as f32;
        let d = match byte {
            // 空白は字を置かずに間を空ける
            Some(b' ') => continue,
            Some(b) => path(&glyph(hand.style, b), half_width, half_height),
            None => match separator(hand.separator, half_width, half_height) {
                Some(d) => d,
                None => continue,
            },
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

fn separator(kind: Separator, hw: f32, hh: f32) -> Option<String> {
    let (w, h) = (hw * 0.6, hh * 0.6);
    Some(match kind {
        Separator::Diamond => format!("M0 {:.1}L{w:.1} 0L0 {h:.1}L{:.1} 0Z", -h, -w),
        // 小さな輪
        Separator::Dot => {
            let r = w * 0.5;
            format!(
                "M{:.1} 0a{r:.1} {r:.1} 0 1 0 {:.1} 0a{r:.1} {r:.1} 0 1 0 {:.1} 0",
                -r,
                2.0 * r,
                -2.0 * r
            )
        }
        // 4 つの尖りを持つ星。辺を中心へ引き込む
        Separator::Star => format!(
            "M0 {:.1}Q0 0 {w:.1} 0Q0 0 0 {h:.1}Q0 0 {:.1} 0Q0 0 0 {:.1}Z",
            -h, -w, -h
        ),
        Separator::Bar => format!("M0 {:.1}L0 {h:.1}", -h),
        Separator::None => return None,
    })
}

/// 1 字ぶんの path の `d`。原点が字の中心で、上が -y。
pub fn glyph_d(style: Style, byte: u8, half_width: f32, half_height: f32) -> String {
    path(&glyph(style, byte), half_width, half_height)
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
            Stroke::Loop((x, y), k) => {
                let r = k * hw;
                write!(
                    d,
                    "M{:.1} {:.1}a{r:.1} {r:.1} 0 1 0 {:.1} 0a{r:.1} {r:.1} 0 1 0 {:.1} 0",
                    x * hw - r,
                    y * hh,
                    2.0 * r,
                    -2.0 * r
                )
                .unwrap()
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_letter_same_glyph() {
        for style in Style::ALL {
            let a = glyph_d(style, b'g', 6.0, 10.0);
            assert_eq!(a, glyph_d(style, b'g', 6.0, 10.0));
            assert_ne!(a, glyph_d(style, b't', 6.0, 10.0), "{style:?}");
        }
    }

    #[test]
    fn styles_have_different_alphabets() {
        let g: Vec<_> = Style::ALL
            .iter()
            .map(|s| glyph_d(*s, b'g', 6.0, 10.0))
            .collect();
        for i in 0..g.len() {
            for j in i + 1..g.len() {
                assert_ne!(g[i], g[j]);
            }
        }
    }

    /// 刻文字の字形を固定する。変われば全部の魔法陣の文字が変わる
    #[test]
    fn runic_keeps_its_glyphs() {
        let pinned = [
            (b'g', "M6.0 0.0L-6.0 10.0M6.0 -10.0L-6.0 0.0"),
            (b's', "M6.0 -10.0Q0.0 0.0 6.0 0.0M-6.0 10.0l0.1 0"),
            (b't', "M6.0 0.0Q0.0 0.0 -6.0 0.0M-6.0 10.0Q0.0 0.0 -6.0 0.0"),
        ];
        for (b, d) in pinned {
            assert_eq!(glyph_d(Style::Runic, b, 6.0, 10.0), d);
        }
    }

    #[test]
    fn ring_repeats_text_around_the_circle() {
        let mut s = String::new();
        ring(&mut s, "ab", &Hand::PLAIN, 200.0, 10.0, 1.0);
        let slots = (TAU * 200.0 / (12.0 + 8.0)).floor() as usize;
        assert_eq!(s.matches("<path ").count(), slots);

        let mut half = String::new();
        ring(&mut half, "ab", &Hand::PLAIN, 200.0, 10.0, 0.5);
        assert_eq!(half.matches("<path ").count(), slots.div_ceil(2));
    }

    #[test]
    fn smaller_tighter_hand_writes_more() {
        let count = |hand: &Hand| {
            let mut s = String::new();
            ring(&mut s, "abc", hand, 200.0, 10.0, 1.0);
            s.matches("<path ").count()
        };
        let dense = Hand {
            size: 0.7,
            spacing: 0.3,
            ..Hand::PLAIN
        };
        let sparse = Hand {
            size: 1.4,
            spacing: 1.6,
            ..Hand::PLAIN
        };
        assert!(count(&dense) > count(&Hand::PLAIN) * 3 / 2);
        assert!(count(&sparse) < count(&Hand::PLAIN));
    }

    #[test]
    fn spaces_leave_gaps() {
        let mut s = String::new();
        ring(&mut s, "a b", &Hand::PLAIN, 200.0, 10.0, 1.0);
        let mut t = String::new();
        ring(&mut t, "axb", &Hand::PLAIN, 200.0, 10.0, 1.0);
        assert!(s.matches("<path ").count() < t.matches("<path ").count());
    }
}
