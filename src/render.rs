//! `MagicCircle` を SVG に描く。
//!
//! 形のパラメータは `MagicCircle` が決め、ルーンの字形と粒子の位置は
//! 同じハッシュから別ストリームの乱数で引く。パラメータ側の値の割り当てを
//! 変えずに描画の細部だけを足せるようにするため。

use std::f32::consts::TAU;
use std::fmt::Write;

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};

use crate::circle::{Band, FORBIDDEN_HUE, Layout, MagicCircle, Ornament, Shape, Tier, unit};
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
/// 突破の配置で、外周を破る多角形の外接半径。はみ出すぶん全体を縮めて描く
const BREACH: f32 = 560.0;
const BREACH_SCALE: f32 = 0.86;

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

    // 小魔法は粒子も控えめにする
    let particles_count = if c.tier == Tier::Small {
        c.particles / 3
    } else {
        c.particles
    };
    // 粒子はふつう補色で光る。禁呪では火の粉になる
    let particle_hue = if c.forbidden {
        20.0
    } else {
        (c.hue + 180.0) % 360.0
    };
    particles(&mut s, &mut rng, particles_count, particle_hue);
    let reach = match c.tier {
        // 禁呪は、引いた格にかかわらず超極大魔法になる
        _ if c.forbidden => {
            beyond(&mut s, c, spell, t, &mut rng);
            BAND_OUTER
        }
        Tier::Ultimate => {
            ultimate(&mut s, c, spell, t, &mut rng);
            BAND_OUTER
        }
        Tier::Large => {
            large(&mut s, c, spell, t, &mut rng);
            BAND_OUTER
        }
        Tier::Small | Tier::Medium => {
            body(&mut s, c, spell, t, &mut rng);
            BAND_OUTER * scale_of(c)
        }
    };

    if st.flash > 0.0 {
        writeln!(
            s,
            r#"<circle r="{:.1}" fill="hsl({:.0},85%,65%)" opacity="{:.3}"/>"#,
            reach,
            c.hue,
            0.18 * st.flash
        )
        .unwrap();
    }
    s.push_str("</svg>\n");
    s
}

/// 魔法陣 1 枚ぶん（粒子と背景を除く）を、原点を中心に半径 `SIZE / 2` の中へ描く。
/// `rng` はルーンの字形に使う。
fn body(s: &mut String, c: &MagicCircle, spell: &str, t: Option<f32>, rng: &mut ChaCha8Rng) {
    let st = Stages::at(t);
    let hue = c.hue;
    let main = format!("hsl({hue:.0},85%,65%)");
    let sub = format!("hsl({:.0},80%,55%)", (hue + 35.0) % 360.0);
    let dir = if c.clockwise { 1.0 } else { -1.0 };
    let scale = scale_of(c);

    writeln!(
        s,
        r#"<g fill="none" stroke="{main}" stroke-linecap="round" stroke-linejoin="round" filter="url(#glow)" transform="rotate({:.2}) scale({scale})">"#,
        c.rotation
    )
    .unwrap();

    // 外周のルーン帯はゆっくり、内側の装飾は逆向きに速く回す。
    // 完成図では SMIL で回し続け（静止画に落とすと初期角で止まる）、
    // アニメーションのコマでは回りながら定位置へ収まっていく。
    open_layer(
        s,
        t,
        st.band,
        dir * 150.0 * (1.0 - st.band),
        dir * 360.0,
        90,
    );
    open_double(s);
    drawn_circle(s, BAND_OUTER, 4.0, st.band);
    drawn_circle(s, BAND_INNER, 2.5, st.band);
    s.push_str("</g>\n");
    let visible = (c.runes as f32 * st.runes).ceil() as u8;
    match c.band {
        Band::Runes | Band::Ticks => runes(s, rng, c.runes, visible, c.band),
        // 帯の幅いっぱいの大きな字で呪文を書く。区切りと字間は内側の帯と揃える
        Band::Script => script::ring(
            s,
            spell,
            &script::Hand {
                size: 1.0,
                ..c.hand
            },
            (BAND_OUTER + BAND_INNER) / 2.0,
            20.0,
            st.runes,
        ),
    }
    s.push_str("</g>\n");

    if st.rings > 0.0 {
        open_fade(s, st.rings);
        open_double(s);
        // 小魔法は内側の同心円を 2 本までにする
        let rings = if c.tier == Tier::Small {
            c.rings.min(4)
        } else {
            c.rings
        };
        inner_rings(s, rings, &sub);
        s.push_str("</g>\n");
        script::ring(s, spell, &c.hand, TEXT_OUTER, 10.0, st.rings);
        s.push_str("</g>\n");
    }

    if st.ornament > 0.0 {
        open_layer(
            s,
            t,
            st.ornament,
            -dir * 240.0 * (1.0 - st.ornament),
            -dir * 360.0,
            45,
        );
        match c.layout {
            Layout::Classic | Layout::Breach => {
                open_double(s);
                ornament(s, c.ornament, c.symmetry, &sub);
                s.push_str("</g>\n");
            }
            Layout::Grand => grand_star(s, c.symmetry, spell, c.hand.style, &main),
        }
        s.push_str("</g>\n");

        // 外周を破る多角形は、装飾とは逆にルーン帯と同じ向きへ回す
        if c.layout == Layout::Breach {
            open_layer(
                s,
                t,
                st.ornament,
                dir * 120.0 * (1.0 - st.ornament),
                dir * 360.0,
                120,
            );
            breach(s, c.symmetry);
            s.push_str("</g>\n");
        }
    }

    if st.core > 0.0 {
        open_fade(s, st.core);
        open_double(s);
        core(s, c.shape, c.symmetry);
        s.push_str("</g>\n");
        // 小魔法は呪文の帯を外側の 1 本だけにする
        if c.tier != Tier::Small {
            script::ring(s, spell, &c.hand, TEXT_INNER, 7.0, st.core);
        }
        s.push_str("</g>\n");
    }

    s.push_str("</g>\n");
}

