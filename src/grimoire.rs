//! 図鑑。唱えた魔法陣を記録し、格や中心図形をどこまで集めたかを数える。
//!
//! 置き場所は `$XDG_DATA_HOME/maho/grimoire`（無ければ `~/.local/share/maho/grimoire`）。
//! コマンドは書かず、ハッシュと唱えた回数だけを 1 行ずつ残す。
//! パラメータはハッシュから引き直せるので、それで足りる（[`MagicCircle::from_hash`]）。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::circle::{Band, Layout, MagicCircle, Ornament, SYMMETRIES, Shape, Tier};
use crate::locale::Locale;
use crate::script::Style;

const HEADER: &str = "# maho grimoire v1";

/// 集める項目。図鑑の 1 マス。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Item {
    Tier(Tier),
    Shape(Shape),
    Layout(Layout),
    Ornament(Ornament),
    Symmetry(u8),
    Style(Style),
    Band(Band),
}

impl Item {
    pub fn name(self, l: Locale) -> String {
        match self {
            Item::Tier(t) => t.name(l).into(),
            Item::Shape(s) => s.name(l).into(),
            Item::Layout(x) => match l {
                Locale::Ja => format!("{}の陣", x.name(l)),
                Locale::En => format!("{} layout", x.name(l)),
            },
            Item::Ornament(o) => o.name(l).into(),
            Item::Symmetry(n) => match l {
                Locale::Ja => format!("{n} 回対称"),
                Locale::En => format!("{n}-fold symmetry"),
            },
            Item::Style(s) => s.name(l).into(),
            Item::Band(b) => match l {
                Locale::Ja => format!("{}の帯", b.name(l)),
                Locale::En => format!("{} band", b.name(l)),
            },
        }
    }
}

/// 図鑑の頁。見出しと、その頁に並ぶ項目。
pub fn pages() -> Vec<(&'static str, &'static str, Vec<Item>)> {
    vec![
        ("格", "tier", Tier::ALL.map(Item::Tier).to_vec()),
        ("中心図形", "shape", Shape::ALL.map(Item::Shape).to_vec()),
        ("陣形", "layout", Layout::ALL.map(Item::Layout).to_vec()),
        (
            "装飾",
            "ornament",
            Ornament::ALL.map(Item::Ornament).to_vec(),
        ),
        ("対称", "symmetry", SYMMETRIES.map(Item::Symmetry).to_vec()),
        ("書体", "script", Style::ALL.map(Item::Style).to_vec()),
        ("外周の帯", "band", Band::ALL.map(Item::Band).to_vec()),
    ]
}

/// その魔法陣を見て埋まる項目。
fn items_of(c: &MagicCircle) -> Vec<Item> {
    let mut items = vec![
        Item::Tier(c.tier),
        Item::Shape(c.shape),
        Item::Layout(c.layout),
        Item::Symmetry(c.symmetry),
        Item::Style(c.hand.style),
        Item::Band(c.band),
    ];
    // 大星の陣では装飾が描かれないので、見たことにしない
    if c.layout != Layout::Grand {
        items.push(Item::Ornament(c.ornament));
    }
    items
}

/// 骨格。一目で形が違うと分かる組み合わせ（中心図形 × 陣形 × 格）。
pub const SKELETONS: usize = Shape::ALL.len() * Layout::ALL.len() * Tier::ALL.len();

#[derive(Debug, Default, PartialEq)]
pub struct Grimoire {
    /// 初めて唱えた順に、ハッシュと唱えた回数
    entries: Vec<([u8; 32], u64)>,
}

