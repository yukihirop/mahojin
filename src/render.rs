//! `MagicCircle` を SVG に描く。
//!
//! 形のパラメータは `MagicCircle` が決め、ルーンの字形と粒子の位置は
//! 同じハッシュから別ストリームの乱数で引く。パラメータ側の値の割り当てを
//! 変えずに描画の細部だけを足せるようにするため。

use std::f32::consts::TAU;
use std::fmt::Write;

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};

use crate::circle::{Layout, MagicCircle, Ornament, Shape, unit};
use crate::script;

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
/// コマンドを書く文字の帯の半径。外側はルーン帯のすぐ内、内側は中心図形のすぐ外
const TEXT_OUTER: f32 = 387.0;
const TEXT_INNER: f32 = 205.0;
/// 大きな星の頂点に置く円の半径。中心はルーン帯の上に来る
const GRAND_VERTEX: f32 = 46.0;
const BACKGROUND: &str = "#07070f";

/// 完成した魔法陣。ブラウザで開くと SMIL で回り続ける。
pub fn svg(c: &MagicCircle, spell: &str) -> String {
    draw(c, spell, None)
}

/// 展開アニメーションの 1 コマ。`t` は 0.0（何も無い）〜 1.0（完成）。
/// 最後のコマは `svg` の初期状態と同じ絵になる。
pub fn frame(c: &MagicCircle, spell: &str, t: f32) -> String {
    draw(c, spell, Some(t.clamp(0.0, 1.0)))
}

/// 展開の段取り。各要素が現れる区間を `t` の上で重ねてずらす。
struct Stages {
    band: f32,
    runes: f32,
    rings: f32,
    ornament: f32,
    core: f32,
    /// 完成の直前に一瞬だけ強く光る
    flash: f32,
}

impl Stages {
    fn at(t: Option<f32>) -> Self {
        let Some(t) = t else {
            return Stages {
                band: 1.0,
                runes: 1.0,
                rings: 1.0,
                ornament: 1.0,
                core: 1.0,
                flash: 0.0,
            };
        };
        let span = |a: f32, b: f32| ease(((t - a) / (b - a)).clamp(0.0, 1.0));
        Stages {
            band: span(0.0, 0.35),
            runes: span(0.15, 0.55),
            rings: span(0.3, 0.65),
            ornament: span(0.4, 0.75),
            core: span(0.55, 0.85),
            // 0.8 から立ち上がり 0.9 で頂点、1.0 で消える三角波
            flash: (1.0 - ((t - 0.9) / 0.1).abs()).max(0.0),
        }
    }
}

/// ease-out cubic。勢いよく出て、すっと収まる。
fn ease(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(3)
}

fn draw(c: &MagicCircle, spell: &str, t: Option<f32>) -> String {
    let mut rng = ChaCha8Rng::from_seed(c.hash);
    rng.set_stream(1);
    let st = Stages::at(t);

    let hue = c.hue;
    let main = format!("hsl({hue:.0},85%,65%)");
    let sub = format!("hsl({:.0},80%,55%)", (hue + 35.0) % 360.0);
    let dir = if c.clockwise { 1.0 } else { -1.0 };

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
        r#"<defs><filter id="glow" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="{:.1}" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>{DOUBLE}</defs>"#,
        5.0 + 14.0 * st.flash
    )
    .unwrap();
    writeln!(
        s,
        r#"<rect x="{}" y="{}" width="{SIZE}" height="{SIZE}" fill="{BACKGROUND}"/>"#,
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
    // 完成図では SMIL で回し続け（静止画に落とすと初期角で止まる）、
    // アニメーションのコマでは回りながら定位置へ収まっていく。
    open_layer(
        &mut s,
        t,
        st.band,
        dir * 150.0 * (1.0 - st.band),
        dir * 360.0,
        90,
    );
    open_double(&mut s);
    drawn_circle(&mut s, BAND_OUTER, 4.0, st.band);
    drawn_circle(&mut s, BAND_INNER, 2.5, st.band);
    s.push_str("</g>\n");
    let visible = (c.runes as f32 * st.runes).ceil() as u8;
    runes(&mut s, &mut rng, c.runes, visible);
    s.push_str("</g>\n");

    if st.rings > 0.0 {
        open_fade(&mut s, st.rings);
        open_double(&mut s);
        inner_rings(&mut s, c.rings, &sub);
        s.push_str("</g>\n");
        script::ring(&mut s, spell, TEXT_OUTER, 10.0, st.rings);
        s.push_str("</g>\n");
    }

    if st.ornament > 0.0 {
        open_layer(
            &mut s,
            t,
            st.ornament,
            -dir * 240.0 * (1.0 - st.ornament),
            -dir * 360.0,
            45,
        );
        match c.layout {
            Layout::Classic => {
                open_double(&mut s);
                ornament(&mut s, c.ornament, c.symmetry, &sub);
                s.push_str("</g>\n");
            }
            Layout::Grand => grand_star(&mut s, c.symmetry, spell, &main),
        }
        s.push_str("</g>\n");
    }

    if st.core > 0.0 {
        open_fade(&mut s, st.core);
        open_double(&mut s);
        core(&mut s, c.shape, c.symmetry);
        s.push_str("</g>\n");
        script::ring(&mut s, spell, TEXT_INNER, 7.0, st.core);
        s.push_str("</g>\n");
    }

    s.push_str("</g>\n");
    if st.flash > 0.0 {
        writeln!(
            s,
            r#"<circle r="{BAND_OUTER}" fill="{main}" opacity="{:.3}"/>"#,
            0.18 * st.flash
        )
        .unwrap();
    }
    s.push_str("</svg>\n");
    s
}

