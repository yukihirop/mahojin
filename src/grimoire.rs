//! 図鑑。唱えた魔法陣を記録し、格や中心図形をどこまで集めたかを数える。
//!
//! 置き場所は `$XDG_DATA_HOME/maho/grimoire`（無ければ `~/.local/share/maho/grimoire`）。
//! コマンドは書かず、ハッシュと唱えた回数だけを 1 行ずつ残す。
//! パラメータはハッシュから引き直せるので、それで足りる（[`MagicCircle::from_hash`]）。
//! 禁呪かどうかだけはハッシュから分からないので、3 つめの欄に印を書く（`forbidden` か `doom:balse` など）。
//! 暦の兆しは呪文ではなく日時のものなので、出会ったものを `omen full-moon` のように別の行に残す。

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use crate::circle::{Band, Layout, MagicCircle, Ornament, SYMMETRIES, Shape, Tier};
use crate::forbidden::Doom;
use crate::locale::Locale;
use crate::omen::Omen;
use crate::script::Style;

const HEADER: &str = "# maho grimoire v2";

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
    /// ここからは頁に並ばない隠し項目。禁呪を唱えて初めて図鑑に現れる
    Forbidden,
    Doom(Doom),
    /// 暦の兆しがある日時に唱えて初めて現れる
    Omen(Omen),
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
            Item::Forbidden => l.pick("禁呪", "forbidden spells").into(),
            Item::Doom(d) => match l {
                Locale::Ja => format!("物語の呪文「{}」", d.name(l)),
                Locale::En => format!("story spell \"{}\"", d.name(l)),
            },
            Item::Omen(o) => match l {
                Locale::Ja => format!("暦「{}」", o.name(l)),
                Locale::En => format!("omen \"{}\"", o.name(l)),
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

/// 禁呪の印。唱えた言葉から決まるので、ハッシュとは別に覚える
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mark {
    None,
    Forbidden,
    Doom(Doom),
}

impl Mark {
    pub fn of(c: &MagicCircle, doom: Option<Doom>) -> Self {
        match doom {
            Some(d) => Mark::Doom(d),
            None if c.forbidden => Mark::Forbidden,
            None => Mark::None,
        }
    }

    fn key(self) -> Option<String> {
        match self {
            Mark::None => None,
            Mark::Forbidden => Some("forbidden".into()),
            Mark::Doom(d) => Some(format!("doom:{}", d.key())),
        }
    }

    /// 知らない印は、印が無いものとして読む
    fn from_key(key: &str) -> Self {
        match key.strip_prefix("doom:") {
            Some(d) => Doom::from_key(d).map_or(Mark::Forbidden, Mark::Doom),
            None if key == "forbidden" => Mark::Forbidden,
            None => Mark::None,
        }
    }

    fn hidden_items(self) -> Vec<Item> {
        match self {
            Mark::None => vec![],
            Mark::Forbidden => vec![Item::Forbidden],
            Mark::Doom(d) => vec![Item::Forbidden, Item::Doom(d)],
        }
    }
}

/// その魔法陣を見て埋まる項目。
/// 禁呪は引いた格にかかわらず超極大魔法として描かれるので、本来の格は見たことにしない。
fn items_of(c: &MagicCircle, mark: Mark) -> Vec<Item> {
    let mut items = vec![
        Item::Shape(c.shape),
        Item::Layout(c.layout),
        Item::Symmetry(c.symmetry),
        Item::Style(c.hand.style),
        Item::Band(c.band),
    ];
    if mark == Mark::None {
        items.push(Item::Tier(c.tier));
    }
    // 大星の陣では装飾が描かれないので、見たことにしない
    if c.layout != Layout::Grand {
        items.push(Item::Ornament(c.ornament));
    }
    items
}

#[derive(Debug, Clone, PartialEq)]
struct Entry {
    hash: [u8; 32],
    count: u64,
    mark: Mark,
}

/// 骨格。一目で形が違うと分かる組み合わせ（中心図形 × 陣形 × 格）。
pub const SKELETONS: usize = Shape::ALL.len() * Layout::ALL.len() * Tier::ALL.len();

#[derive(Debug, Default, PartialEq)]
pub struct Grimoire {
    /// 初めて唱えた順に、ハッシュと唱えた回数と禁呪の印
    entries: Vec<Entry>,
    /// 出会った暦の兆し
    omens: BTreeSet<Omen>,
}

impl Grimoire {
    /// 読めない行は飛ばす。ファイルが無ければ空の図鑑。
    pub fn parse(text: &str) -> Self {
        let omens = text
            .lines()
            .filter_map(|line| Omen::from_key(line.strip_prefix("omen ")?.trim()))
            .collect();
        let entries = text
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                Some(Entry {
                    hash: parse_hash(fields.next()?)?,
                    count: fields.next()?.parse().ok()?,
                    mark: fields.next().map_or(Mark::None, Mark::from_key),
                })
            })
            .collect();
        Grimoire { entries, omens }
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
        for e in &self.entries {
            let hex: String = e.hash.iter().map(|b| format!("{b:02x}")).collect();
            match e.mark.key() {
                Some(mark) => s.push_str(&format!("{hex} {} {mark}\n", e.count)),
                None => s.push_str(&format!("{hex} {}\n", e.count)),
            }
        }
        for o in &self.omens {
            s.push_str(&format!("omen {}\n", o.key()));
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
    /// `doom` は、物語の滅びの呪文を唱えたときにどれだったか。
    pub fn record(&mut self, c: &MagicCircle, doom: Option<Doom>) -> Vec<Item> {
        let mut new = self.record_circle(c, doom);
        if let Some(o) = c.omen
            && self.omens.insert(o)
        {
            new.push(Item::Omen(o));
        }
        new
    }

    fn record_circle(&mut self, c: &MagicCircle, doom: Option<Doom>) -> Vec<Item> {
        let mark = Mark::of(c, doom);
        let index = self.entries.iter().position(|e| e.hash == c.hash);
        // 前にも唱えた呪文なら、埋まる項目は無い。
        // ただし禁呪の印が無かった頃に記録したものなら、印だけ付け直す
        if let Some(i) = index
            && self.entries[i].mark >= mark
        {
            self.entries[i].count += 1;
            return Vec::new();
        }
        let before = self.found();
        match index {
            Some(i) => {
                self.entries[i].count += 1;
                self.entries[i].mark = mark;
            }
            None => self.entries.push(Entry {
                hash: c.hash,
                count: 1,
                mark,
            }),
        }
        let mut new: Vec<Item> = items_of(c, mark);
        new.extend(mark.hidden_items());
        new.retain(|i| !before.contains(i));
        new
    }

    fn circles(&self) -> impl Iterator<Item = (MagicCircle, &Entry)> + '_ {
        self.entries
            .iter()
            .map(|e| (MagicCircle::from_hash(e.hash), e))
    }

    /// 頁に並ぶ項目のうち、埋まったもの
    pub fn items(&self) -> HashSet<Item> {
        self.circles()
            .flat_map(|(c, e)| items_of(&c, e.mark))
            .collect()
    }

    /// 頁に並ぶ項目と隠し項目の両方
    fn found(&self) -> HashSet<Item> {
        let mut found = self.items();
        found.extend(self.entries.iter().flat_map(|e| e.mark.hidden_items()));
        found
    }

    /// 格ごとの、その格になった呪文の種類数
    pub fn spells_of(&self, tier: Tier) -> usize {
        self.circles()
            .filter(|(c, e)| e.mark == Mark::None && c.tier == tier)
            .count()
    }

    pub fn skeletons(&self) -> usize {
        self.circles()
            .filter(|(_, e)| e.mark == Mark::None)
            .map(|(c, _)| (c.shape, c.layout, c.tier))
            .collect::<HashSet<_>>()
            .len()
    }

    /// 呪文の種類数と、延べの回数
    pub fn casts(&self) -> (usize, u64) {
        (
            self.entries.len(),
            self.entries.iter().map(|e| e.count).sum(),
        )
    }

    /// 禁呪の種類数と延べの回数、見つけた物語の呪文
    fn forbidden(&self) -> (usize, u64, HashSet<Doom>) {
        let marked = || self.entries.iter().filter(|e| e.mark != Mark::None);
        let dooms = marked()
            .filter_map(|e| match e.mark {
                Mark::Doom(d) => Some(d),
                _ => None,
            })
            .collect();
        (marked().count(), marked().map(|e| e.count).sum(), dooms)
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
        Locale::En => format!(
            "  {} cast, {} in all\n\n",
            plural(spells as u64, "spell"),
            plural(casts, "time")
        ),
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
    // 禁呪の欄は、禁呪を 1 度でも唱えるまで出さない
    let (spells, casts, dooms) = g.forbidden();
    if spells > 0 {
        s.push_str(&match l {
            Locale::Ja => format!(
                "\n  {} {:>6}  延べ {casts} 回\n",
                pad("禁呪", 10),
                format!("{spells} 種")
            ),
            Locale::En => format!(
                "\n  {} {:>6}  {}, cast {}\n",
                pad("forbidden", 10),
                spells,
                if spells == 1 { "spell" } else { "spells" },
                plural(casts, "time")
            ),
        });
        let names: Vec<&str> = Doom::ALL
            .iter()
            .map(|d| {
                if dooms.contains(d) {
                    d.name(l)
                } else {
                    unknown
                }
            })
            .collect();
        s.push_str(&format!(
            "  {} {:>6}  {}\n",
            pad(l.pick("物語の呪文", "story"), 10),
            format!("{}/{}", dooms.len(), Doom::ALL.len()),
            names.join(l.pick("  ", ", "))
        ));
    }
    // 暦の欄も、兆しに 1 度でも出会うまで出さない
    if !g.omens.is_empty() {
        let names: Vec<&str> = Omen::ALL
            .iter()
            .map(|o| {
                if g.omens.contains(o) {
                    o.name(l)
                } else {
                    unknown
                }
            })
            .collect();
        s.push_str(&format!(
            "\n  {} {:>6}  {}\n",
            pad(l.pick("暦", "omens"), 10),
            format!("{}/{}", g.omens.len(), Omen::ALL.len()),
            names.join(l.pick("  ", ", "))
        ));
    }
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

/// 英語の「1 spell」「2 spells」
fn plural(n: u64, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
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
        let new = g.record(&c, None);
        assert!(new.contains(&Item::Tier(c.tier)));
        assert!(new.contains(&Item::Shape(c.shape)));
        // 2 回目は何も増えないが、回数は数える
        assert!(g.record(&c, None).is_empty());
        assert_eq!(g.casts(), (1, 2));
        assert_eq!(g.skeletons(), 1);
    }

    #[test]
    fn round_trip_keeps_no_command() {
        let mut g = Grimoire::default();
        g.record(&MagicCircle::from_command("secret --token abc"), None);
        g.record(&MagicCircle::from_command("ls"), None);
        let mut rm = MagicCircle::from_command("rm -rf secret");
        rm.forbid();
        g.record(&rm, None);
        let mut balse = MagicCircle::from_command("バルス");
        balse.forbid();
        g.record(&balse, Some(Doom::Balse));
        let text = g.to_text();
        assert!(!text.contains("secret") && !text.contains("abc"));
        assert!(text.contains(" 1 forbidden\n") && text.contains(" 1 doom:balse\n"));
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
        assert!(!items_of(&c, Mark::None).contains(&Item::Ornament(c.ornament)));
    }

    #[test]
    fn many_spells_fill_the_book() {
        // record を 3000 回呼ぶと毎回全体を数え直して遅いので、直接詰める
        let g = Grimoire {
            entries: (0..3000)
                .map(|i| Entry {
                    hash: MagicCircle::from_command(&format!("cmd {i}")).hash,
                    count: 1,
                    mark: Mark::None,
                })
                .collect(),
            omens: BTreeSet::new(),
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
        // 禁呪を唱えるまで、禁呪の欄は無い
        assert!(!page.contains("forbidden") && !page.contains("story"));
    }

    #[test]
    fn forbidden_spells_open_a_hidden_section() {
        let mut g = Grimoire::default();
        let mut rm = MagicCircle::from_command("rm -rf x");
        rm.forbid();
        let new = g.record(&rm, None);
        assert!(new.contains(&Item::Forbidden));
        // 超極大魔法として描かれたので、本来の格は埋めない
        assert!(!new.iter().any(|i| matches!(i, Item::Tier(_))));
        let page = show(&g, Locale::Ja);
        assert!(page.contains("禁呪") && page.contains("0/4"));
        assert!(!page.contains("バルス"));

        let mut balse = MagicCircle::from_command("バルス");
        balse.forbid();
        let new = g.record(&balse, Some(Doom::Balse));
        assert_eq!(new.last(), Some(&Item::Doom(Doom::Balse)));
        assert!(!new.contains(&Item::Forbidden));
        let page = show(&g, Locale::Ja);
        assert!(page.contains("1/4") && page.contains("バルス  ？？？"));
        assert!(page.contains("2 種"));
    }

    #[test]
    fn omens_open_a_hidden_section() {
        let mut g = Grimoire::default();
        let mut c = MagicCircle::from_command("git status");
        g.record(&c, None);
        assert!(!show(&g, Locale::Ja).contains("暦"));
        // 前に唱えた呪文でも、初めての兆しなら記録する
        c.bless(Omen::FullMoon);
        assert_eq!(g.record(&c, None), vec![Item::Omen(Omen::FullMoon)]);
        assert!(g.record(&c, None).is_empty());
        assert_eq!(g.casts(), (1, 3));
        let page = show(&g, Locale::Ja);
        assert!(
            page.contains("1/6") && page.contains("？？？  満月の夜"),
            "{page}"
        );
        // 兆しは別の行に残り、読み直しても消えない
        let text = g.to_text();
        assert!(text.ends_with("omen full-moon\n"));
        assert_eq!(Grimoire::parse(&text), g);
    }

    #[test]
    fn old_entries_get_their_mark() {
        // 印が無かった頃の図鑑（v1）もそのまま読め、禁呪を唱え直すと印が付く
        let c = MagicCircle::from_command("バルス");
        let hex = c.hash_hex();
        let mut g = Grimoire::parse(&format!("# maho grimoire v1\n{hex} 3\n"));
        let mut balse = c.clone();
        balse.forbid();
        let new = g.record(&balse, Some(Doom::Balse));
        assert!(new.contains(&Item::Doom(Doom::Balse)));
        assert_eq!(g.casts(), (1, 4));
        assert!(g.to_text().contains(&format!("{hex} 4 doom:balse")));
        // 知らない印は、印の無いものとして読む
        assert_eq!(
            Grimoire::parse(&format!("{hex} 2 sparkly\n")).entries[0].mark,
            Mark::None
        );
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
