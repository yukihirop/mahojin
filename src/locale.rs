//! 表示する言語。
//!
//! 決める順番: `--locale` > `MAHO_LOCALE` > 設定ファイル > `LC_ALL` / `LC_MESSAGES` / `LANG` > 英語。
//! 設定ファイルの読み書きは `config` にある。

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
}
