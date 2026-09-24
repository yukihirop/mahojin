//! `mahojin --skills`: `/mahojin-setup` と `/mahojin-teardown` のスキルを `~/.agents/skills/` に置く。
//!
//! スキル（`skills/<ja|en>/mahojin-{setup,teardown}/SKILL.md`）はバイナリに埋め込んである。
//! Claude Code と Codex が自分のディレクトリしか見ないときのために、`~/.claude/skills/` と
//! `~/.codex/skills/` があれば、そこからリンクも張る。

use std::path::{Path, PathBuf};

use crate::locale::Locale;

pub const NAMES: [&str; 2] = ["mahojin-setup", "mahojin-teardown"];

fn body(name: &str, l: Locale) -> &'static str {
    match (name, l) {
        ("mahojin-setup", Locale::Ja) => include_str!("../skills/ja/mahojin-setup/SKILL.md"),
        ("mahojin-setup", Locale::En) => include_str!("../skills/en/mahojin-setup/SKILL.md"),
        (_, Locale::Ja) => include_str!("../skills/ja/mahojin-teardown/SKILL.md"),
        (_, Locale::En) => include_str!("../skills/en/mahojin-teardown/SKILL.md"),
    }
}

/// 中身が違うファイルだけ書き直す（言語を変えれば書き直される）。書いたパスと張ったリンクを返す。
/// リンクは、ツールのディレクトリがあって、同じ名前のものがまだ無いときだけ張る。
pub fn install(home: &Path, l: Locale) -> std::io::Result<Vec<String>> {
    let root = home.join(".agents").join("skills");
    let mut changed = Vec::new();
    for name in NAMES {
        let path = root.join(name).join("SKILL.md");
        let text = body(name, l);
        if std::fs::read_to_string(&path).is_ok_and(|cur| cur == text) {
            continue;
        }
        std::fs::create_dir_all(root.join(name))?;
        std::fs::write(&path, text)?;
        changed.push(path.display().to_string());
    }
    for tool in [".claude", ".codex"] {
        let skills = home.join(tool).join("skills");
        for name in NAMES {
            let (target, link): (PathBuf, PathBuf) = (root.join(name), skills.join(name));
            if skills.is_dir()
                && std::fs::symlink_metadata(&link).is_err()
                && std::os::unix::fs::symlink(&target, &link).is_ok()
            {
                changed.push(format!("{} -> {}", link.display(), target.display()));
            }
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_skills_and_links_once() {
        let home = std::env::temp_dir().join(format!("mahojin-skill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".claude/skills")).unwrap();

        let first = install(&home, Locale::Ja).unwrap();
        assert_eq!(first.len(), 4, "{first:?}");
        let setup = home.join(".claude/skills/mahojin-setup/SKILL.md");
        assert!(
            std::fs::read_to_string(setup)
                .unwrap()
                .contains("name: mahojin-setup")
        );
        // .codex が無ければリンクは張らない
        assert!(!home.join(".codex").exists());

        assert!(install(&home, Locale::Ja).unwrap().is_empty());
        // 言語を変えると本体だけ書き直す
        assert_eq!(install(&home, Locale::En).unwrap().len(), 2);
        std::fs::remove_dir_all(&home).unwrap();
    }
}
