//! 禁呪。取り返しのつかないコマンドを唱えると、引いた格にかかわらず深紅の超極大魔法になる。
//!
//! 中心の魔法陣の形はいつもどおりハッシュから決まる。
//! 止めはしない。魔法陣は飾りで、唱えるかどうかは唱える人が決める。

use crate::locale::Locale;
use crate::shell::is_assignment;

/// この行に禁呪が含まれるか。`&&` `;` `|` でつないだ先のコマンドも見る。
/// 引用符は解釈しないので、`echo "rm -rf"` のような行も禁呪とみなす（見た目が物騒なのは同じ）。
pub fn is_forbidden(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if ["drop table", "drop database", "truncate table"]
        .iter()
        .any(|sql| lower.contains(sql))
    {
        return true;
    }
    // フォーク爆弾
    let packed: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    if packed.contains(":(){:|:&};:") {
        return true;
    }
    doom_of(line).is_some() || line.split(['|', '&', ';', '\n']).any(command)
}

/// 1 つのコマンドが禁呪か。
fn command(segment: &str) -> bool {
    let words: Vec<&str> = segment
        .split_whitespace()
        .skip_while(|w| {
            is_assignment(w)
                || matches!(*w, "sudo" | "doas" | "command" | "exec" | "nohup" | "time")
        })
        .collect();
    let Some((name, args)) = words.split_first() else {
        return false;
    };
    // `/bin/rm` も rm
    let name = name.rsplit('/').next().unwrap_or(name);
    match name {
        "rm" => flag(args, &['r', 'R'], "--recursive") && flag(args, &['f'], "--force"),
        "git" => git(args),
        "dd" => args.iter().any(|a| a.starts_with("of=/dev/")),
        "chmod" => flag(args, &['R'], "--recursive") && args.contains(&"777"),
        "terraform" => args.contains(&"destroy"),
        "kubectl" => args.contains(&"delete"),
        n => n == "mkfs" || n.starts_with("mkfs."),
    }
}

/// 物語の中の滅びの呪文。コマンドとしては存在しないが、それだけを唱えれば禁呪になる。
/// 図鑑では「物語の呪文」として集められる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Doom {
    /// 天空の城を崩した言葉。英語版では Balse
    Balse,
    /// 許されざる呪文
    AvadaKedavra,
    /// ジェダイを滅ぼす命令
    Order66,
    /// 唱え損ねると死者がよみがえる言葉
    Klaatu,
}

impl Doom {
    pub const ALL: [Doom; 4] = [Doom::Balse, Doom::AvadaKedavra, Doom::Order66, Doom::Klaatu];

    /// 唱え方。英語は語の間を空白 1 つで、日本語は詰めて書く
    /// （`アバダ ケダブラ` のように空けて唱えても当たる）。
    fn spellings(self) -> &'static [&'static str] {
        match self {
            Doom::Balse => &["バルス", "ばるす", "balse", "barusu"],
            Doom::AvadaKedavra => &["avada kedavra", "アバダケダブラ"],
            Doom::Order66 => &["execute order 66", "オーダー66"],
            Doom::Klaatu => &["klaatu barada nikto", "クラトゥバラダニクト"],
        }
    }

    pub fn name(self, l: Locale) -> &'static str {
        match self {
            Doom::Balse => l.pick("バルス", "Balse"),
            Doom::AvadaKedavra => l.pick("アバダ ケダブラ", "Avada Kedavra"),
            Doom::Order66 => l.pick("オーダー66", "Order 66"),
            Doom::Klaatu => l.pick("クラトゥ バラダ ニクト", "Klaatu barada nikto"),
        }
    }

    /// 図鑑のファイルに書く名前。言語によらない
    pub fn key(self) -> &'static str {
        match self {
            Doom::Balse => "balse",
            Doom::AvadaKedavra => "avada-kedavra",
            Doom::Order66 => "order-66",
            Doom::Klaatu => "klaatu",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.key() == key)
    }
}

