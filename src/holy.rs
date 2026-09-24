//! 神聖魔法。このリポジトリに star を付ける呪文を唱えると、金と白の超極大魔法になり、
//! 呪文が通ったあとでお礼を言う。
//!
//! 中心の魔法陣の形はいつもどおりハッシュから決まる。

use crate::shell::is_assignment;

/// star を付ける先。`gh api` のパスとして書く
const STARRED: &str = "user/starred/yukihirop/mahojin";

/// この行に、このリポジトリへ star を付ける `gh api` が含まれるか。
/// `&&` `;` `|` でつないだ先のコマンドも見る。外す（DELETE）ほうは当たらない。
pub fn is_holy(line: &str) -> bool {
    line.split(['|', '&', ';', '\n']).any(command)
}

fn command(segment: &str) -> bool {
    let words: Vec<String> = segment
        .split_whitespace()
        .skip_while(|w| is_assignment(w) || matches!(*w, "command" | "exec" | "time"))
        .map(|w| w.trim_matches(['\'', '"']).to_string())
        .collect();
    let Some((name, args)) = words.split_first() else {
        return false;
    };
    if name.rsplit('/').next() != Some("gh") || args.first().map(String::as_str) != Some("api") {
        return false;
    }
    let args = &args[1..];
    let starred = args
        .iter()
        .any(|a| a.trim_start_matches('/').trim_end_matches('/') == STARRED);
    starred && method(args).is_some_and(|m| m.eq_ignore_ascii_case("put"))
}

/// `-X PUT` / `-XPUT` / `--method PUT` / `--method=PUT` の値
fn method(args: &[String]) -> Option<&str> {
    let mut rest = args.iter();
    while let Some(a) = rest.next() {
        match a.as_str() {
            "-X" | "--method" => return rest.next().map(String::as_str),
            a if a.starts_with("--method=") => return Some(&a["--method=".len()..]),
            a if a.starts_with("-X") => return Some(&a[2..]),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_star() {
        for line in [
            "gh api -X PUT /user/starred/yukihirop/mahojin",
            "gh api -X PUT user/starred/yukihirop/mahojin",
            "gh api -XPUT /user/starred/yukihirop/mahojin",
            "gh api --method PUT /user/starred/yukihirop/mahojin",
            "gh api --method=put /user/starred/yukihirop/mahojin",
            "gh api /user/starred/yukihirop/mahojin -X put",
            "gh api -X PUT '/user/starred/yukihirop/mahojin'",
            "GH_HOST=github.com gh api -X PUT /user/starred/yukihirop/mahojin",
            "/opt/homebrew/bin/gh api -X PUT /user/starred/yukihirop/mahojin",
            "gh auth status && gh api -X PUT /user/starred/yukihirop/mahojin",
        ] {
            assert!(is_holy(line), "{line}");
        }
    }

    #[test]
    fn leaves_other_calls_alone() {
        for line in [
            "gh api /user/starred/yukihirop/mahojin",
            "gh api -X DELETE /user/starred/yukihirop/mahojin",
            "gh api -X PUT /user/starred/yukihirop/other",
            "gh api -X PUT /user/starred/someone/mahojin",
            "gh repo view yukihirop/mahojin",
            "echo gh api -X PUT /user/starred/yukihirop/mahojin",
            "gh api -X PUT",
            "",
        ] {
            assert!(!is_holy(line), "{line}");
        }
    }
}
