//! `mahojin init <shell>`: `mahojin on` で、打ったコマンドすべてに魔法陣を出す詠唱モード。
//!
//! 子プロセスの mahojin は親のシェルを変えられないので、シェル側に関数とフックを読み込んでもらう。
//! コマンドの前に `mahojin` を足すのではなく、実行直前（preexec）に魔法陣だけ描いて、
//! 実行はシェルに任せる。`cd` もエイリアスもパイプもそのまま動く。

use crate::locale::Locale;

pub const SHELLS: [&str; 3] = ["zsh", "bash", "fish"];

/// 詠唱モードで魔法陣を出さないコマンドの既定。設定ファイルの `chant_skip` で置き換えられる。
pub const DEFAULT_SKIP: [&str; 7] = ["cd", "ls", "ll", "la", "pwd", "exit", "history"];

/// 設定ファイルの `chant_skip`（無ければ既定）。`chant_skip = []` なら、すべてのコマンドに魔法陣を出す。
pub fn skip_list(config: Option<&[String]>) -> Vec<String> {
    match config {
        Some(list) => list.to_vec(),
        None => DEFAULT_SKIP.map(String::from).to_vec(),
    }
}

/// 詠唱モードで、この行には魔法陣を出さないか。
/// 1 つだけのコマンド（`|` `;` `&` を含まない）で、先頭の語が除外リストにあるときだけ飛ばす。
/// `cd src && make` のように続きがある行は、続きのほうを唱えているので出す。
pub fn skipped(line: &str, skip: &[String]) -> bool {
    if line.contains(['|', ';', '&']) {
        return false;
    }
    // `FOO=1 ls` の環境変数の代入は飛ばして、コマンド名を見る
    let name = line.split_whitespace().find(|w| !is_assignment(w));
    name.is_some_and(|name| skip.iter().any(|s| s == name))
}

