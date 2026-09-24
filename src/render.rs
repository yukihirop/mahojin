//! `MagicCircle` を SVG に描く。
//!
//! 形のパラメータは `MagicCircle` が決め、ルーンの字形と粒子の位置は
//! 同じハッシュから別ストリームの乱数で引く。パラメータ側の値の割り当てを
//! 変えずに描画の細部だけを足せるようにするため。

use std::f32::consts::TAU;
use std::fmt::Write;

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};

use crate::circle::{MagicCircle, Shape, unit};

const SIZE: f32 = 1000.0;
/// ルーン帯の外周と内周
const BAND_OUTER: f32 = 470.0;
const BAND_INNER: f32 = 405.0;
/// 内側の同心円を並べる範囲
const RINGS_OUTER: f32 = 370.0;
const RINGS_INNER: f32 = 220.0;
/// 対称装飾（星形多角形と小円）を載せる半径
const ORNAMENT: f32 = 300.0;
/// 中心図形の外接半径
const CORE: f32 = 190.0;

pub fn svg(c: &MagicCircle) -> String {
    let mut rng = ChaCha8Rng::from_seed(c.hash);
    rng.set_stream(1);

    let hue = c.hue;
    let main = format!("hsl({hue:.0},85%,65%)");
    let sub = format!("hsl({:.0},80%,55%)", (hue + 35.0) % 360.0);
    let spin = if c.clockwise { 360 } else { -360 };

    let mut s = String::new();
    let h = SIZE / 2.0;
    writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {SIZE} {SIZE}" width="{SIZE}" height="{SIZE}">"#,
        -h, -h
    )
    .unwrap();
    writeln!(
        s,
        r#"<defs><filter id="glow" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="5" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs>"#
    )
    .unwrap();
    writeln!(
        s,
        r##"<rect x="{}" y="{}" width="{SIZE}" height="{SIZE}" fill="#07070f"/>"##,
        -h, -h
    )
    .unwrap();

    particles(&mut s, &mut rng, c.particles, hue);

    writeln!(
        s,
        r#"<g fill="none" stroke="{main}" stroke-linecap="round" stroke-linejoin="round" filter="url(#glow)" transform="rotate({:.2})">"#,
        c.rotation
    )
    .unwrap();

    // 外周のルーン帯はゆっくり、内側の装飾は逆向きに速く回す。
    // SMIL なのでブラウザでは動き、静止画に落とすと初期角で止まる。
    open_spin(&mut s, spin, 90);
    circle(&mut s, BAND_OUTER, 4.0);
    circle(&mut s, BAND_INNER, 2.5);
    runes(&mut s, &mut rng, c.runes);
    s.push_str("</g>\n");

    inner_rings(&mut s, c.rings, &sub);

    open_spin(&mut s, -spin, 45);
    ornament(&mut s, c.symmetry, &sub);
    s.push_str("</g>\n");

    core(&mut s, c.shape, c.symmetry);

    s.push_str("</g>\n</svg>\n");
    s
}

fn open_spin(s: &mut String, degrees: i32, seconds: u32) {
    writeln!(
        s,
        r#"<g><animateTransform attributeName="transform" type="rotate" from="0" to="{degrees}" dur="{seconds}s" repeatCount="indefinite"/>"#
    )
    .unwrap();
}

