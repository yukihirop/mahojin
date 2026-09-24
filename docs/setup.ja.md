# セットアップ

ここに書いたのは、`/mahojin-setup` が代わりにやることを手でやる方法です。[README に戻る](../README.ja.md)

## インストール

Rust が入っていれば:

```sh
cargo install mahojin
```

macOS（Apple Silicon / Intel）と Linux（x86_64 / aarch64）のビルド済みバイナリは
[Releases](https://github.com/yukihirop/mahojin/releases) にあります。展開して、`mahojin` を `PATH` の通った場所に置いてください。

```sh
tar xzf mahojin-v0.1.1-aarch64-apple-darwin.tar.gz
mv mahojin-v0.1.1-aarch64-apple-darwin/mahojin ~/.local/bin/
```

macOS のバイナリには署名をしていません。ブラウザで落としたものが開けないと言われたら、
一度だけ隔離の印を外してください。

```sh
xattr -d com.apple.quarantine ~/.local/bin/mahojin
```

## スキル

```sh
mahojin --skills    # 言語は --locale、無ければ表示の言語
```

`/mahojin-setup` と `/mahojin-teardown` のスキルを `~/.agents/skills/` に置きます。`~/.claude/skills/` と `~/.codex/skills/` があれば、そこからリンクも張ります。
`/mahojin-setup` は、下の詠唱モードの行と `alias m='mahojin'` を rc に書き、tmux の中なら素通しの設定を確かめます。

## 詠唱モード

リリース作業のあいだだけ、全部のコマンドで魔法陣を見たいこともあります。シェルの設定に 1 行足すと、
`mahojin --on` から `mahojin --off` までのあいだ、打ったコマンドに魔法陣が出ます。

```sh
eval "$(mahojin --init zsh)"     # ~/.zshrc
eval "$(mahojin --init bash)"    # ~/.bashrc
mahojin --init fish | source     # ~/.config/fish/config.fish
```

```console
$ mahojin --on
✦ 詠唱モード: 打ったコマンドに魔法陣が出ます（mahojin --off で戻る）
$ git tag v1.2.0 && git push --tags    # この行全体が 1 つの呪文になる
$ mahojin --off
```

コマンドの前に `mahojin` を足すのではなく、実行の直前に魔法陣だけを描いて、実行はシェルに任せます。
`cd` もエイリアスもパイプもそのまま動きます。オンになるのはそのシェルの中だけで、閉じれば元に戻ります。
エージェントがコマンドを打つシェルは対話用の設定を読まないので、魔法陣が出るのは人間が打ったときだけです。

`ls` や `cd` のように何度も打つコマンドには出しません。飛ばすのは、パイプや `&&` の無い 1 つだけのコマンドで、
先頭が次のどれかのときです。`cd src && make` なら出ます。設定ファイル（`mahojin --setup` で場所が出ます）の
`chant_skip` で入れ替えられ、`[]` にすればすべてのコマンドに出ます。

```toml
chant_skip = ["cd", "ls", "ll", "la", "pwd", "exit", "history"]    # 既定
```

bash には zsh や fish のような実行直前のフックが無いので、DEBUG トラップで作っています。
[bash-preexec](https://github.com/rcaloras/bash-preexec) を先に読み込んでいれば、そちらに乗ります。
ほかの道具が DEBUG トラップを使っているときは、入れずにそう知らせます。

## 対応している端末

| 端末 | 方式 |
| --- | --- |
| Ghostty / Kitty | Kitty graphics protocol |
| iTerm2 / WezTerm | iTerm2 inline images |
| tmux の中（3.3 以降） | 素通しを有効にしていれば、外側の端末へそのまま送る |
| それ以外 | 画像は出さず、`✦ 魔法陣展開: <コマンド>` の 1 行だけ |

stderr が端末でないとき（リダイレクト中など）は何も出しません。

tmux は、許可しない限り画像のエスケープシーケンスを捨てます。`~/.tmux.conf` に次を足してください。

```tmux
set -g allow-passthrough on
```

tmux の中の Ghostty と Kitty には、Unicode placeholder で画像を送ります。画像が文字のマスに結び付くので、
tmux のスクロールや描き直しについてきます。iTerm2 にはその仕組みが無いので、外側の画面でのペインの位置を計算して描きます。
ウィンドウを切り替えたときに画像が残ることがあります。

| 環境変数 | 意味 |
| --- | --- |
| `MAHOJIN_GRAPHICS=kitty\|iterm\|none` | 自動判定をやめて方式を固定する |
| `MAHOJIN_ANIMATION=off` | アニメーションをやめ、完成図（砕けるときは砕けた姿）を 1 枚だけ出す |
| `MAHOJIN_LOCALE=ja\|en` | 設定ファイルより優先して、この言語で表示する |
| `MAHOJIN_GRIMOIRE=off` | 唱えた魔法陣を図鑑に記録しない |

## mahojin をやめる

Claude Code か Codex で `/mahojin-teardown` を呼ぶと、mahojin が置いたものを片付けます: rc の行（`mahojin --init` の行と mahojin を指す alias）、
`~/.tmux.conf` の mahojin の行、スキルとその symlink、設定、図鑑、バイナリ。何を消すかを聞き、消す前に一覧を見せます。
図鑑は、既定では消さずに `~/mahojin-backup-<日付>/` に移します。
