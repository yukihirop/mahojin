//! コマンド文字列から魔法陣のパラメータを決定論的に導く。
//!
//! SHA-256 → ChaCha8 (seeded RNG) → `MagicCircle`。
//! 同じコマンドはどのマシンでも同じ魔法陣になり、1 文字違えば別物になる。

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};
use sha2::{Digest, Sha256};

use crate::locale::Locale;
use crate::omen::Omen;
use crate::script::{Hand, Separator, Style};

/// 魔法陣の中心に据える図形。ハッシュの先頭バイトがそのまま決める。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    Wheel,
    Satellites,
}

impl Shape {
    pub const ALL: [Shape; 12] = [
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
        Shape::Wheel,
        Shape::Satellites,
    ];

    /// 0x00..=0xff を種類の数で等分し、先頭バイトが入った区間の図形にする。
    fn from_byte(b: u8) -> Self {
        Self::ALL[b as usize * Self::ALL.len() / 256]
    }

    pub fn name(self, l: Locale) -> &'static str {
        match self {
            Shape::Circle => l.pick("円", "circle"),
            Shape::Triangle => l.pick("三角", "triangle"),
            Shape::Hexagram => l.pick("六芒星", "hexagram"),
            Shape::MultiCircle => l.pick("多重円", "concentric circles"),
            Shape::Pentagram => l.pick("五芒星", "pentagram"),
            Shape::Octagram => l.pick("八芒星", "octagram"),
            Shape::NestedPolygons => l.pick("入れ子多角形", "nested polygons"),
            Shape::Spiral => l.pick("螺旋", "spiral"),
            Shape::Petals => l.pick("花弁", "petals"),
            Shape::Metatron => l.pick("メタトロン", "Metatron's cube"),
            Shape::Wheel => l.pick("車輪", "wheel"),
            Shape::Satellites => l.pick("衛星", "satellites"),
        }
    }
}

/// 中心図形の外側を囲む、`symmetry` 回対称の装飾。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ornament {
    /// 頂点を飛ばして結ぶ星形と、頂点の小円
    Star,
    /// 円周上に並べて隣と接する円の鎖
    Chain,
    /// 中心から伸びる光条と、先端の菱形
    Rays,
    /// 多角形の各辺に三角を載せ、頂点を外の円に届かせた冠
    Crown,
    /// 外と内の多角形をジグザグの三角でつないだ網
    Web,
    /// 2 本の円の間に小さな円を並べた数珠
    Beads,
}

impl Ornament {
    pub const ALL: [Ornament; 6] = [
        Ornament::Star,
        Ornament::Chain,
        Ornament::Rays,
        Ornament::Crown,
        Ornament::Web,
        Ornament::Beads,
    ];

    pub fn name(self, l: Locale) -> &'static str {
        match self {
            Ornament::Star => l.pick("星形", "star"),
            Ornament::Chain => l.pick("円鎖", "chain"),
            Ornament::Rays => l.pick("光条", "rays"),
            Ornament::Crown => l.pick("冠", "crown"),
            Ornament::Web => l.pick("網", "web"),
            Ornament::Beads => l.pick("数珠", "beads"),
        }
    }
}

/// 全体の配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layout {
    /// 中心図形を装飾が囲み、外周にルーン帯がある
    Classic,
    /// 星が魔法陣いっぱいに広がり、頂点の円が外周の帯に重なる
    Grand,
    /// 標準の配置に、頂点が外周を突き破る大きな三角か四角を重ねる
    Breach,
}

impl Layout {
    pub const ALL: [Layout; 3] = [Layout::Classic, Layout::Grand, Layout::Breach];

    pub fn name(self, l: Locale) -> &'static str {
        match self {
            Layout::Classic => l.pick("標準", "classic"),
            Layout::Grand => l.pick("大星", "grand star"),
            Layout::Breach => l.pick("突破", "breach"),
        }
    }
}

/// 外周の帯に何を並べるか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Band {
    /// ルーンの間に点を打つ
    Runes,
    /// 呪文そのものを大きな字で書く
    Script,
    /// ルーンの間を、時計の文字盤のような目盛りで埋める
    Ticks,
}

impl Band {
    pub const ALL: [Band; 3] = [Band::Runes, Band::Script, Band::Ticks];

