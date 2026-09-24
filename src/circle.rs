//! コマンド文字列から魔法陣のパラメータを決定論的に導く。
//!
//! SHA-256 → ChaCha8 (seeded RNG) → `MagicCircle`。
//! 同じコマンドはどのマシンでも同じ魔法陣になり、1 文字違えば別物になる。

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};
use sha2::{Digest, Sha256};

/// 魔法陣の中心に据える図形。ハッシュの先頭バイトがそのまま決める。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Circle,
    Triangle,
    Hexagram,
    MultiCircle,
    Pentagram,
    Octagram,
    NestedPolygons,
    Spiral,
    Petals,
    Metatron,
}

impl Shape {
    const ALL: [Shape; 10] = [
        Shape::Circle,
        Shape::Triangle,
        Shape::Hexagram,
        Shape::MultiCircle,
        Shape::Pentagram,
        Shape::Octagram,
        Shape::NestedPolygons,
        Shape::Spiral,
        Shape::Petals,
        Shape::Metatron,
    ];

    /// 0x00..=0xff を種類の数で等分し、先頭バイトが入った区間の図形にする。
    fn from_byte(b: u8) -> Self {
        Self::ALL[b as usize * Self::ALL.len() / 256]
    }

    pub fn name(self) -> &'static str {
        match self {
            Shape::Circle => "円",
            Shape::Triangle => "三角",
            Shape::Hexagram => "六芒星",
            Shape::MultiCircle => "多重円",
            Shape::Pentagram => "五芒星",
            Shape::Octagram => "八芒星",
            Shape::NestedPolygons => "入れ子多角形",
            Shape::Spiral => "螺旋",
            Shape::Petals => "花弁",
            Shape::Metatron => "メタトロン",
        }
    }
}

/// 中心図形の外側を囲む、`symmetry` 回対称の装飾。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ornament {
    /// 頂点を飛ばして結ぶ星形と、頂点の小円
    Star,
    /// 円周上に並べて隣と接する円の鎖
    Chain,
    /// 中心から伸びる光条と、先端の菱形
    Rays,
}

impl Ornament {
    const ALL: [Ornament; 3] = [Ornament::Star, Ornament::Chain, Ornament::Rays];

    pub fn name(self) -> &'static str {
        match self {
            Ornament::Star => "星形",
            Ornament::Chain => "円鎖",
            Ornament::Rays => "光条",
        }
    }
}

/// 全体の配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// 中心図形を装飾が囲み、外周にルーン帯がある
    Classic,
    /// 星が魔法陣いっぱいに広がり、頂点の円が外周の帯に重なる
    Grand,
}

impl Layout {
    pub fn name(self) -> &'static str {
        match self {
            Layout::Classic => "標準",
            Layout::Grand => "大星",
        }
    }
}

const SYMMETRIES: [u8; 6] = [3, 4, 5, 6, 8, 12];

#[derive(Debug, Clone, PartialEq)]
pub struct MagicCircle {
    pub hash: [u8; 32],
    pub layout: Layout,
    pub shape: Shape,
    /// `Layout::Classic` のときだけ描かれる。`Grand` では大きな星が代わりを務める
    pub ornament: Ornament,
    /// 同心円の数 (3..=7)。外周のルーン帯の 2 本を含むので、内側には 1〜5 本
    pub rings: u8,
    /// 回転対称の次数
    pub symmetry: u8,
    /// 外周に並ぶルーン文字の数 (12..=32)
    pub runes: u8,
    /// 初期回転角 [0, 360)
    pub rotation: f32,
    /// 粒子数 (0..=500)
    pub particles: u16,
    /// 色相 [0, 360)
    pub hue: f32,
    /// 時計回りなら true
    pub clockwise: bool,
}

