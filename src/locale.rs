//! 表示する言語と、それを覚えておく設定ファイル。
//!
//! 決める順番: `--locale` > `MAHO_LOCALE` > 設定ファイル > `LC_ALL` / `LC_MESSAGES` / `LANG` > 英語。
//! 設定ファイルは `$XDG_CONFIG_HOME/maho/config`（無ければ `~/.config/maho/config`）で、
//! 中身は `locale = ja` の 1 行だけ。

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    Ja,
    En,
}

impl Locale {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ja" => Some(Locale::Ja),
            "en" => Some(Locale::En),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Locale::Ja => "ja",
            Locale::En => "en",
        }
    }

    /// 同じ文言の日本語と英語から、この言語のほうを選ぶ。
    pub fn pick<'a>(self, ja: &'a str, en: &'a str) -> &'a str {
        match self {
            Locale::Ja => ja,
            Locale::En => en,
        }
    }
}

/// 言語がどこから決まったか。`maho --setup` で見せる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Flag,
    Env,
    Config,
    System,
    Default,
}

/// `--locale` を除いた決め方。`--locale` は呼び出し側で先に見る。
pub fn detect(env: impl Fn(&str) -> Option<String>, config: Option<Locale>) -> (Locale, Source) {
    if let Some(l) = env("MAHO_LOCALE").as_deref().and_then(Locale::parse) {
        return (l, Source::Env);
    }
    if let Some(l) = config {
        return (l, Source::Config);
    }
    // POSIX と同じく、最初に値が入っている変数だけを見る
    let system = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|k| env(k).filter(|v| !v.is_empty()));
    match system {
        Some(v) if v.starts_with("ja") => (Locale::Ja, Source::System),
        Some(_) => (Locale::En, Source::System),
        None => (Locale::En, Source::Default),
    }
}

pub fn config_path(env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let base = match env("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env("HOME")?).join(".config"),
    };
    Some(base.join("maho").join("config"))
}

/// 設定ファイルの locale。ファイルが無い・読めない・値がおかしいときは `None`。
pub fn read_config(path: &Path) -> Option<Locale> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if key.trim() != "locale" {
            return None;
        }
        Locale::parse(value.trim().trim_matches('"'))
    })
}

pub fn write_config(path: &Path, locale: Locale) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, format!("locale = {}\n", locale.code()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| {
            vars.iter()
                .find(|(name, _)| *name == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn detection_order() {
        let ja = Some(Locale::Ja);
        assert_eq!(
            detect(env(&[("MAHO_LOCALE", "en"), ("LANG", "ja_JP.UTF-8")]), ja),
            (Locale::En, Source::Env)
        );
        assert_eq!(
            detect(env(&[("LANG", "en_US.UTF-8")]), ja),
            (Locale::Ja, Source::Config)
        );
        assert_eq!(
            detect(env(&[("LANG", "ja_JP.UTF-8")]), None),
            (Locale::Ja, Source::System)
        );
        assert_eq!(
            detect(env(&[("LC_ALL", "C"), ("LANG", "ja_JP.UTF-8")]), None),
            (Locale::En, Source::System)
        );
        assert_eq!(detect(env(&[]), None), (Locale::En, Source::Default));
        // 知らない値の MAHO_LOCALE は無視して次を見る
        assert_eq!(
            detect(env(&[("MAHO_LOCALE", "fr")]), ja),
            (Locale::Ja, Source::Config)
        );
    }

    #[test]
    fn config_path_follows_xdg() {
        assert_eq!(
            config_path(env(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")])),
            Some(PathBuf::from("/x/maho/config"))
        );
        assert_eq!(
            config_path(env(&[("HOME", "/h")])),
            Some(PathBuf::from("/h/.config/maho/config"))
        );
        assert_eq!(config_path(env(&[])), None);
    }

    #[test]
    fn config_round_trip() {
        let dir = std::env::temp_dir().join(format!("maho-test-{}", std::process::id()));
        let path = dir.join("maho").join("config");
        assert_eq!(read_config(&path), None);
        write_config(&path, Locale::En).unwrap();
        assert_eq!(read_config(&path), Some(Locale::En));
        write_config(&path, Locale::Ja).unwrap();
        assert_eq!(read_config(&path), Some(Locale::Ja));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
