//! `--share`: X などに貼る投稿文と、添える画像の名前を作る。

use std::fmt::Write;

use crate::circle::MagicCircle;
use crate::locale::Locale;

const REPO: &str = "https://github.com/yukihirop/mahojin";
/// 投稿文に載せる呪文の長さの上限（文字数）。X は全角を 2 と数えて 280 まで。
const MAX_SPELL: usize = 60;

/// 投稿文。コマンドと、その魔法陣の中身を短く並べる。
pub fn post(c: &MagicCircle, spell: &str, l: Locale) -> String {
    let (layout, shape, ornament) = (c.layout.name(l), c.shape.name(l), c.ornament.name(l));
    let (spell, sigil) = (shorten(spell), &c.hash_hex()[..8]);
    let tier = c.starred_title(l);
    let when = c.omen.map(|o| o.when(l));
    let body = match l {
        Locale::Ja => format!(
            "「{spell}」を{}唱えたら、{tier}の魔法陣が展開した ✦\n\n\
             {layout}の陣 / {shape} / {ornament} / {} 回対称\n\
             呪紋 {sigil}",
            when.unwrap_or_default(),
            c.symmetry
        ),
        Locale::En => format!(
            "I cast \"{spell}\"{} and {} {tier} circle unfolded ✦\n\n\
             {layout} layout / {shape} / {ornament} / {}-fold symmetry\n\
             Sigil {sigil}",
            when.map(|w| format!(" {w}")).unwrap_or_default(),
            if tier.starts_with(['a', 'e', 'i', 'o', 'u']) {
                "an"
            } else {
                "a"
            },
            c.symmetry
        ),
    };
    format!("{body}\n\n#mahojin\n{REPO}")
}

/// 投稿文が入った状態の X の投稿画面の URL。画像は添えられないので手で付ける。
pub fn intent_url(text: &str) -> String {
    format!("https://x.com/intent/post?text={}", percent_encode(text))
}

/// 添える画像のファイル名。同じ呪文なら同じ名前になる。
pub fn image_name(c: &MagicCircle) -> String {
    format!("mahojin-{}.png", &c.hash_hex()[..8])
}

fn shorten(spell: &str) -> String {
    if spell.chars().count() <= MAX_SPELL {
        return spell.to_string();
    }
    let head: String = spell.chars().take(MAX_SPELL - 1).collect();
    format!("{head}…")
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => write!(out, "%{b:02X}").unwrap(),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_names_the_spell_and_circle() {
        let c = MagicCircle::from_command("git status");
        let p = post(&c, "git status", Locale::Ja);
        assert!(p.starts_with(&format!(
            "「git status」を唱えたら、{}の魔法陣が",
            c.tier.starred(Locale::Ja)
        )));
        assert!(p.contains("突破の陣 / 車輪 / 星形 / 3 回対称"));
        assert!(p.contains("呪紋 e62b04aa"));
        assert!(p.ends_with(REPO));
        assert_eq!(image_name(&c), "mahojin-e62b04aa.png");

        let p = post(&c, "git status", Locale::En);
        assert!(p.starts_with("I cast \"git status\" and "));
        assert!(p.contains(&c.tier.starred(Locale::En)));
        // 母音で始まる格には an を付ける
        let ultimate = MagicCircle {
            tier: crate::circle::Tier::Ultimate,
            ..c.clone()
        };
        assert!(
            post(&ultimate, "git status", Locale::En).starts_with(
                "I cast \"git status\" and an ultimate spell (★★★★☆☆) circle unfolded"
            )
        );
        assert!(p.contains("breach layout / wheel / star / 3-fold symmetry"));
        assert!(p.contains("Sigil e62b04aa"));
    }

    #[test]
    fn post_names_the_omen() {
        let mut c = MagicCircle::from_command("git status");
        c.bless(crate::omen::Omen::FullMoon);
        assert!(
            post(&c, "git status", Locale::Ja).starts_with("「git status」を満月の夜に唱えたら、")
        );
        assert!(
            post(&c, "git status", Locale::En)
                .starts_with("I cast \"git status\" on a full moon night and a major spell")
        );
    }

    #[test]
    fn long_spells_are_shortened() {
        let long = "x".repeat(200);
        let s = shorten(&long);
        assert_eq!(s.chars().count(), MAX_SPELL);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn intent_url_is_encoded() {
        assert_eq!(
            intent_url("a b&#「"),
            "https://x.com/intent/post?text=a%20b%26%23%E3%80%8C"
        );
    }
}