impl Grimoire {
    /// 読めない行は飛ばす。ファイルが無ければ空の図鑑。
    pub fn parse(text: &str) -> Self {
        let entries = text
            .lines()
            .filter_map(|line| {
                let (hex, count) = line.split_once(' ')?;
                Some((parse_hash(hex)?, count.trim().parse().ok()?))
            })
            .collect();
        Grimoire { entries }
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Self::parse(&text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    fn to_text(&self) -> String {
        let mut s = format!("{HEADER}\n");
        for (hash, count) in &self.entries {
            let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
            s.push_str(&format!("{hex} {count}\n"));
        }
        s
    }

    /// 一時ファイルに書いてから置き換える。同時に唱えても壊れたファイルは残らない。
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        std::fs::write(&tmp, self.to_text())?;
        std::fs::rename(&tmp, path)
    }

    /// 唱えた魔法陣を記録し、今回初めて埋まった項目を返す。
    pub fn record(&mut self, c: &MagicCircle) -> Vec<Item> {
        // 前にも唱えた呪文なら、埋まる項目は無い
        if let Some((_, count)) = self.entries.iter_mut().find(|(h, _)| *h == c.hash) {
            *count += 1;
            return Vec::new();
        }
        let before = self.items();
        self.entries.push((c.hash, 1));
        items_of(c)
            .into_iter()
            .filter(|i| !before.contains(i))
            .collect()
    }

    fn circles(&self) -> impl Iterator<Item = MagicCircle> + '_ {
        self.entries.iter().map(|(h, _)| MagicCircle::from_hash(*h))
    }

    pub fn items(&self) -> HashSet<Item> {
        self.circles().flat_map(|c| items_of(&c)).collect()
    }

    /// 格ごとの、その格になった呪文の種類数
    pub fn spells_of(&self, tier: Tier) -> usize {
        self.circles().filter(|c| c.tier == tier).count()
    }

    pub fn skeletons(&self) -> usize {
        self.circles()
            .map(|c| (c.shape, c.layout, c.tier))
            .collect::<HashSet<_>>()
            .len()
    }

    /// 呪文の種類数と、延べの回数
    pub fn casts(&self) -> (usize, u64) {
        (
            self.entries.len(),
            self.entries.iter().map(|(_, n)| n).sum(),
        )
    }
}

fn parse_hash(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 || !hex.is_ascii() {
        return None;
    }
    let mut hash = [0u8; 32];
    for (i, b) in hash.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(hash)
}

pub fn path(env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let base = match env("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env("HOME")?).join(".local").join("share"),
    };
    Some(base.join("maho").join("grimoire"))
}

/// `MAHO_GRIMOIRE=off` なら記録しない。
pub fn enabled(env: impl Fn(&str) -> Option<String>) -> bool {
    env("MAHO_GRIMOIRE").as_deref() != Some("off")
}

/// `maho --grimoire` で見せる頁。
pub fn show(g: &Grimoire, l: Locale) -> String {
    let found = g.items();
    let pages = pages();
    let total: usize = pages.iter().map(|(_, _, items)| items.len()).sum();
    let have = found.len();
    let unknown = l.pick("？？？", "???");

    let mut s = format!(
        "✦ {}  {have} / {total}  {}  {}%\n",
        l.pick("魔法陣図鑑", "Grimoire"),
        bar(have, total, 20),
        have * 100 / total
    );
    let (spells, casts) = g.casts();
    s.push_str(&match l {
        Locale::Ja => format!("  唱えた呪文 {spells} 種 / 延べ {casts} 回\n\n"),
        Locale::En => format!("  {spells} spells cast, {casts} times in all\n\n"),
    });
    for (ja, en, items) in &pages {
        let n = items.iter().filter(|i| found.contains(i)).count();
        let names: Vec<String> = items
            .iter()
            .map(|&i| match i {
                _ if !found.contains(&i) => unknown.to_string(),
                Item::Tier(t) => format!("{} {}", t.name(l), g.spells_of(t)),
                Item::Symmetry(n) => n.to_string(),
                Item::Layout(x) => x.name(l).into(),
                Item::Band(b) => b.name(l).into(),
                _ => i.name(l),
            })
            .collect();
        s.push_str(&format!(
            "  {} {:>6}  {}\n",
            pad(l.pick(ja, en), 10),
            format!("{n}/{}", items.len()),
            names.join(l.pick("  ", ", "))
        ));
    }
    s.push_str(&match l {
        Locale::Ja => format!(
            "\n  {} {:>6}  中心図形 × 陣形 × 格\n",
            pad("骨格", 10),
            format!("{}/{SKELETONS}", g.skeletons())
        ),
        Locale::En => format!(
            "\n  {} {:>6}  shape × layout × tier\n",
            pad("skeleton", 10),
            format!("{}/{SKELETONS}", g.skeletons())
        ),
    });
    if have == total {
        s.push_str(l.pick(
            "\n✦ 図鑑が埋まりました。骨格も集めてみてください\n",
            "\n✦ The grimoire is complete. Now try collecting every skeleton\n",
        ));
    }
    s
}