/// 大魔法で、後ろの魔法陣の上に重ねる本来の魔法陣の倍率
const LARGE_CENTER: f32 = 0.84;
/// 大魔法で、後ろの魔法陣の色相のずらし幅と不透明度。
/// 隣り合う色（-60°）にして、極大魔法（120° ずつずらす）より控えめに見せる。
/// 150° などの遠い色は、薄くすると濁って見えた。
const LARGE_BACK_HUE: f32 = 300.0;
const LARGE_BACK_OPACITY: f32 = 0.6;

/// 大魔法。後ろに別の魔法陣を薄く、逆向きに回して敷き、本来の魔法陣を少し縮めて重ねる。
/// 極大魔法の外枠を控えめにした形で、後ろの魔法陣は透けて見える。
fn large(s: &mut String, c: &MagicCircle, spell: &str, t: Option<f32>, rng: &mut ChaCha8Rng) {
    let mut back = companion(c, spell, 0, LARGE_BACK_HUE);
    back.clockwise = !c.clockwise;
    writeln!(s, r#"<g opacity="{LARGE_BACK_OPACITY}">"#).unwrap();
    body(s, &back, spell, delayed(t, 0.0), &mut companion_rng(&back));
    s.push_str("</g>\n");
    placed(
        s,
        (0.0, 0.0),
        LARGE_CENTER,
        delayed(t, 0.1),
        0.35,
        |s, t| body(s, c, spell, t, rng),
    );
}

/// `delay` だけ遅れて始まり、`t` = 1 でそろう時刻。完成図（`None`）はそのまま。
fn delayed(t: Option<f32>, delay: f32) -> Option<f32> {
    t.map(|t| ((t - delay) / (1.0 - delay)).clamp(0.0, 1.0))
}

/// 極大魔法で、中心に据える本来の魔法陣の倍率
const ULTIMATE_CENTER: f32 = 0.6;
/// 極大魔法で、周りを回る小さな魔法陣の倍率と、中心からの距離
const ULTIMATE_ORBIT: f32 = 0.27;
const ULTIMATE_ORBIT_R: f32 = 345.0;

/// 極大魔法。種類の違う魔法陣を重ねる。
/// 外枠に別の魔法陣を大きく敷き、中心に本来の魔法陣を縮めて据え、
/// 外枠の帯にかかる位置へ小さな魔法陣を対称に配る。外から順に時間差で展開する。
fn ultimate(s: &mut String, c: &MagicCircle, spell: &str, t: Option<f32>, rng: &mut ChaCha8Rng) {
    let local = |delay: f32| delayed(t, delay);

    let frame = companion(c, spell, 0, 120.0);
    body(s, &frame, spell, local(0.0), &mut companion_rng(&frame));

    placed(s, (0.0, 0.0), ULTIMATE_CENTER, local(0.15), 0.8, |s, t| {
        body(s, c, spell, t, rng)
    });

    let n = orbit_count(c);
    for i in 0..n {
        let theta = TAU * i as f32 / n as f32 + c.rotation.to_radians();
        let orbit = companion(c, spell, 1 + i, 240.0);
        placed(
            s,
            polar(ULTIMATE_ORBIT_R, theta),
            ULTIMATE_ORBIT,
            local(0.3 + 0.05 * i as f32),
            0.8,
            |s, t| body(s, &orbit, spell, t, &mut companion_rng(&orbit)),
        );
    }
}

/// 超極大魔法で、中に据える極大魔法の倍率
const BEYOND_CENTER: f32 = 0.66;
/// 超極大魔法で、外枠の帯の上に並ぶ小さな魔法陣の倍率と、中心からの距離
const BEYOND_ORBIT: f32 = 0.15;
const BEYOND_ORBIT_R: f32 = 430.0;

/// 超極大魔法（禁呪）。極大魔法をまるごと縮めて中に据え、さらに外枠を敷いて、
/// その帯の上へ極大魔法の倍の数の小さな魔法陣を並べる。外から順に時間差で展開する。
fn beyond(s: &mut String, c: &MagicCircle, spell: &str, t: Option<f32>, rng: &mut ChaCha8Rng) {
    let local = |delay: f32| delayed(t, delay);

    // 極大魔法が使う番号（0〜5）とは別の番号から引いて、中の極大魔法と絵がかぶらないようにする
    let mut frame = companion(c, spell, 10, 60.0);
    frame.tier = Tier::Medium;
    body(s, &frame, spell, local(0.0), &mut companion_rng(&frame));

    // 引いた格が小さくても、中身は極大魔法として組む
    let core = MagicCircle {
        tier: Tier::Ultimate,
        ..c.clone()
    };
    placed(s, (0.0, 0.0), BEYOND_CENTER, local(0.1), 0.85, |s, t| {
        ultimate(s, &core, spell, t, rng)
    });

    let n = 2 * orbit_count(c);
    for i in 0..n {
        let theta = TAU * (i as f32 + 0.5) / n as f32 + c.rotation.to_radians();
        let orbit = companion(c, spell, 11 + i, 180.0);
        placed(
            s,
            polar(BEYOND_ORBIT_R, theta),
            BEYOND_ORBIT,
            local(0.35 + 0.03 * i as f32),
            0.85,
            |s, t| body(s, &orbit, spell, t, &mut companion_rng(&orbit)),
        );
    }
}

/// 極大魔法で周りを回る魔法陣の数。対称性に合わせる
fn orbit_count(c: &MagicCircle) -> u8 {
    match c.symmetry {
        4 | 8 => 4,
        5 => 5,
        _ => 3,
    }
}

/// 大魔法・極大魔法に重ねる魔法陣。呪文に番号を足したものから引くので、同じ呪文なら毎回同じになる。
/// 色相は本来の魔法陣からずらして、重なっても見分けられるようにする。
fn companion(c: &MagicCircle, spell: &str, index: u8, hue_shift: f32) -> MagicCircle {
    let mut d = MagicCircle::from_command(&format!("{spell}\u{1f}{index}"));
    d.hue = (c.hue + hue_shift) % 360.0;
    // 禁呪は重ねた魔法陣まで赤く染める。見分けがつくよう、色相は火の色の範囲でだけずらす
    if c.forbidden {
        d.forbid();
        d.hue = (FORBIDDEN_HUE + hue_shift / 12.0) % 360.0;
    }
    // 小さく置くものは、外へはみ出す多角形を付けず、小魔法と同じく簡略に描く
    if index > 0 {
        if d.layout == Layout::Breach {
            d.layout = Layout::Classic;
        }
        d.tier = Tier::Small;
    } else {
        d.tier = Tier::Medium;
    }
    d
}

fn companion_rng(c: &MagicCircle) -> ChaCha8Rng {
    let mut rng = ChaCha8Rng::from_seed(c.hash);
    rng.set_stream(1);
    rng
}

/// `at` を中心に `scale` 倍で魔法陣を置く。下の線が透けすぎないよう、背景色の円を
/// 不透明度 `veil` で敷く。
/// 展開が始まる前（`t` が 0）には何も描かない。
fn placed(
    s: &mut String,
    at: (f32, f32),
    scale: f32,
    t: Option<f32>,
    veil: f32,
    draw: impl FnOnce(&mut String, Option<f32>),
) {
    let p = t.unwrap_or(1.0);
    if p <= 0.0 {
        return;
    }
    writeln!(
        s,
        r#"<g transform="translate({:.2} {:.2}) scale({scale})">"#,
        at.0, at.1
    )
    .unwrap();
    writeln!(
        s,
        r#"<circle r="{:.1}" fill="{BACKGROUND}" opacity="{:.3}"/>"#,
        BAND_OUTER + 10.0,
        veil * ease(p.min(0.3) / 0.3)
    )
    .unwrap();
    draw(s, t);
    s.push_str("</g>\n");
}

/// 外周を破る多角形のぶんだけ縮める
fn scale_of(c: &MagicCircle) -> f32 {
    if c.layout == Layout::Breach {
        BREACH_SCALE
    } else {
        1.0
    }
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
fn grand_star(s: &mut String, n: u8, spell: &str, style: script::Style, color: &str) {
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
                script::glyph_d(style, b, 11.0, 18.0),
                360.0 * i as f32 / n as f32
            )
            .unwrap();
        }
    }
    s.push_str("</g>\n");
}