/// 線を平行な 2 本に割るフィルタ。線を少しと大きめに膨らませ、大きい方から小さい方を
/// くり抜いて両縁だけ残す。半径は viewBox の単位で、2 本の間隔は「元の線幅 + 3」、
/// 1 本の太さは 2。間隔を線より広く取らないと、光のにじみで 1 本に潰れて見える。
const DOUBLE: &str = r#"<filter id="double" x="-10%" y="-10%" width="120%" height="120%"><feMorphology in="SourceGraphic" operator="dilate" radius="1.5" result="core"/><feMorphology in="SourceGraphic" operator="dilate" radius="3.5" result="wide"/><feComposite in="wide" in2="core" operator="out"/></filter>"#;

fn open_double(s: &mut String) {
    s.push_str("<g filter=\"url(#double)\">\n");
}

/// 魔法陣いっぱいに広がる星。頂点はルーン帯に届き、そこに背景色で塗った円を置いて
/// 帯の上に乗っているように見せる。星の内側にできる多角形には内接円を引く。
fn grand_star(s: &mut String, n: u8, spell: &str, color: &str) {
    let r = BAND_INNER;
    let pts: Vec<(f32, f32)> = (0..n)
        .map(|i| polar(r, TAU * i as f32 / n as f32))
        .collect();
    open_double(s);
    writeln!(s, r#"<g stroke="{color}">"#).unwrap();
    // 星の内側の多角形の頂点までの距離
    let inner = if n <= 4 {
        // 3 と 4 は同じ多角形を半歩ずらして重ね、六芒星・八芒星にする
        polygon(s, n, r, 0.0, 3.5);
        polygon(s, n, r, TAU / (2 * n) as f32, 3.5);
        r * (TAU / (2 * n) as f32).cos() / (TAU / (4 * n) as f32).cos()
    } else {
        let n = n as usize;
        let step = n / 2 - n.is_multiple_of(2) as usize;
        for i in 0..n {
            line(s, pts[i], pts[(i + step) % n], 3.5);
        }
        let k = step as f32;
        r * (TAU * k / (2 * n) as f32).cos() / (TAU * (k - 1.0) / (2 * n) as f32).cos()
    };
    // 内側の多角形の辺に接する円
    circle(s, inner * (TAU / (4 * n) as f32).cos(), 2.0);
    s.push_str("</g>\n</g>\n");

    // 頂点の円は二重線にせず、中を塗ってルーンを隠す。
    // 中にはコマンドの文字を頭から 1 字ずつ書く（空白は飛ばす）。
    let mut letters = spell.bytes().filter(|b| *b != b' ').cycle();
    writeln!(s, r#"<g stroke="{color}" fill="{BACKGROUND}">"#).unwrap();
    for (i, &(x, y)) in pts.iter().enumerate() {
        writeln!(
            s,
            r#"<circle cx="{x:.2}" cy="{y:.2}" r="{GRAND_VERTEX}" stroke-width="3"/>"#
        )
        .unwrap();
        writeln!(
            s,
            r#"<circle cx="{x:.2}" cy="{y:.2}" r="{:.1}" stroke-width="1.5"/>"#,
            GRAND_VERTEX * 0.72
        )
        .unwrap();
        if let Some(b) = letters.next() {
            // 字の上が外を向くよう、頂点の向きに回す
            writeln!(
                s,
                r#"<path d="{}" fill="none" stroke-width="2.5" transform="translate({x:.2} {y:.2}) rotate({:.2})"/>"#,
                script::glyph_d(b, 11.0, 18.0),
                360.0 * i as f32 / n as f32
            )
            .unwrap();
        }
    }
    s.push_str("</g>\n");
}

/// 回転する層を開く。完成図なら SMIL で回し続け、コマなら `angle` だけ回して
/// 中心から広がりながら現れる。
fn open_layer(s: &mut String, t: Option<f32>, p: f32, angle: f32, spin: f32, seconds: u32) {
    match t {
        None => writeln!(
            s,
            r#"<g><animateTransform attributeName="transform" type="rotate" from="0" to="{spin}" dur="{seconds}s" repeatCount="indefinite"/>"#
        )
        .unwrap(),
        Some(_) => writeln!(
            s,
            r#"<g opacity="{p:.3}" transform="rotate({angle:.2}) scale({:.3})">"#,
            0.6 + 0.4 * p
        )
        .unwrap(),
    }
}

/// 中心から膨らみながらフェードインする層を開く。
fn open_fade(s: &mut String, p: f32) {
    if p >= 1.0 {
        return s.push_str("<g>\n");
    }
    writeln!(
        s,
        r#"<g opacity="{p:.3}" transform="scale({:.3})">"#,
        0.8 + 0.2 * p
    )
    .unwrap();
}

/// 円周を `p` の割合だけ描く。1.0 なら普通の円。
fn drawn_circle(s: &mut String, r: f32, width: f32, p: f32) {
    if p >= 1.0 {
        return circle(s, r, width);
    }
    let len = TAU * r;
    // 円周は 3 時の位置から描かれるので、-90° 回して真上から描き始める
    writeln!(
        s,
        r#"<circle r="{r:.2}" stroke-width="{width}" stroke-dasharray="{len:.1} {len:.1}" stroke-dashoffset="{:.1}" transform="rotate(-90)"/>"#,
        len * (1.0 - p)
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

/// ルーン帯に `count` 個の字形を等間隔で並べ、先頭から `visible` 個だけ描く。
/// 字形は 3×3 の格子上の縦棒 1 本と、乱数で選んだ 1〜3 本の画でできている。
/// 描かない字形も乱数は引く。コマによって字形が変わらないようにするため。
fn runes(s: &mut String, rng: &mut ChaCha8Rng, count: u8, visible: u8) {
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
        if i >= visible {
            continue;
        }
        writeln!(
            s,
            r#"<path class="rune" d="{d}" transform="translate({x:.2} {y:.2}) rotate({:.2})"/>"#,
            theta.to_degrees()
        )
        .unwrap();
        // 字と字の間に小さな点を打ち、字が少ないときでも帯が途切れて見えないようにする
        let (dx, dy) = polar(mid, theta + TAU / (2 * count as u32) as f32);
        writeln!(s, r#"<circle cx="{dx:.2}" cy="{dy:.2}" r="3.5"/>"#).unwrap();
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

/// `symmetry` 回対称の装飾。
fn ornament(s: &mut String, kind: Ornament, n: u8, color: &str) {
    writeln!(s, r#"<g stroke="{color}" stroke-width="1.8">"#).unwrap();
    let pts: Vec<(f32, f32)> = (0..n)
        .map(|i| polar(ORNAMENT, TAU * i as f32 / n as f32))
        .collect();
    match kind {
        Ornament::Star => {
            if n <= 4 {
                // 3 と 4 は飛ばして結べる頂点が無く、星にならない。
                // 同じ多角形を半歩ずらして重ね、六芒星・八芒星にする。
                polygon(s, n, ORNAMENT, 0.0, 1.8);
                polygon(s, n, ORNAMENT, TAU / (2 * n) as f32, 1.8);
            } else {
                // 隣の隣より遠くを結ぶ。n の約数で閉じても、全頂点から引くので形は崩れない。
                let n = n as usize;
                let step = n / 2 - n.is_multiple_of(2) as usize;
                for i in 0..n {
                    line(s, pts[i], pts[(i + step) % n], 1.8);
                }
            }
            for &(x, y) in &pts {
                writeln!(
                    s,
                    r#"<circle cx="{x:.2}" cy="{y:.2}" r="18" stroke-width="2"/>"#
                )
                .unwrap();
            }
        }
        Ornament::Chain => {
            // 隣同士がちょうど接する半径。3 回対称だと中心まで届くので上限を付ける。
            let r = (ORNAMENT * (TAU / (2 * n) as f32).sin()).min(70.0);
            circle(s, ORNAMENT, 1.2);
            for &(x, y) in &pts {
                writeln!(
                    s,
                    r#"<circle cx="{x:.2}" cy="{y:.2}" r="{r:.2}" stroke-width="2"/>"#
                )
                .unwrap();
                writeln!(
                    s,
                    r#"<circle cx="{x:.2}" cy="{y:.2}" r="{:.2}" stroke-width="1.2"/>"#,
                    r * 0.45
                )
                .unwrap();
            }
        }
        Ornament::Rays => {
            for i in 0..n {
                let theta = TAU * i as f32 / n as f32;
                line(
                    s,
                    polar(CORE + 15.0, theta),
                    polar(ORNAMENT - 26.0, theta),
                    1.8,
                );
                line(
                    s,
                    polar(ORNAMENT + 26.0, theta),
                    polar(RINGS_OUTER, theta),
                    1.2,
                );
                // 先端の菱形
                let d = [
                    polar(ORNAMENT - 26.0, theta),
                    polar(ORNAMENT, theta + 0.07),
                    polar(ORNAMENT + 26.0, theta),
                    polar(ORNAMENT, theta - 0.07),
                ];
                let pts: Vec<String> = d.iter().map(|(x, y)| format!("{x:.2},{y:.2}")).collect();
                writeln!(
                    s,
                    r#"<polygon points="{}" stroke-width="2"/>"#,
                    pts.join(" ")
                )
                .unwrap();
            }
        }
    }
    s.push_str("</g>\n");
}

fn core(s: &mut String, shape: Shape, symmetry: u8) {
    let n = symmetry;
    let at = |r: f32, i: u32, of: u32| polar(r, TAU * i as f32 / of as f32);
    match shape {
        Shape::Circle => {
            circle(s, CORE, 3.0);
            circle(s, CORE * 0.35, 2.0);
            // 中心から対称の数だけスポークを出す
            for i in 0..n as u32 {
                line(s, at(CORE * 0.35, i, n as u32), at(CORE, i, n as u32), 2.0);
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
                let (x, y) = at(r, i, 6);
                writeln!(
                    s,
                    r#"<circle cx="{x:.2}" cy="{y:.2}" r="{r:.2}" stroke-width="2.5"/>"#
                )
                .unwrap();
            }
            circle(s, CORE, 3.0);
        }
        Shape::Pentagram => {
            circle(s, CORE, 3.0);
            for i in 0..5 {
                line(s, at(CORE, i, 5), at(CORE, i + 2, 5), 3.0);
            }
            // 星の内側にできる五角形に内接する円
            circle(s, CORE * 0.38 * 0.81, 2.0);
        }
        Shape::Octagram => {
            polygon(s, 4, CORE, 0.0, 3.0);
            polygon(s, 4, CORE, TAU / 8.0, 3.0);
            circle(s, CORE * 0.55, 2.0);
            circle(s, CORE * 0.2, 2.0);
        }
        Shape::NestedPolygons => {
            // 対称の数の多角形を、半歩ずつずらしながら小さく重ねる
            let k = n.clamp(3, 8);
            for (i, scale) in [1.0, 0.78, 0.56, 0.34].into_iter().enumerate() {
                let offset = if i % 2 == 0 {
                    0.0
                } else {
                    TAU / (2 * k) as f32
                };
                polygon(s, k, CORE * scale, offset, 3.0 - i as f32 * 0.5);
            }
        }
        Shape::Spiral => {
            circle(s, CORE, 3.0);
            // 対称の数だけ腕を出し、外へ向かうほど回り込ませる
            for arm in 0..n as u32 {
                let start = TAU * arm as f32 / n as f32;
                let d: String = (0..=24)
                    .map(|j| {
                        let f = j as f32 / 24.0;
                        let (x, y) = polar(CORE * (0.08 + 0.92 * f), start + f * TAU * 0.45);
                        format!("{}{x:.2} {y:.2}", if j == 0 { "M" } else { "L" })
                    })
                    .collect();
                writeln!(s, r#"<path d="{d}" stroke-width="2.2"/>"#).unwrap();
            }
            circle(s, CORE * 0.08, 2.0);
        }
        Shape::Petals => {
            circle(s, CORE, 3.0);
            // 中心から外周へ伸びる花弁。両脇の制御点で膨らませる
            for i in 0..n as u32 {
                let theta = TAU * i as f32 / n as f32;
                let spread = (TAU / n as f32).min(1.2) * 0.5;
                let tip = polar(CORE * 0.95, theta);
                let l = polar(CORE * 0.6, theta - spread);
                let r = polar(CORE * 0.6, theta + spread);
                writeln!(
                    s,
                    r#"<path d="M0 0Q{:.2} {:.2} {:.2} {:.2}Q{:.2} {:.2} 0 0" stroke-width="2.2"/>"#,
                    l.0, l.1, tip.0, tip.1, r.0, r.1
                )
                .unwrap();
            }
            circle(s, CORE * 0.18, 2.0);
        }
        Shape::Metatron => {
            // メタトロンの立方体: 中心・内側 6・外側 6 の 13 個の円と、その中心同士を結ぶ線
            let d = CORE / 2.5;
            let mut pts = vec![(0.0, 0.0)];
            pts.extend((0..6).map(|i| at(d, i, 6)));
            pts.extend((0..6).map(|i| at(d * 2.0, i, 6)));
            for i in 0..pts.len() {
                for j in i + 1..pts.len() {
                    line(s, pts[i], pts[j], 0.9);
                }
            }
            for &(x, y) in &pts {
                writeln!(
                    s,
                    r#"<circle cx="{x:.2}" cy="{y:.2}" r="{:.2}" stroke-width="2"/>"#,
                    d / 2.0
                )
                .unwrap();
            }
        }
    }
}

fn line(s: &mut String, a: (f32, f32), b: (f32, f32), width: f32) {
    writeln!(
        s,
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke-width="{width}"/>"#,
        a.0, a.1, b.0, b.1
    )
    .unwrap();
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

    const RUNE: &str = r#"<path class="rune""#;

    #[test]
    fn same_command_same_svg() {
        let a = svg(&MagicCircle::from_command("cargo build"), "cargo build");
        let b = svg(&MagicCircle::from_command("cargo build"), "cargo build");
        assert_eq!(a, b);
    }

    #[test]
    fn different_command_different_svg() {
        assert_ne!(
            svg(&MagicCircle::from_command("git push"), "git push"),
            svg(&MagicCircle::from_command("git pull"), "git pull")
        );
    }

    #[test]
    fn frames_build_up_to_the_full_circle() {
        let c = MagicCircle::from_command("git status");
        assert_eq!(frame(&c, "git status", 0.0).matches(RUNE).count(), 0);
        assert_eq!(
            frame(&c, "git status", 1.0).matches(RUNE).count(),
            c.runes as usize
        );
        // 途中のコマでも字形は完成図と同じものが先頭から並ぶ
        let half = frame(&c, "git status", 0.4);
        let full = frame(&c, "git status", 1.0);
        let first_rune = |s: &str| s.split(RUNE).nth(1).map(|p| p[..60].to_string());
        assert_eq!(first_rune(&half), first_rune(&full));
        // 光りは完成の直前にだけ出て、完成のコマでは収まっている
        let flash = |s: &str| s.contains(&format!(r#"<circle r="{BAND_OUTER}" fill="#));
        assert!(flash(&frame(&c, "git status", 0.9)));
        assert!(!flash(&full));
    }

    #[test]
    fn draws_every_rune_and_particle() {
        let c = MagicCircle::from_command("git status");
        let out = svg(&c, "git status");
        assert!(out.starts_with("<svg"));
        assert!(out.trim_end().ends_with("</svg>"));
        assert_eq!(out.matches(RUNE).count(), c.runes as usize);
        assert_eq!(out.matches(r#"opacity=""#).count(), c.particles as usize);
    }
}
