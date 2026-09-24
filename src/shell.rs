//! `maho init zsh`: `maho on` で、打ったコマンドすべてに魔法陣を出す詠唱モード。
//!
//! 子プロセスの maho は親のシェルを変えられないので、シェル側に関数とフックを読み込んでもらう。
//! コマンドの前に `maho` を足すのではなく、実行直前（preexec）に魔法陣だけ描いて、
//! 実行はシェルに任せる。`cd` もエイリアスもパイプもそのまま動く。

use crate::locale::Locale;

/// `eval "$(maho init zsh)"` で読み込ませるスクリプト。
/// on / off の知らせはシェルを開いたときの言語で埋め込む。
pub fn zsh(l: Locale) -> String {
    let on = l.pick(
        "✦ 詠唱モード: 打ったコマンドすべてに魔法陣が出ます（maho off で戻る）",
        "✦ Chanting: every command you type unfolds a circle (maho off to stop)",
    );
    let off = l.pick("✦ 詠唱モードを解きました", "✦ Chanting stopped");
    ZSH.replace("{on}", on).replace("{off}", off)
}

const ZSH: &str = r#"# maho: eval "$(maho init zsh)"
maho() {
  if (( $# == 1 )) && [[ $1 == on ]]; then
    typeset -g _MAHO_CHANTING=1
    print -u2 -r -- '{on}'
  elif (( $# == 1 )) && [[ $1 == off ]]; then
    unset _MAHO_CHANTING
    print -u2 -r -- '{off}'
  else
    command maho "$@"
  fi
}

_maho_preexec() {
  [[ -n $_MAHO_CHANTING ]] || return 0
  # maho を自分で唱えた行には重ねない
  local -a words=(${(z)1})
  [[ $words[1] == maho || $words[1] == command && $words[2] == maho ]] && return 0
  command maho --no-run -- "$1"
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec _maho_preexec
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_is_localized_and_hooks_preexec() {
        let ja = zsh(Locale::Ja);
        assert!(ja.contains("add-zsh-hook preexec _maho_preexec"));
        assert!(ja.contains("詠唱モード"));
        assert!(!ja.contains("{on}") && !ja.contains("{off}"));
        assert!(zsh(Locale::En).contains("Chanting stopped"));
    }

    #[test]
    fn script_parses_in_zsh() {
        // zsh が無い環境（CI の一部など）では飛ばす
        let Ok(mut child) = std::process::Command::new("zsh")
            .arg("-n")
            .stdin(std::process::Stdio::piped())
            .spawn()
        else {
            return;
        };
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(zsh(Locale::Ja).as_bytes())
            .unwrap();
        assert!(child.wait().unwrap().success());
    }
}
