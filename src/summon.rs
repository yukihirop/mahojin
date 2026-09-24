//! 召喚魔法。作者の名前だけを唱えると、コマンドが見つからなくても砕けず、作者が現れる。
//!
//! 中心の魔法陣の形はいつもどおりハッシュから決まる。

/// 呼び出せる名前。`@` を付けても、`!` で締めても当たる
const NAME: &str = "yukihirop";

/// この行が、作者の名前だけを唱えたものか。
pub fn is_summon(line: &str) -> bool {
    let spoken = line.trim().to_lowercase();
    let spoken = spoken.trim_start_matches('@').trim_end_matches(['!', '！']);
    spoken == NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_to_the_name() {
        for line in [
            "yukihirop",
            "YukiHirop",
            " @yukihirop ",
            "yukihirop!",
            "yukihirop！",
        ] {
            assert!(is_summon(line), "{line}");
        }
        for line in [
            "yukihirop --help",
            "echo yukihirop",
            "yukihiro",
            "cd yukihirop",
            "",
        ] {
            assert!(!is_summon(line), "{line}");
        }
    }
}