/// 図鑑に新しく載ったことを知らせる 1 行。
pub fn announce(new: &[Item], l: Locale) -> String {
    let names: Vec<String> = new.iter().map(|i| i.name(l)).collect();
    format!(
        "✦ {}: {}",
        l.pick("図鑑に記録", "New in the grimoire"),
        names.join(" / ")
    )
}

fn bar(have: usize, total: usize, width: usize) -> String {
    let filled = have * width / total;
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

/// 全角を 2 桁と数えて右を空白で埋める。
fn pad(s: &str, width: usize) -> String {
    let w: usize = s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum();
    format!("{s}{}", " ".repeat(width.saturating_sub(w)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_counts() {
        let mut g = Grimoire::default();
        let c = MagicCircle::from_command("git status");
        let new = g.record(&c);
        assert!(new.contains(&Item::Tier(c.tier)));
        assert!(new.contains(&Item::Shape(c.shape)));
        // 2 回目は何も増えないが、回数は数える
        assert!(g.record(&c).is_empty());
        assert_eq!(g.casts(), (1, 2));
        assert_eq!(g.skeletons(), 1);
    }

    #[test]
    fn round_trip_keeps_no_command() {
        let mut g = Grimoire::default();
        g.record(&MagicCircle::from_command("secret --token abc"));
        g.record(&MagicCircle::from_command("ls"));
        let text = g.to_text();
        assert!(!text.contains("secret") && !text.contains("abc"));
        assert_eq!(Grimoire::parse(&text), g);
        // 壊れた行は読み飛ばす
        let broken = format!("{text}zz 3\nnot a line\n");
        assert_eq!(Grimoire::parse(&broken), g);
    }

    #[test]
    fn grand_layout_hides_the_ornament() {
        let c = (0..)
            .map(|i| MagicCircle::from_command(&format!("cmd {i}")))
            .find(|c| c.layout == Layout::Grand)
            .unwrap();
        assert!(!items_of(&c).contains(&Item::Ornament(c.ornament)));
    }

    #[test]
    fn many_spells_fill_the_book() {
        // record を 3000 回呼ぶと毎回全体を数え直して遅いので、直接詰める
        let g = Grimoire {
            entries: (0..3000)
                .map(|i| (MagicCircle::from_command(&format!("cmd {i}")).hash, 1))
                .collect(),
        };
        let total: usize = pages().iter().map(|(_, _, items)| items.len()).sum();
        assert_eq!(g.items().len(), total);
        let page = show(&g, Locale::Ja);
        assert!(page.contains(&format!("{total} / {total}")));
        assert!(!page.contains("？？？"));
    }

    #[test]
    fn empty_book_hides_names() {
        let page = show(&Grimoire::default(), Locale::En);
        assert!(page.contains("0 / 38"));
        assert!(page.contains("???"));
        assert!(!page.contains("hexagram"));
    }

    #[test]
    fn path_follows_xdg() {
        let env = |vars: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                vars.iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| v.to_string())
            }
        };
        assert_eq!(
            path(env(&[("XDG_DATA_HOME", "/d"), ("HOME", "/h")])),
            Some(PathBuf::from("/d/maho/grimoire"))
        );
        assert_eq!(
            path(env(&[("HOME", "/h")])),
            Some(PathBuf::from("/h/.local/share/maho/grimoire"))
        );
        assert!(!enabled(env(&[("MAHO_GRIMOIRE", "off")])));
        assert!(enabled(env(&[])));
    }
}
