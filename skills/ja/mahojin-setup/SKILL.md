---
name: mahojin-setup
description: mahojin を初めて使えるようにする。どのシェルで使うかを聞いて、その rc に `mahojin --init` の行(詠唱モード)と短いエイリアスを書き、tmux の中なら画像の素通しを確かめる。`/mahojin-setup` で呼ぶ。「mahojin をセットアップして」「mahojin の初期設定」と言われたときにも使う。
---

# mahojin-setup — mahojin の初回セットアップ

mahojin は、コマンドを実行する前に、そのコマンドだけの魔法陣を端末に出すラッパー。`mahojin git status` はそれだけで動くが、詠唱モード(`mahojin --on`)には rc の `mahojin --init <shell>` の行が要る。このスキルはそれを書く。

**聞くことは AskUserQuestion で聞く。** 勝手に決めない。
rc を書き換える前に、今の中身を読む。同じ行がもうあれば足さない。rc が symlink なら実体を書き換える(`ls -l` で見る)。

## 手順

### 1. mahojin があるか

```
mahojin --version
```

無ければ止めて、`cargo install mahojin` を案内する(このスキルでは入れない)。

### 2. シェルとエイリアスを聞く

`echo $SHELL` でログインシェルを見て、それを推奨(先頭、ラベル末尾に「(Recommended)」)にして聞く。

| シェル | rc | 書く行 |
|---|---|---|
| zsh | `~/.zshrc` | `eval "$(mahojin --init zsh)"` |
| bash | `~/.bashrc` | `eval "$(mahojin --init bash)"` |
| fish | `~/.config/fish/config.fish` | `mahojin --init fish \| source` |

同じ AskUserQuestion で、エイリアスも聞く(multiSelect):

- 「`m` (Recommended)」: `alias m='mahojin'`(fish も `alias` で書ける)
- 「節目のエイリアス」: `push` / `release` / `deploy` を `mahojin git push` / `mahojin git tag` / `mahojin make deploy` に。**`type push` などで既に使われている名前は書かず、報告に回す**
- 「書かない」

### 3. rc に書く

`mahojin --init` を含む行がもうあれば足さない。無ければ末尾に足す:

```
# mahojin: mahojin --on で、打ったコマンドに魔法陣を出す
<2 の表の行>
alias m='mahojin'
```

書いたら構文を確かめる(`zsh -n ~/.zshrc`、`bash -n ~/.bashrc`、`fish -n ~/.config/fish/config.fish`)。

### 4. 言語

`mahojin --setup` の `locale` を見る。会話の言語と違えば、AskUserQuestion で聞いて `mahojin --setup --locale <ja|en>` で保存する。

### 5. tmux

`$TMUX` があるときだけ見る:

```
tmux show -gv allow-passthrough
```

`on` でなければ、`~/.tmux.conf` に次の 2 行を足すかを聞く(足さないと tmux の中では画像が出ない)。足したら `tmux source-file ~/.tmux.conf`。

```
# mahojin: 画像を外側の端末へ素通しする
set -g allow-passthrough on
```

### 6. 報告

- 書き換えた rc(実体のパス)と足した行。足さなかった行とその理由
- 言語、tmux の設定
- 今のシェルには効いていない。`exec zsh`(bash なら `exec bash`、fish なら `exec fish`)で開き直す
- 試す例: `mahojin git status`、`mahojin --on` のあと普通にコマンドを打つ(`mahojin --off` で戻る)、`mahojin --grimoire`
- エージェントのシェルは端末ではないので、ここで唱えても魔法陣は見えない。試すのはユーザーの端末で
- 片付けるときは `/mahojin-teardown`