fn circle(s: &mut String, r: f32, width: f32) {
    writeln!(s, r#"<circle r="{r:.2}" stroke-width="{width}"/>"#).unwrap();
}

fn polar(r: f32, theta: f32) -> (f32, f32) {
    // 0 rad を真上に取る
    (r * theta.sin(), -r * theta.cos())
}

/// ルーン帯に `count` 個の字形を等間隔で並べる。
/// 字形は 3×3 の格子上の縦棒 1 本と、乱数で選んだ 1〜3 本の画でできている。
fn runes(s: &mut String, rng: &mut ChaCha8Rng, count: u8) {
    const GRID: [(f32, f32); 9] = [
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
    // 字形の半幅・半高さ（帯の幅に収まるように）
    let (hw, hh) = (11.0, 20.0);
    let mid = (BAND_OUTER + BAND_INNER) / 2.0;

    s.push_str(r#"<g stroke-width="3">"#);
    for i in 0..count {
        let theta = TAU * i as f32 / count as f32;
        let (x, y) = polar(mid, theta);
        let mut d = format!("M0 {:.1}L0 {:.1}", -hh, hh);
        let strokes = 1 + rng.next_u32() % 3;
        for _ in 0..strokes {
            let a = GRID[(rng.next_u32() % 9) as usize];
            let mut b = GRID[(rng.next_u32() % 9) as usize];
            if a == b {
                b = GRID[(rng.next_u32() % 9) as usize];
            }
            write!(
                d,
                "M{:.1} {:.1}L{:.1} {:.1}",
                a.0 * hw,
                a.1 * hh,
                b.0 * hw,
                b.1 * hh
            )
            .unwrap();
        }
        writeln!(
            s,
            r#"<path d="{d}" transform="translate({x:.2} {y:.2}) rotate({:.2})"/>"#,
            theta.to_degrees()
        )
        .unwrap();
    }
    s.push_str("</g>\n");
}

/// ルーン帯の内側に残りの同心円を並べる。帯の 2 本も `rings` に数える。
fn inner_rings(s: &mut String, rings: u8, color: &str) {
    let n = rings.saturating_sub(2);
    if n == 0 {
        return;
    }
    writeln!(s, r#"<g stroke="{color}">"#).unwrap();
    for i in 0..n {
        let t = if n == 1 {
            0.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let r = RINGS_OUTER - t * (RINGS_OUTER - RINGS_INNER);
        circle(s, r, if i % 2 == 0 { 2.0 } else { 1.2 });
    }
    s.push_str("</g>\n");
}

/// `symmetry` 回対称の星形多角形と、その頂点に置く小円。
fn ornament(s: &mut String, n: u8, color: &str) {
    let n = n as usize;
    // 隣の隣を結ぶ。n の約数で閉じても、全頂点から引くので形は崩れない。
    let step = if n <= 4 {
        1
    } else {
        n / 2 - n.is_multiple_of(2) as usize
    };
    let pts: Vec<(f32, f32)> = (0..n)
        .map(|i| polar(ORNAMENT, TAU * i as f32 / n as f32))
        .collect();

    writeln!(s, r#"<g stroke="{color}" stroke-width="1.8">"#).unwrap();
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + step) % n]);
        writeln!(
            s,
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}"/>"#,
            a.0, a.1, b.0, b.1
        )
        .unwrap();
    }
    for &(x, y) in &pts {
        writeln!(
            s,
            r#"<circle cx="{x:.2}" cy="{y:.2}" r="18" stroke-width="2"/>"#
        )
        .unwrap();
    }
    s.push_str("</g>\n");
}

fn core(s: &mut String, shape: Shape, symmetry: u8) {
    match shape {
        Shape::Circle => {
            circle(s, CORE, 3.0);
            circle(s, CORE * 0.35, 2.0);
            // 中心から対称の数だけスポークを出す
            for i in 0..symmetry {
                let (x, y) = polar(CORE, TAU * i as f32 / symmetry as f32);
                let (ix, iy) = polar(CORE * 0.35, TAU * i as f32 / symmetry as f32);
                writeln!(
                    s,
                    r#"<line x1="{ix:.2}" y1="{iy:.2}" x2="{x:.2}" y2="{y:.2}" stroke-width="2"/>"#
                )
                .unwrap();
            }
        }
        Shape::Triangle => {
            polygon(s, 3, CORE, 0.0, 3.5);
            polygon(s, 3, CORE * 0.5, TAU / 2.0, 2.0);
            circle(s, CORE * 0.5 * 0.5, 2.0);
        }
        Shape::Hexagram => {
            polygon(s, 3, CORE, 0.0, 3.0);
            polygon(s, 3, CORE, TAU / 2.0, 3.0);
            circle(s, CORE * 0.5, 2.0);
        }
        Shape::MultiCircle => {
            // 生命の種: 中心の円と、その円周上に中心を持つ 6 つの円
            let r = CORE / 2.0;
            circle(s, r, 2.5);
            for i in 0..6 {
                let (x, y) = polar(r, TAU * i as f32 / 6.0);
                writeln!(
                    s,
                    r#"<circle cx="{x:.2}" cy="{y:.2}" r="{r:.2}" stroke-width="2.5"/>"#
                )
                .unwrap();
            }
            circle(s, CORE, 3.0);
        }
    }
}

fn polygon(s: &mut String, n: u8, r: f32, offset: f32, width: f32) {
    let pts: Vec<String> = (0..n)
        .map(|i| {
            let (x, y) = polar(r, offset + TAU * i as f32 / n as f32);
            format!("{x:.2},{y:.2}")
        })
        .collect();
    writeln!(
        s,
        r#"<polygon points="{}" stroke-width="{width}"/>"#,
        pts.join(" ")
    )
    .unwrap();
}

/// 魔法陣の周りに漂う光の粒。
fn particles(s: &mut String, rng: &mut ChaCha8Rng, count: u16, hue: f32) {
    writeln!(s, r#"<g fill="hsl({:.0},90%,80%)">"#, (hue + 180.0) % 360.0).unwrap();
    for _ in 0..count {
        // 面積あたり一様になるよう半径は平方根で取る
        let r = unit(rng).sqrt() * SIZE / 2.0;
        let (x, y) = polar(r, unit(rng) * TAU);
        let size = 0.6 + unit(rng) * 2.2;
        let opacity = 0.15 + unit(rng) * 0.6;
        writeln!(
            s,
            r#"<circle cx="{x:.1}" cy="{y:.1}" r="{size:.2}" opacity="{opacity:.2}"/>"#
        )
        .unwrap();
    }
    s.push_str("</g>\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_command_same_svg() {
        let a = svg(&MagicCircle::from_command("cargo build"));
        let b = svg(&MagicCircle::from_command("cargo build"));
        assert_eq!(a, b);
    }

    #[test]
    fn different_command_different_svg() {
        assert_ne!(
            svg(&MagicCircle::from_command("git push")),
            svg(&MagicCircle::from_command("git pull"))
        );
    }

    #[test]
    fn draws_every_rune_and_particle() {
        let c = MagicCircle::from_command("git status");
        let out = svg(&c);
        assert!(out.starts_with("<svg"));
        assert!(out.trim_end().ends_with("</svg>"));
        assert_eq!(out.matches("<path ").count(), c.runes as usize);
        assert_eq!(out.matches(r#"opacity=""#).count(), c.particles as usize);
    }
}