/// `FOO=1` のような環境変数の代入か
pub fn is_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && !name.starts_with(|c: char| c.is_ascii_digit())
            && name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
    })
}

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
        "✦ 詠唱モード: 打ったコマンドに魔法陣が出ます（mahojin off で戻る）",
        "✦ Chanting: the commands you type unfold circles (mahojin off to stop)",
    );
    let off = l.pick("✦ 詠唱モードを解きました", "✦ Chanting stopped");
    let taken = l.pick(
        "mahojin: DEBUG トラップが使用中なので詠唱モードを入れられません。bash-preexec を先に読み込むと使えます",
        "mahojin: the DEBUG trap is already in use, so chanting mode can't hook in; load bash-preexec first",
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

const ZSH: &str = r#"# mahojin: eval "$(mahojin init zsh)"
mahojin() {
  if (( $# == 1 )) && [[ $1 == on ]]; then
    typeset -g _MAHOJIN_CHANTING=1
    print -u2 -r -- '{on}'
  elif (( $# == 1 )) && [[ $1 == off ]]; then
    unset _MAHOJIN_CHANTING
    print -u2 -r -- '{off}'
  else
    command mahojin "$@"
  fi
}

_mahojin_preexec() {
  [[ -n $_MAHOJIN_CHANTING ]] || return 0
  # mahojin を自分で唱えた行には重ねない
  local -a words=(${(z)1})
  # `alias m=mahojin` のようなエイリアスも、たどって mahojin かどうかを見る
  local first=$words[1]
  if [[ -n ${aliases[$first]} ]]; then
    local -a expanded=(${(z)aliases[$first]})
    first=$expanded[1]
  fi
  [[ $first == mahojin || $words[1] == command && $words[2] == mahojin ]] && return 0
  typeset -g _mahojin_last=$1
  command mahojin --chant -- "$1"
}

# 唱えた呪文が失敗したら、魔法陣が砕ける。star を付ける呪文だけは、通ったときも呼ぶ
_mahojin_precmd() {
  local st=$? line=$_mahojin_last
  _mahojin_last=
  [[ -n $line && -n $_MAHOJIN_CHANTING ]] || return $st
  [[ $st -ne 0 || $line == *starred/yukihirop/mahojin* ]] && command mahojin --shatter $st -- "$line"
  return $st
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec _mahojin_preexec
add-zsh-hook precmd _mahojin_precmd
"#;

/// bash には preexec が無い。bash-preexec があればそれに乗り、無ければ DEBUG トラップで作る。
/// DEBUG トラップはパイプの各コマンドやプロンプトのフックでも発火するので、
/// 「プロンプトを出してから最初の、プロンプトのフックでないコマンド」でだけ動かす。
const BASH: &str = r#"# mahojin: eval "$(mahojin init bash)"
mahojin() {
  if [[ $# -eq 1 && $1 == on ]]; then
    _MAHOJIN_CHANTING=1
    printf '%s\n' '{on}' >&2
  elif [[ $# -eq 1 && $1 == off ]]; then
    unset _MAHOJIN_CHANTING
    printf '%s\n' '{off}' >&2
  else
    command mahojin "$@"
  fi
}

_mahojin_cast() {
  [[ -n $_MAHOJIN_CHANTING ]] || return 0
  local -a words
  IFS=$' \t\n' read -r -a words <<< "$1"
  # `alias m=mahojin` のようなエイリアスも、たどって mahojin かどうかを見る（alias は m='mahojin' の形で出る）
  local first=${words[0]} a
  if a=$(builtin alias -- "$first" 2>/dev/null); then
    a=${a#*=}
    a=${a#\'}
    first=${a%%[ \']*}
  fi
  [[ $first == mahojin || ( ${words[0]} == command && ${words[1]} == mahojin ) ]] && return 0
  _mahojin_last=$1
  command mahojin --chant -- "$1"
}

# 唱えた呪文が失敗したら、魔法陣が砕ける。star を付ける呪文だけは、通ったときも呼ぶ。$1 は呪文の終了コード
_mahojin_after() {
  local line=$_mahojin_last
  _mahojin_last=
  [[ -n $line && -n $_MAHOJIN_CHANTING ]] || return 0
  [[ $1 -ne 0 || $line == *starred/yukihirop/mahojin* ]] && command mahojin --shatter "$1" -- "$line"
  return 0
}

if [[ -n ${bash_preexec_imported:-} || -n ${__bp_imported:-} ]]; then
  preexec_functions+=(_mahojin_cast)
  _mahojin_precmd() { _mahojin_after $?; }
  precmd_functions+=(_mahojin_precmd)
elif [[ -n $(trap -p DEBUG) ]]; then
  printf '%s\n' '{taken}' >&2
else
  _mahojin_at_prompt=1
  # 呪文の終了コードは、ほかのプロンプトのフックに上書きされる前に取っておく。$? はそのまま返す
  _mahojin_status() { _mahojin_st=$?; return $_mahojin_st; }
  _mahojin_prompt() { _mahojin_after "$_mahojin_st"; _mahojin_at_prompt=1; return $_mahojin_st; }

  _mahojin_debug() {
    [[ -n $_mahojin_at_prompt && -z ${COMP_LINE:-} ]] || return 0
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
    _mahojin_at_prompt=
    local entry line=$BASH_COMMAND
    entry=$(HISTTIMEFORMAT= builtin history 1 2>/dev/null)
    [[ $entry =~ ^[[:space:]]*[0-9]+[*]?[[:space:]]+(.*)$ ]] && line=${BASH_REMATCH[1]}
    _mahojin_cast "$line"
  }

  PROMPT_COMMAND="_mahojin_status; ${PROMPT_COMMAND:+$PROMPT_COMMAND; }_mahojin_prompt"
  trap '_mahojin_debug' DEBUG
fi
"#;

const FISH: &str = r#"# mahojin: mahojin init fish | source
function mahojin
    if test (count $argv) -eq 1; and test "$argv[1]" = on
        set -g _mahojin_chanting 1
        echo '{on}' >&2
    else if test (count $argv) -eq 1; and test "$argv[1]" = off
        set -e _mahojin_chanting
        echo '{off}' >&2
    else
        command mahojin $argv
    end
end

function _mahojin_preexec --on-event fish_preexec
    set -q _mahojin_chanting; or return 0
    set -l words (string split -n ' ' -- $argv[1])
    # `alias m mahojin` で作った関数も、たどって mahojin かどうかを見る
    if functions -q -- "$words[1]"; and functions -- "$words[1]" | string match -q -r -- "--wraps[= ]'?mahojin\b"
        return 0
    end
    if test "$words[1]" = mahojin
        return 0
    end
    if test "$words[1]" = command; and test "$words[2]" = mahojin
        return 0
    end
    set -g _mahojin_last $argv[1]
    command mahojin --chant -- $argv[1]
end

# 唱えた呪文が失敗したら、魔法陣が砕ける。star を付ける呪文だけは、通ったときも呼ぶ
function _mahojin_postexec --on-event fish_postexec
    set -l st $status
    set -q _mahojin_last; or return 0
    set -l line $_mahojin_last
    set -e _mahojin_last
    set -q _mahojin_chanting; or return 0
    if test $st -ne 0; or string match -q -- '*starred/yukihirop/mahojin*' $line
        command mahojin --shatter $st -- $line
    end
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
        for shell in SHELLS {
            assert!(
                init(shell, Locale::Ja).unwrap().contains("--shatter"),
                "{shell}"
            );
        }
    }

    #[test]
    fn skips_only_plain_listed_commands() {
        let skip = skip_list(None);
        for line in ["ls", "ls -la", "  cd src", "LANG=C ls", "history"] {
            assert!(skipped(line, &skip), "{line}");
        }
        for line in [
            "git status",
            "cd src && make",
            "ls | grep x",
            "ls; make",
            "lsof",
            // 画面を消し去るのは魔法らしいので、既定では唱える
            "clear",
            "",
            "FOO=1",
        ] {
            assert!(!skipped(line, &skip), "{line}");
        }
        let custom = skip_list(Some(&["git".into(), "htop".into()]));
        assert!(skipped("git status", &custom) && !skipped("ls", &custom));
        // 空にすればすべてに出す
        assert!(!skipped("ls", &skip_list(Some(&[]))));
    }

    /// そのシェルが入っていれば、構文だけ確かめる。
    /// 手元に zsh や fish が無くても通るようにしてあるが、CI では `MAHOJIN_TEST_SHELLS=1` で必須にする。
    fn parses_in(shell: &str, check: &[&str]) {
        use std::io::Write;
        let Ok(mut child) = std::process::Command::new(shell)
            .args(check)
            .stdin(std::process::Stdio::piped())
            .spawn()
        else {
            assert!(
                std::env::var_os("MAHOJIN_TEST_SHELLS").is_none(),
                "{shell} is not installed"
            );
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
