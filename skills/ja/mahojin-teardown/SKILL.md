---
name: mahojin-teardown
description: mahojin をやめるときに、mahojin が置いたものを片付ける。rc の `mahojin --init` の行と mahojin を指す alias、`~/.tmux.conf` の mahojin の行、`~/.config/mahojin/`(設定)、`~/.local/share/mahojin/`(図鑑)、`~/.agents/skills/` のスキルとその symlink、バイナリ。何を消すかは AskUserQuestion で聞き、消す前に一覧を見せて確認を取る。`/mahojin-teardown` で呼ぶ。「mahojin をアンインストールして」「mahojin をやめたい」と言われたときにも使う。
---

# mahojin-teardown — mahojin を片付ける

`/mahojin-setup` と `mahojin` が置いたものを外す。

**消すものは AskUserQuestion で聞く。** 取り消しが効かないので、消す前に対象の一覧(パスと行)を見せて、もう一度確認を取る。
図鑑は集めたものなので、既定では消さずに退避する。

## mahojin が置くもの

| もの | 場所 |
|---|---|
| rc の行 | `~/.zshrc` / `~/.bashrc` / `~/.config/fish/config.fish` の `mahojin --init` の行、mahojin を指す alias、`# mahojin: …` のコメント |
| tmux | `~/.tmux.conf` の `# mahojin: …` と、その直後の `set -g allow-passthrough on` |
| 設定 | `$XDG_CONFIG_HOME/mahojin/`(無ければ `~/.config/mahojin/`) |
| 図鑑 | `$XDG_DATA_HOME/mahojin/`(無ければ `~/.local/share/mahojin/`) |
| スキル | `~/.agents/skills/mahojin-{setup,teardown}/` と、`~/.claude/skills/`・`~/.codex/skills/` からの symlink |
| バイナリ | `~/.cargo/bin/mahojin`(`cargo install`)、または Releases から置いた場所(`~/.local/bin/mahojin` など) |

## 手順

### 1. 見つける

消さずに、在りかだけを集める:

```
command -v mahojin; mahojin --version; mahojin --setup
grep -n 'mahojin' ~/.zshrc ~/.bashrc ~/.config/fish/config.fish ~/.tmux.conf 2>/dev/null
ls -d "${XDG_CONFIG_HOME:-$HOME/.config}/mahojin" "${XDG_DATA_HOME:-$HOME/.local/share}/mahojin" 2>/dev/null
ls -la ~/.agents/skills ~/.claude/skills ~/.codex/skills 2>/dev/null | grep mahojin
```

- rc は symlink なら実体を見る(`ls -l`)
- 拾うのは mahojin のものだけ: `mahojin --init` の行、値が `mahojin` で始まる alias(`alias m='mahojin'`、`alias push='mahojin git push'`)、`# mahojin: …` のコメント。**mahojin を含むだけの別の行は拾わない**。迷う行は報告に回して触らない
- `allow-passthrough` は、`# mahojin: …` のコメントの直後にあるものだけ拾う。コメントが無ければ、ほかの道具が使っているかもしれないので触らず、報告に書く
- symlink は、リンク先が mahojin のスキルのものだけ(`readlink` で確かめる)

### 2. 何を片付けるか聞く

AskUserQuestion で、1 の結果を添えて聞く(multiSelect)。

- 「rc と tmux の行 (Recommended)」: バイナリを消すなら必須(残すとシェル起動時に `mahojin: command not found` が出る)
- 「スキルと symlink (Recommended)」
- 「設定と図鑑」: 図鑑に何種集めたかを `mahojin --grimoire` の 1 行目で書き添える
- 「バイナリ」

「設定と図鑑」を選んだら、続けて聞く:

- 「退避する (Recommended)」: `~/mahojin-backup-<YYYYMMDD>/` に `config/` と `data/` として移す。戻したくなったら移し戻せる
- 「消す」: 図鑑も消える

### 3. 一覧を見せて確認する

外す行(ファイルと行番号と中身)、移す・消すパスを全部並べて見せ、AskUserQuestion で「この内容で片付ける」「やめる」を聞く。「やめる」なら何もせずに終わる。

### 4. 片付ける

この順で行う:

1. **rc と tmux の行**: 書き換える前に `<file>.mahojin-teardown.bak` にコピーし、3 で見せた行だけを消す。rc は構文を確かめる(`zsh -n` / `bash -n` / `fish -n`)。落ちたらバックアップから戻して止まる
2. **設定と図鑑**: 退避なら `mv`、消すなら `rm -rf`。パスは 1 で確かめたものだけ
3. **スキルと symlink**: 先に symlink、次にスキル本体。**このスキル自身(mahojin-teardown)は最後に消す**
4. **バイナリ**: `~/.cargo/bin/mahojin` なら `cargo uninstall mahojin`。それ以外の場所なら、そのファイルを消す。Homebrew など別の道具が入れたものは消さずに場所を報告する

### 5. 報告

- 外した行(ファイルと中身)とバックアップの場所
- 退避先、または消したパス
- 消したスキルと symlink、バイナリ
- 残したもの(選ばなかったもの、迷って触らなかった行)
- 今開いているシェルには mahojin の関数が残っている。`exec zsh`(bash なら `exec bash`、fish なら `exec fish`)で開き直す
- 退避したなら、戻し方: `mv ~/mahojin-backup-<YYYYMMDD>/config ~/.config/mahojin` と `mv ~/mahojin-backup-<YYYYMMDD>/data ~/.local/share/mahojin`