/// 外周の円を頂点が突き破る大きな三角か四角。頂点には小さな二重円を置く。
fn breach(s: &mut String, symmetry: u8) {
    let k = match symmetry {
        3 | 4 => symmetry,
        n if n % 2 == 0 => 4,
        _ => 3,
    };
    open_double(s);
    polygon(s, k, BREACH, 0.0, 3.5);
    for i in 0..k {
        let (x, y) = polar(BREACH, TAU * i as f32 / k as f32);
        writeln!(
            s,
            r#"<circle cx="{x:.2}" cy="{y:.2}" r="14" stroke-width="2.5"/>"#
        )
        .unwrap();
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
fn runes(s: &mut String, rng: &mut ChaCha8Rng, count: u8, visible: u8, band: Band) {
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
        let gap = TAU / count as f32;
        if band == Band::Ticks {
            // 字と字の間を目盛りで刻む。帯の内と外の縁から短い線を伸ばす
            for k in 1..6 {
                let a = theta + gap * k as f32 / 6.0;
                let len = if k == 3 { 14.0 } else { 7.0 };
                line(
                    s,
                    polar(BAND_INNER + 3.0, a),
                    polar(BAND_INNER + 3.0 + len, a),
                    2.0,
                );
                line(
                    s,
                    polar(BAND_OUTER - 3.0, a),
                    polar(BAND_OUTER - 3.0 - len, a),
                    2.0,
                );
            }
        } else {
            // 字と字の間に小さな点を打ち、字が少ないときでも帯が途切れて見えないようにする
            let (dx, dy) = polar(mid, theta + gap / 2.0);
            writeln!(s, r#"<circle cx="{dx:.2}" cy="{dy:.2}" r="3.5"/>"#).unwrap();
        }
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
        Ornament::Crown => {
            // 3 と 4 は辺が中心図形にかかるので、倍の角数で組む
            let k = if n < 5 { 2 * n } else { n } as u32;
            let base = CORE / (TAU / (2 * k) as f32).cos() + 25.0;
            let apex = RINGS_OUTER - 10.0;
            circle(s, apex, 1.2);
            for i in 0..k {
                let a = polar(base, TAU * i as f32 / k as f32);
                let b = polar(base, TAU * (i + 1) as f32 / k as f32);
                let top = polar(apex, TAU * (i as f32 + 0.5) / k as f32);
                line(s, a, b, 2.0);
                line(s, a, top, 1.6);
                line(s, top, b, 1.6);
            }
        }
        Ornament::Web => {
            let k = if n < 5 { 2 * n } else { n } as u32;
            let inner = CORE / (TAU / (2 * k) as f32).cos() + 20.0;
            let outer = RINGS_OUTER - 20.0;
            polygon(s, k as u8, outer, 0.0, 2.0);
            polygon(s, k as u8, inner, TAU / (2 * k) as f32, 2.0);
            // 外の頂点から内の両隣の頂点へ引いて、三角の帯にする
            for i in 0..k {
                let o = polar(outer, TAU * i as f32 / k as f32);
                let l = polar(inner, TAU * (i as f32 - 0.5) / k as f32);
                let r = polar(inner, TAU * (i as f32 + 0.5) / k as f32);
                line(s, o, l, 1.4);
                line(s, o, r, 1.4);
            }
        }
        Ornament::Beads => {
            let (lo, hi) = (ORNAMENT - 28.0, ORNAMENT + 28.0);
            circle(s, lo, 1.6);
            circle(s, hi, 1.6);
            let k = 2 * n as u32;
            for i in 0..k {
                let (x, y) = polar(ORNAMENT, TAU * i as f32 / k as f32);
                // 対称の頂点に来る玉だけ大きくする
                let r = if i % 2 == 0 { 21.0 } else { 12.0 };
                writeln!(
                    s,
                    r#"<circle cx="{x:.2}" cy="{y:.2}" r="{r}" stroke-width="2"/>"#
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
        Shape::Wheel => {
            // 多角形の頂点へ、中心の二重円からスポークを出す
            let k = if n < 5 { 2 * n } else { n };
            polygon(s, k, CORE, 0.0, 3.0);
            circle(s, CORE * 0.3, 2.0);
            circle(s, CORE * 0.37, 2.0);
            for i in 0..k as u32 {
                line(s, at(CORE * 0.37, i, k as u32), at(CORE, i, k as u32), 2.0);
                // 隣の頂点との中点へも細いスポークを出して、扇に割る
                let mid = polar(
                    CORE * (TAU / (2 * k) as f32).cos(),
                    TAU * (i as f32 + 0.5) / k as f32,
                );
                line(s, at(CORE * 0.37, i, k as u32), mid, 1.0);
            }
        }
        Shape::Satellites => {
            // 多角形の中の、頂点寄りに小さな円を浮かべる
            let k = n.clamp(3, 6);
            polygon(s, k, CORE, 0.0, 3.0);
            polygon(s, k, CORE * 0.42, TAU / (2 * k) as f32, 1.8);
            for i in 0..k as u32 {
                let (x, y) = at(CORE * 0.62, i, k as u32);
                writeln!(
                    s,
                    r#"<circle cx="{x:.2}" cy="{y:.2}" r="{:.1}" stroke-width="2.2"/>"#,
                    CORE * 0.12
                )
                .unwrap();
            }
            circle(s, CORE * 0.12, 2.0);
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
    writeln!(s, r#"<g fill="hsl({hue:.0},90%,80%)">"#).unwrap();
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
    fn forbidden_spells_go_beyond_ultimate() {
        let placements = |svg: &str| svg.matches(r#"<g transform="translate("#).count();
        let (c, spell) = (0..)
            .map(|i| format!("rm -rf dir{i}"))
            .map(|cmd| (MagicCircle::from_command(&cmd), cmd))
            .find(|(c, _)| c.tier == Tier::Ultimate)
            .unwrap();
        let ultimate = svg(&c, &spell);
        let mut sealed = c.clone();
        sealed.forbid();
        let beyond = svg(&sealed, &spell);
        // 極大魔法をまるごと中に抱え、その外に倍の数の魔法陣を並べる
        let n = orbit_count(&c) as usize;
        assert_eq!(placements(&ultimate), 1 + n);
        assert_eq!(placements(&beyond), 1 + (1 + n) + 2 * n);
        assert!(beyond.contains(&format!("hsl({FORBIDDEN_HUE:.0},")));
        // 引いた格が小さくても、禁呪なら超極大魔法として描く
        let mut small = MagicCircle::from_command("ls");
        small.forbid();
        assert_eq!(
            placements(&svg(&small, "ls")),
            1 + (1 + orbit_count(&small) as usize) + 2 * orbit_count(&small) as usize
        );
    }

    /// 帯が `band` の魔法陣になるコマンド
    fn with_band(band: Band) -> (MagicCircle, String) {
        (0..)
            .map(|i| format!("cmd {i}"))
            .map(|cmd| (MagicCircle::from_command(&cmd), cmd))
            // 重ね描きや簡略化の無い中魔法から選ぶ
            .find(|(c, _)| c.band == band && c.tier == Tier::Medium)
            .unwrap()
    }

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
        let (c, spell) = with_band(Band::Runes);
        let spell = spell.as_str();
        assert_eq!(frame(&c, spell, 0.0).matches(RUNE).count(), 0);
        assert_eq!(
            frame(&c, spell, 1.0).matches(RUNE).count(),
            c.runes as usize
        );
        // 途中のコマでも字形は完成図と同じものが先頭から並ぶ
        let half = frame(&c, spell, 0.4);
        let full = frame(&c, spell, 1.0);
        let first_rune = |s: &str| s.split(RUNE).nth(1).map(|p| p[..60].to_string());
        assert_eq!(first_rune(&half), first_rune(&full));
        // 光りは完成の直前にだけ出て、完成のコマでは収まっている
        // 光りは、塗りのある大きな円として 1 枚だけ重ねる
        let flash = |s: &str| {
            s.lines()
                .any(|l| l.starts_with("<circle r=") && l.contains(r#"fill="hsl"#))
        };
        assert!(flash(&frame(&c, spell, 0.9)));
        assert!(!flash(&full));
    }

    #[test]
    fn draws_every_rune_and_particle() {
        let (c, spell) = with_band(Band::Runes);
        let out = svg(&c, &spell);
        assert!(out.starts_with("<svg"));
        assert!(out.trim_end().ends_with("</svg>"));
        assert_eq!(out.matches(RUNE).count(), c.runes as usize);
        assert_eq!(out.matches(r#"opacity=""#).count(), c.particles as usize);
    }

    #[test]
    fn ultimate_layers_several_circles() {
        let c = MagicCircle::from_command("cargo run");
        assert_eq!(c.tier, Tier::Ultimate);
        // 外枠 1 + 中心 1 + 衛星 n。中心と衛星は translate して置く
        let placed = |s: &str| s.matches(r#"<g transform="translate("#).count();
        let orbits = match c.symmetry {
            4 | 8 => 4,
            5 => 5,
            _ => 3,
        };
        assert_eq!(placed(&svg(&c, "cargo run")), 1 + orbits);
        // 外から順に展開する。始まってすぐは外枠だけ
        assert_eq!(placed(&frame(&c, "cargo run", 0.1)), 0);
        assert_eq!(placed(&frame(&c, "cargo run", 1.0)), 1 + orbits);
        // 極大魔法でない魔法陣は 1 枚だけ
        assert_eq!(placed(&svg(&MagicCircle::from_command("ls"), "ls")), 0);
    }

    fn with_tier(tier: Tier) -> (MagicCircle, String) {
        (0..)
            .map(|i| format!("cmd {i}"))
            .map(|cmd| (MagicCircle::from_command(&cmd), cmd))
            .find(|(c, _)| c.tier == tier)
            .unwrap()
    }

    #[test]
    fn large_puts_one_circle_behind() {
        let (c, spell) = with_tier(Tier::Large);
        let out = svg(&c, &spell);
        assert_eq!(out.matches(r#"<g transform="translate("#).count(), 1);
        let back = format!(r#"<g opacity="{LARGE_BACK_OPACITY}">"#);
        assert_eq!(out.matches(&back).count(), 1);
    }

    #[test]
    fn small_is_simpler() {
        // 内側の呪文の帯（半径 TEXT_INNER）は小魔法にだけ無い
        let inner = format!("translate(0 {:.2})", -TEXT_INNER);
        let (small, spell) = with_tier(Tier::Small);
        assert!(!svg(&small, &spell).contains(&inner));
        let (medium, spell) = with_tier(Tier::Medium);
        assert!(svg(&medium, &spell).contains(&inner));
    }

    #[test]
    fn band_styles() {
        // 目盛りは字の間に 5 本ずつ、内と外の縁で 2 倍
        let (c, spell) = with_band(Band::Ticks);
        let out = svg(&c, &spell);
        assert_eq!(out.matches(RUNE).count(), c.runes as usize);
        let ticks = |s: &str| s.matches("<line ").count();
        let (plain, plain_spell) = with_band(Band::Runes);
        let base = ticks(&svg(&plain, &plain_spell));
        assert!(
            ticks(&out) >= c.runes as usize * 10,
            "{} {base}",
            ticks(&out)
        );
        // 呪文の帯にはルーンが無い
        let (c, spell) = with_band(Band::Script);
        assert_eq!(svg(&c, &spell).matches(RUNE).count(), 0);
    }
}
