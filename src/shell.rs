//! `maho init <shell>`: `maho on` で、打ったコマンドすべてに魔法陣を出す詠唱モード。
//!
//! 子プロセスの maho は親のシェルを変えられないので、シェル側に関数とフックを読み込んでもらう。
//! コマンドの前に `maho` を足すのではなく、実行直前（preexec）に魔法陣だけ描いて、
//! 実行はシェルに任せる。`cd` もエイリアスもパイプもそのまま動く。

use crate::locale::Locale;

pub const SHELLS: [&str; 3] = ["zsh", "bash", "fish"];

/// シェルに読み込ませるスクリプト。知らないシェルなら `None`。
/// on / off の知らせはシェルを開いたときの言語で埋め込む。
pub fn init(shell: &str, l: Locale) -> Option<String> {
    let script = match shell {
        "zsh" => ZSH,
        "bash" => BASH,
        "fish" => FISH,
        _ => return None,
    };
    let on = l.pick(
        "✦ 詠唱モード: 打ったコマンドすべてに魔法陣が出ます（maho off で戻る）",
        "✦ Chanting: every command you type unfolds a circle (maho off to stop)",
    );
    let off = l.pick("✦ 詠唱モードを解きました", "✦ Chanting stopped");
    let taken = l.pick(
        "maho: DEBUG トラップが使用中なので詠唱モードを入れられません。bash-preexec を先に読み込むと使えます",
        "maho: the DEBUG trap is already in use, so chanting mode can't hook in; load bash-preexec first",
    );
    // 知らせは一重引用符の中に埋め込むので、中の ' を逃がす
    let quote = |msg: &str| match shell {
        "fish" => msg.replace('\\', "\\\\").replace('\'', "\\'"),
        _ => msg.replace('\'', "'\\''"),
    };
    Some(
        script
            .replace("{on}", &quote(on))
            .replace("{off}", &quote(off))
            .replace("{taken}", &quote(taken)),
    )
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

/// bash には preexec が無い。bash-preexec があればそれに乗り、無ければ DEBUG トラップで作る。
/// DEBUG トラップはパイプの各コマンドやプロンプトのフックでも発火するので、
/// 「プロンプトを出してから最初の、プロンプトのフックでないコマンド」でだけ動かす。
const BASH: &str = r#"# maho: eval "$(maho init bash)"
maho() {
  if [[ $# -eq 1 && $1 == on ]]; then
    _MAHO_CHANTING=1
    printf '%s\n' '{on}' >&2
  elif [[ $# -eq 1 && $1 == off ]]; then
    unset _MAHO_CHANTING
    printf '%s\n' '{off}' >&2
  else
    command maho "$@"
  fi
}

_maho_cast() {
  [[ -n $_MAHO_CHANTING ]] || return 0
  local -a words
  IFS=$' \t\n' read -r -a words <<< "$1"
  [[ ${words[0]} == maho || ( ${words[0]} == command && ${words[1]} == maho ) ]] && return 0
  command maho --no-run -- "$1"
}

if [[ -n ${bash_preexec_imported:-} || -n ${__bp_imported:-} ]]; then
  preexec_functions+=(_maho_cast)
elif [[ -n $(trap -p DEBUG) ]]; then
  printf '%s\n' '{taken}' >&2
else
  _maho_at_prompt=1
  _maho_prompt() { _maho_at_prompt=1; }

  _maho_debug() {
    [[ -n $_maho_at_prompt && -z ${COMP_LINE:-} ]] || return 0
    # プロンプトのフック自身（何も打たずに Enter したときもここに来る）
    local hook part
    local -a parts
    for hook in "${PROMPT_COMMAND[@]}"; do
      IFS=';' read -r -a parts <<< "${hook//$'\n'/;}"
      for part in "${parts[@]}"; do
        part=${part#"${part%%[![:space:]]*}"}
        part=${part%"${part##*[![:space:]]}"}
        [[ $BASH_COMMAND == "$part" ]] && return 0
      done
    done
    _maho_at_prompt=
    local entry line=$BASH_COMMAND
    entry=$(HISTTIMEFORMAT= builtin history 1 2>/dev/null)
    [[ $entry =~ ^[[:space:]]*[0-9]+[*]?[[:space:]]+(.*)$ ]] && line=${BASH_REMATCH[1]}
    _maho_cast "$line"
  }

  PROMPT_COMMAND="${PROMPT_COMMAND:+$PROMPT_COMMAND; }_maho_prompt"
  trap '_maho_debug' DEBUG
fi
"#;

const FISH: &str = r#"# maho: maho init fish | source
function maho
    if test (count $argv) -eq 1; and test "$argv[1]" = on
        set -g _maho_chanting 1
        echo '{on}' >&2
    else if test (count $argv) -eq 1; and test "$argv[1]" = off
        set -e _maho_chanting
        echo '{off}' >&2
    else
        command maho $argv
    end
end

function _maho_preexec --on-event fish_preexec
    set -q _maho_chanting; or return 0
    set -l words (string split -n ' ' -- $argv[1])
    if test "$words[1]" = maho
        return 0
    end
    if test "$words[1]" = command; and test "$words[2]" = maho
        return 0
    end
    command maho --no-run -- $argv[1]
end
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_are_localized() {
        for shell in SHELLS {
            let ja = init(shell, Locale::Ja).unwrap();
            assert!(ja.contains("詠唱モード"), "{shell}");
            assert!(!ja.contains("{on}") && !ja.contains("{off}") && !ja.contains("{taken}"));
            assert!(
                init(shell, Locale::En)
                    .unwrap()
                    .contains("Chanting stopped")
            );
        }
        assert!(init("tcsh", Locale::Ja).is_none());
        assert!(
            init("zsh", Locale::Ja)
                .unwrap()
                .contains("add-zsh-hook preexec")
        );
        assert!(init("fish", Locale::Ja).unwrap().contains("fish_preexec"));
    }

    /// そのシェルが入っていれば、構文だけ確かめる（CI には zsh や fish が無いことがある）
    fn parses_in(shell: &str, check: &[&str]) {
        use std::io::Write;
        let Ok(mut child) = std::process::Command::new(shell)
            .args(check)
            .stdin(std::process::Stdio::piped())
            .spawn()
        else {
            return;
        };
        // 英語の知らせには ' が入る（can't）ので、両方の言語で確かめる
        let script = init(shell, Locale::Ja).unwrap() + &init(shell, Locale::En).unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        assert!(child.wait().unwrap().success(), "{shell}");
    }

    #[test]
    fn scripts_parse() {
        parses_in("zsh", &["-n"]);
        parses_in("bash", &["-n"]);
        parses_in("fish", &["--no-execute"]);
    }
}