impl MagicCircle {
    pub fn from_command(command: &str) -> Self {
        let hash: [u8; 32] = Sha256::digest(command.as_bytes()).into();
        // 値の導き方は `rand` の分布実装に依存させない。
        // バージョンが上がっても同じコマンドが同じ魔法陣であり続けるため。
        let mut rng = ChaCha8Rng::from_seed(hash);

        MagicCircle {
            hash,
            shape: Shape::from_byte(hash[0]),
            ornament: Ornament::ALL[below(&mut rng, Ornament::ALL.len() as u32) as usize],
            rings: 3 + below(&mut rng, 5) as u8,
            symmetry: SYMMETRIES[below(&mut rng, SYMMETRIES.len() as u32) as usize],
            runes: 12 + below(&mut rng, 21) as u8,
            rotation: unit(&mut rng) * 360.0,
            particles: below(&mut rng, 501) as u16,
            hue: unit(&mut rng) * 360.0,
            clockwise: rng.next_u32() & 1 == 0,
            // 後から足したパラメータは必ず末尾で引く。前の値の割り当てが変わらず、
            // それまでの魔法陣のパラメータがそのまま保たれる。
            layout: if rng.next_u32() & 1 == 0 {
                Layout::Classic
            } else {
                Layout::Grand
            },
        }
    }

    pub fn hash_hex(&self) -> String {
        self.hash.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// [0, n) の整数。n は小さいので剰余の偏りは無視できる。
fn below(rng: &mut ChaCha8Rng, n: u32) -> u32 {
    rng.next_u32() % n
}

/// [0, 1) の実数。
pub(crate) fn unit(rng: &mut ChaCha8Rng) -> f32 {
    (rng.next_u32() >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_command_same_circle() {
        assert_eq!(
            MagicCircle::from_command("git status"),
            MagicCircle::from_command("git status")
        );
    }

    #[test]
    fn one_char_changes_circle() {
        assert_ne!(
            MagicCircle::from_command("ls").hash,
            MagicCircle::from_command("ls ").hash
        );
    }

    #[test]
    fn values_stay_in_range() {
        for i in 0..1000 {
            let c = MagicCircle::from_command(&format!("cmd {i}"));
            assert!((3..=7).contains(&c.rings));
            assert!(SYMMETRIES.contains(&c.symmetry));
            assert!((12..=32).contains(&c.runes));
            assert!((0.0..360.0).contains(&c.rotation));
            assert!(c.particles <= 500);
            assert!((0.0..360.0).contains(&c.hue));
        }
    }

    #[test]
    fn every_shape_and_ornament_appears() {
        let circles: Vec<_> = (0..2000)
            .map(|i| MagicCircle::from_command(&format!("cmd {i}")))
            .collect();
        for shape in Shape::ALL {
            assert!(circles.iter().any(|c| c.shape == shape), "{shape:?}");
        }
        assert!(circles.iter().any(|c| c.layout == Layout::Classic));
        assert!(circles.iter().any(|c| c.layout == Layout::Grand));
        for ornament in Ornament::ALL {
            assert!(
                circles.iter().any(|c| c.ornament == ornament),
                "{ornament:?}"
            );
        }
    }

    /// 依存クレートの更新などで魔法陣が変わったら落ちる。
    /// 意図して変えるときだけ期待値を書き換える。
    #[test]
    fn golden_git_status() {
        let c = MagicCircle::from_command("git status");
        assert_eq!(c.hash_hex(), GOLDEN_HASH);
        assert_eq!(
            (
                c.layout,
                c.shape,
                c.ornament,
                c.rings,
                c.symmetry,
                c.runes,
                c.particles,
                c.clockwise
            ),
            GOLDEN_PARAMS
        );
    }

    const GOLDEN_HASH: &str = "e62b04aadf39df1a47b771265e4ae5c452df3f1903d5c263ab00f088e86102f6";
    const GOLDEN_PARAMS: (Layout, Shape, Ornament, u8, u8, u8, u16, bool) = (
        Layout::Grand,
        Shape::Petals,
        Ornament::Star,
        5,
        3,
        27,
        398,
        true,
    );
}
