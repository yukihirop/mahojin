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
}

impl Shape {
    fn from_byte(b: u8) -> Self {
        match b {
            0x00..=0x3f => Shape::Circle,
            0x40..=0x7f => Shape::Triangle,
            0x80..=0xbf => Shape::Hexagram,
            0xc0..=0xff => Shape::MultiCircle,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Shape::Circle => "円",
            Shape::Triangle => "三角",
            Shape::Hexagram => "六芒星",
            Shape::MultiCircle => "多重円",
        }
    }
}

const SYMMETRIES: [u8; 6] = [3, 4, 5, 6, 8, 12];

#[derive(Debug, Clone, PartialEq)]
pub struct MagicCircle {
    pub hash: [u8; 32],
    pub shape: Shape,
    /// 同心円の数 (2..=7)
    pub rings: u8,
    /// 回転対称の次数
    pub symmetry: u8,
    /// 外周に並ぶルーン文字の数 (8..=32)
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
            rings: 2 + below(&mut rng, 6) as u8,
            symmetry: SYMMETRIES[below(&mut rng, SYMMETRIES.len() as u32) as usize],
            runes: 8 + below(&mut rng, 25) as u8,
            rotation: unit(&mut rng) * 360.0,
            particles: below(&mut rng, 501) as u16,
            hue: unit(&mut rng) * 360.0,
            clockwise: rng.next_u32() & 1 == 0,
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
            assert!((2..=7).contains(&c.rings));
            assert!(SYMMETRIES.contains(&c.symmetry));
            assert!((8..=32).contains(&c.runes));
            assert!((0.0..360.0).contains(&c.rotation));
            assert!(c.particles <= 500);
            assert!((0.0..360.0).contains(&c.hue));
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
                c.shape,
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
    const GOLDEN_PARAMS: (Shape, u8, u8, u8, u16, bool) =
        (Shape::MultiCircle, 2, 5, 21, 118, false);
}
