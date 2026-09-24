//! 設定ファイル。`$XDG_CONFIG_HOME/mahojin/config.toml`（無ければ `~/.config/mahojin/config.toml`）。
//!
//! ```toml
//! locale = "ja"
//! chant_skip = ["cd", "ls"]
//! shatter = false
//! ```
//!
//! 書き換えるのは `mahojin --setup --locale` の locale だけで、ほかの設定やコメントは残す。

use std::path::{Path, PathBuf};

use toml_edit::DocumentMut;

use crate::locale::Locale;

#[derive(Debug, Default, PartialEq)]
pub struct Config {
    pub locale: Option<Locale>,
    /// 詠唱モードで魔法陣を出さないコマンド。無ければ既定を使う
    pub chant_skip: Option<Vec<String>>,
    /// コマンドが失敗したとき魔法陣を砕くか。無ければ砕く
    pub shatter: Option<bool>,
}

pub fn path(env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let base = match env("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env("HOME")?).join(".config"),
    };
    Some(base.join("mahojin").join("config.toml"))
}

impl Config {
    /// 知らないキーや型の合わない値は無視する。TOML として読めないときだけ誤りにする。
    pub fn parse(text: &str) -> Result<Self, String> {
        let doc: DocumentMut = text
            .parse()
            .map_err(|e: toml_edit::TomlError| e.to_string())?;
        let locale = doc
            .get("locale")
            .and_then(|v| v.as_str())
            .and_then(Locale::parse);
        let chant_skip = doc.get("chant_skip").and_then(|v| v.as_array()).map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });
        let shatter = doc.get("shatter").and_then(|v| v.as_bool());
        Ok(Config {
            locale,
            chant_skip,
            shatter,
        })
    }

    /// ファイルが無ければ既定の設定。
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// locale だけを書き換える。TOML として読めないファイルは、壊さないよう書き換えずに誤りを返す。
pub fn save_locale(path: &Path, locale: Locale) -> Result<(), String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.to_string()),
    };
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e: toml_edit::TomlError| e.to_string())?;
    doc["locale"] = toml_edit::value(locale.code());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, doc.to_string()).map_err(|e| e.to_string())
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
    fn path_follows_xdg() {
        assert_eq!(
            path(env(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")])),
            Some(PathBuf::from("/x/mahojin/config.toml"))
        );
        assert_eq!(
            path(env(&[("HOME", "/h")])),
            Some(PathBuf::from("/h/.config/mahojin/config.toml"))
        );
        assert_eq!(path(env(&[])), None);
    }

    #[test]
    fn parses_known_keys() {
        let c =
            Config::parse("locale = \"en\"\nchant_skip = [\"ls\", \"git\"]\nfuture = 1\n").unwrap();
        assert_eq!(c.locale, Some(Locale::En));
        assert_eq!(c.chant_skip, Some(vec!["ls".into(), "git".into()]));
        // 型の合わない値や知らない言語は、無いものとして扱う
        let c = Config::parse("locale = \"fr\"\nchant_skip = \"ls\"\n").unwrap();
        assert_eq!(c, Config::default());
        assert_eq!(Config::parse("").unwrap(), Config::default());
        assert_eq!(
            Config::parse("shatter = false").unwrap().shatter,
            Some(false)
        );
        assert_eq!(Config::parse("shatter = \"no\"").unwrap().shatter, None);
        // 空のリストは「すべてに出す」なので、無いのとは区別する
        assert_eq!(
            Config::parse("chant_skip = []").unwrap().chant_skip,
            Some(vec![])
        );
        // 前の形式（引用符なし）は TOML として読めない
        assert!(Config::parse("locale = ja").is_err());
    }

    #[test]
    fn saving_the_locale_keeps_the_rest() {
        let dir = std::env::temp_dir().join(format!("mahojin-config-{}", std::process::id()));
        let path = dir.join("mahojin").join("config.toml");
        assert_eq!(Config::load(&path).unwrap(), Config::default());
        save_locale(&path, Locale::En).unwrap();
        assert_eq!(Config::load(&path).unwrap().locale, Some(Locale::En));

        let mine = "# my settings\nchant_skip = [\"ls\"]  # quiet ones\nlocale = \"en\"\n";
        std::fs::write(&path, mine).unwrap();
        save_locale(&path, Locale::Ja).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, mine.replace("\"en\"", "\"ja\""));

        // 壊れたファイルは上書きしない
        std::fs::write(&path, "locale = ja\n").unwrap();
        assert!(save_locale(&path, Locale::En).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "locale = ja\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