    pub fn name(self, l: Locale) -> &'static str {
        match self {
            Band::Runes => l.pick("ルーン", "runes"),
            Band::Script => l.pick("呪文", "script"),
            Band::Ticks => l.pick("目盛り", "ticks"),
        }
    }
}

/// 魔法の格。端末に出す魔法陣の大きさが変わる。大きいものほど出にくい。
/// 極大魔法だけは、種類の違う魔法陣を重ねて描く。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    Small,
    Medium,
    Large,
    Ultimate,
}

impl Tier {
    pub const ALL: [Tier; 4] = [Tier::Small, Tier::Medium, Tier::Large, Tier::Ultimate];

    /// 百分率で引いた値から格を決める。小 35% / 中 45% / 大 19% / 極大 1%
    fn from_roll(roll: u32) -> Self {
        match roll {
            0..35 => Tier::Small,
            35..80 => Tier::Medium,
            80..99 => Tier::Large,
            _ => Tier::Ultimate,
        }
    }

    /// 端末に出すときの高さ（行数）
    pub fn rows(self) -> u32 {
        match self {
            Tier::Small => 8,
            Tier::Medium => 16,
            Tier::Large => 24,
            Tier::Ultimate => 36,
        }
    }

    pub fn name(self, l: Locale) -> &'static str {
        match self {
            Tier::Small => l.pick("小魔法", "minor spell"),
            Tier::Medium => l.pick("中魔法", "standard spell"),
            Tier::Large => l.pick("大魔法", "major spell"),
            Tier::Ultimate => l.pick("極大魔法", "ultimate spell"),
        }
    }
}

pub const SYMMETRIES: [u8; 6] = [3, 4, 5, 6, 8, 12];

/// 禁呪の色相（深紅）
pub const FORBIDDEN_HUE: f32 = 355.0;
/// 神聖魔法の色相（金）
pub const HOLY_HUE: f32 = 46.0;
/// 召喚魔法の色相（藍）
pub const SUMMON_HUE: f32 = 228.0;

#[derive(Debug, Clone, PartialEq)]
pub struct MagicCircle {
    pub hash: [u8; 32],
    pub layout: Layout,
    pub shape: Shape,
    /// `Layout::Grand` では描かれず、大きな星が代わりを務める
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
    /// 呪文を書く筆致（書体・大きさ・字間・区切り）
    pub hand: Hand,
    pub band: Band,
    pub tier: Tier,
    /// 呪文の暴走。極大魔法から上がグラデーションで光る。それより下の格では何も変わらない。
    /// 唱えるたびの運で決まるので、`from_hash` では常に false
    pub aurora: bool,
    /// 禁呪。ハッシュではなくコマンドの中身で決まるので、`from_hash` では常に false
    pub forbidden: bool,
    /// 神聖魔法。禁呪と同じく、`from_hash` では常に false
    pub holy: bool,
    /// 召喚魔法。これも `from_hash` では常に false
    pub summoned: bool,
    /// 暦の兆し。唱えた日時で決まるので、`from_hash` では常に `None`
    pub omen: Option<Omen>,
}

impl MagicCircle {
    pub fn from_command(command: &str) -> Self {
        Self::from_hash(Sha256::digest(command.as_bytes()).into())
    }