/// この行が、物語の滅びの呪文だけを唱えたものか。
pub fn doom_of(line: &str) -> Option<Doom> {
    let spoken = line.trim().to_lowercase();
    let spoken = spoken.trim_end_matches(['!', '！']);
    let spaced = spoken.split_whitespace().collect::<Vec<_>>().join(" ");
    let packed: String = spoken.split_whitespace().collect();
    Doom::ALL
        .into_iter()
        .find(|d| d.spellings().iter().any(|w| *w == spaced || *w == packed))
}

fn git(args: &[&str]) -> bool {
    // `git -C dir push` のような、値を取る全体オプションを飛ばしてサブコマンドを探す
    let mut rest = args.iter();
    let sub = loop {
        match rest.next() {
            Some(&("-C" | "-c")) => {
                rest.next();
            }
            Some(a) if a.starts_with('-') => {}
            Some(a) => break *a,
            None => return false,
        }
    };
    let rest: Vec<&str> = rest.copied().collect();
    match sub {
        // --force-with-lease は安全に上書きするための道具なので、禁呪にしない
        "push" => flag(&rest, &['f'], "--force"),
        "reset" => rest.contains(&"--hard"),
        "clean" => flag(&rest, &['f'], "--force"),
        _ => false,
    }
}

/// `-rf` のようにまとめた短いオプションも、長いオプションも見る。
fn flag(args: &[&str], short: &[char], long: &str) -> bool {
    args.iter().any(|a| {
        *a == long
            || (a.starts_with('-')
                && !a.starts_with("--")
                && a[1..].chars().any(|c| short.contains(&c)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_forbidden_spells() {
        for line in [
            "rm -rf node_modules",
            "rm -fr /",
            "rm -r -f build",
            "rm --recursive --force x",
            "sudo rm -Rf /var/tmp/x",
            "/bin/rm -rf x",
            "cd build && rm -rf *",
            "git push --force",
            "git push -f origin main",
            "git -C repo push --force",
            "git reset --hard HEAD~3",
            "git clean -fdx",
            "dd if=img of=/dev/disk4",
            "mkfs.ext4 /dev/sdb1",
            "chmod -R 777 .",
            "terraform destroy",
            "kubectl delete pod web",
            "psql -c 'DROP TABLE users'",
            ":(){ :|:& };:",
            "FOO=1 rm -rf x",
            "バルス",
            "バルス！",
            "ばるす",
            "Balse!",
            "barusu",
            "Avada Kedavra!",
            "avada kedavra",
            "アバダ ケダブラ",
            "Execute Order 66",
            "オーダー 66",
            "Klaatu Barada Nikto!",
            "クラトゥ バラダ ニクト",
        ] {
            assert!(is_forbidden(line), "{line}");
        }
    }

    #[test]
    fn names_the_story_spell() {
        assert_eq!(doom_of("バルス！"), Some(Doom::Balse));
        assert_eq!(doom_of("Avada  Kedavra"), Some(Doom::AvadaKedavra));
        assert_eq!(doom_of("オーダー 66"), Some(Doom::Order66));
        assert_eq!(doom_of("rm -rf x"), None);
        for d in Doom::ALL {
            assert_eq!(Doom::from_key(d.key()), Some(d));
        }
    }

    #[test]
    fn leaves_ordinary_spells_alone() {
        for line in [
            "rm -r build",
            "rm -f lock",
            "rm file",
            "git push",
            "git push --force-with-lease",
            "git reset HEAD~1",
            "git reset --soft HEAD~1",
            "git clean -n",
            "git status",
            "git commit -m 'fix'",
            "dd if=/dev/zero of=out.img",
            "chmod 777 file",
            "chmod -R 755 .",
            "kubectl get pods",
            "terraform plan",
            "ls -rf",
            "echo バルス",
            "balsamic",
            "avada",
            "avada kedavra now",
            "execute order 65",
            "klaatu barada",
            "",
        ] {
            assert!(!is_forbidden(line), "{line}");
        }
    }
}