    /// パラメータはすべてハッシュから決まる。図鑑はコマンドを覚えずハッシュだけで引き直す。
    pub fn from_hash(hash: [u8; 32]) -> Self {
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
            layout: Layout::ALL[below(&mut rng, 3) as usize],
            hand: Hand {
                style: Style::ALL[below(&mut rng, Style::ALL.len() as u32) as usize],
                size: 0.7 + unit(&mut rng) * 0.7,
                spacing: 0.3 + unit(&mut rng) * 1.3,
                separator: Separator::ALL[below(&mut rng, Separator::ALL.len() as u32) as usize],
            },
            band: Band::ALL[below(&mut rng, Band::ALL.len() as u32) as usize],
            tier: Tier::from_roll(below(&mut rng, 100)),
            aurora: false,
            forbidden: false,
            holy: false,
            summoned: false,
            omen: None,
        }
    }

    /// 禁呪にする。中心の魔法陣の形はハッシュのまま、深紅に染めて逆さに回し、超極大魔法として描く。
    pub fn forbid(&mut self) {
        self.forbidden = true;
        self.hue = FORBIDDEN_HUE;
        self.clockwise = false;
    }

    /// 神聖魔法にする。形はハッシュのまま、金と白に染めて右に回し、禁呪よりも大きな超極大魔法として描く。
    pub fn sanctify(&mut self) {
        self.holy = true;
        self.hue = HOLY_HUE;
        self.clockwise = true;
    }

    /// 召喚魔法にする。形はハッシュのまま、銀と藍に染めて、引いた格にかかわらず極大魔法として描く。
    pub fn summon(&mut self) {
        self.summoned = true;
        self.hue = SUMMON_HUE;
    }

    /// 暦の色に染める。形はハッシュのまま。13 日の金曜日だけは逆さに回す。
    pub fn bless(&mut self, omen: Omen) {
        self.omen = Some(omen);
        self.hue = omen.palette().main.0;
        if omen == Omen::Friday13 {
            self.clockwise = false;
        }
    }

    /// 格の名前。禁呪は引いた格にかかわらず超極大魔法になり、禁呪であることも名乗る。
    pub fn title(&self, l: Locale) -> &'static str {
        if self.holy {
            l.pick("神聖魔法", "holy spell")
        } else if self.summoned {
            l.pick("召喚魔法", "summoning")
        } else if self.forbidden {
            l.pick("超極大魔法（禁呪）", "super ultimate spell (forbidden)")
        } else {
            self.tier.name(l)
        }
    }

    /// 端末に出すときの高さ（行数）。超極大魔法は極大魔法より一回り大きく、神聖魔法はさらに大きい
    pub fn rows(&self) -> u32 {
        if self.holy {
            48
        } else if self.forbidden {
            44
        } else if self.summoned {
            Tier::Ultimate.rows()
        } else {
            self.tier.rows()
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
    fn omens_change_only_the_colors() {
        let plain = MagicCircle::from_command("git status");
        let mut blessed = plain.clone();
        blessed.bless(Omen::Halloween);
        assert_eq!(blessed.hue, 28.0);
        assert_eq!((blessed.shape, blessed.tier), (plain.shape, plain.tier));
        assert_eq!(blessed.clockwise, plain.clockwise);
        // 13 日の金曜日だけは逆さに回る
        blessed.bless(Omen::Friday13);
        assert!(!blessed.clockwise);
    }

    #[test]
    fn holy_spells_keep_the_shape() {
        let plain = MagicCircle::from_command("gh api -X PUT /user/starred/yukihirop/mahojin");
        let mut holy = plain.clone();
        holy.sanctify();
        assert_eq!((holy.hue, holy.clockwise), (HOLY_HUE, true));
        assert_eq!((holy.shape, holy.layout), (plain.shape, plain.layout));
        assert!(holy.rows() > Tier::Ultimate.rows());
        assert_eq!(holy.title(Locale::Ja), "神聖魔法");
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
            assert!((0.7..1.4).contains(&c.hand.size));
            assert!((0.3..1.6).contains(&c.hand.spacing));
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
        assert!(circles.iter().any(|c| c.layout == Layout::Breach));
        for style in Style::ALL {
            assert!(circles.iter().any(|c| c.hand.style == style), "{style:?}");
        }
        for sep in Separator::ALL {
            assert!(circles.iter().any(|c| c.hand.separator == sep), "{sep:?}");
        }
        for tier in Tier::ALL {
            assert!(circles.iter().any(|c| c.tier == tier), "{tier:?}");
        }
        // 大きいほど出にくい
        let count = |t: Tier| circles.iter().filter(|c| c.tier == t).count();
        assert!(count(Tier::Medium) > count(Tier::Large));
        assert!(count(Tier::Large) > count(Tier::Ultimate));
        for band in Band::ALL {
            assert!(circles.iter().any(|c| c.band == band), "{band:?}");
        }
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
        assert_eq!(
            (c.hand.style, c.hand.separator, c.band),
            (Style::Dotted, Separator::None, Band::Runes)
        );
    }

    const GOLDEN_HASH: &str = "e62b04aadf39df1a47b771265e4ae5c452df3f1903d5c263ab00f088e86102f6";
    const GOLDEN_PARAMS: (Layout, Shape, Ornament, u8, u8, u8, u16, bool) = (
        Layout::Breach,
        Shape::Wheel,
        Ornament::Star,
        5,
        3,
        27,
        398,
        true,
    );
}
