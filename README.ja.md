# maho

[English](README.md) | 日本語

**いつものコマンドを、魔法として唱える。**

`maho` はどんな CLI コマンドの前にも付けられるラッパーです。コマンドを実行する前に、
そのコマンド専用の魔法陣を端末に展開します。

![maho のデモ](assets/demo.gif)

```console
$ maho git status
```

- **同じ呪文からは、必ず同じ魔法陣が出ます。**魔法陣はコマンド文字列の SHA-256 から決まります。
  `git status` を唱えれば、いつでもどのマシンでもこの紫の三角が出ます
- **1 文字違えば、まったく別の魔法陣になります。**`ls` と `ls -la` は似ても似つきません
- **円に書かれた文字はコマンドそのものです。**帯に並ぶ架空の魔法文字は、
  唱えたコマンドを 1 バイトずつ書いたものです。同じ文字はどの魔法陣でも同じ字形になります
- **仕事の邪魔はしません。**展開はおよそ 1 秒で、演出はすべて stderr に出ます。
  stdout・終了コード・パイプはそのままコマンドのものです

![コマンドごとの魔法陣](assets/gallery.jpg)

## インストール

```sh
cargo install --git https://github.com/yukihirop/maho
```

## 使い方

```sh
maho git status
maho cargo build --release
maho "cargo build && ls"      # 空白を含む引数 1 つはシェル（sh -c）に渡す
```

毎回付けるなら alias にしておくと楽です。

```sh
alias git='maho git'
```

| オプション | 意味 |
| --- | --- |
| `--explain` | 魔法陣のハッシュと、そこから引いたパラメータを表示する |
| `--svg <file>` | 魔法陣を SVG に書き出す |
| `--share` | コマンドは実行せず、X などに貼る投稿文と画像を作る |
| `--locale <ja\|en>` | 今回だけ表示の言語を変える |
| `--setup` | 設定を表示する。`--locale` と一緒なら、その言語を保存する |

オプションはコマンドより前にだけ置けます。`maho cargo --explain E0308` の `--explain` は cargo のものです。

```console
$ maho --explain git status
✦ 魔法陣展開: git status
  hash      e62b04aadf39df1a47b771265e4ae5c452df3f1903d5c263ab00f088e86102f6
  layout 突破  shape 車輪  ornament 星形  rings 5  symmetry 3  runes 27  particles 398
  rotation 265.3°  hue 288.8°  右回り
```

### 魔法陣を見せびらかす

```console
$ maho --share git status | pbcopy
```

コマンドは実行しません。投稿文を stdout に、1200px の画像（`maho-<ハッシュ先頭8桁>.png`）を
今のディレクトリに書き出し、投稿文が入った X の投稿画面の URL を stderr に出します。画像は手で添えてください。

```text
「git status」を唱えたら、この魔法陣が展開した ✦

突破の陣 / 車輪 / 星形 / 3 回対称
呪紋 e62b04aa

#maho
https://github.com/yukihirop/maho
```

### 表示の言語

メッセージ・図形の名前・投稿文は、日本語と英語を切り替えられます。

```sh
maho --setup --locale ja   # 日本語を保存する
maho --setup               # 今の言語と、それがどこから決まったかを表示する
maho --locale en ls        # 今回だけ英語
```

決まる順番は `--locale` > `MAHO_LOCALE` > 設定ファイル
（`$XDG_CONFIG_HOME/maho/config`、無ければ `~/.config/maho/config`）> `LC_ALL` / `LC_MESSAGES` / `LANG`
（`ja` で始まれば日本語）> 英語 です。

## 魔法陣の決まり方

1. コマンド文字列（引数を空白で連結したもの）の SHA-256 を取る
2. それを種にした乱数（ChaCha8）から、配置・中心図形・装飾・円の数・対称性・ルーン数・
   粒子数・回転・色相・回転方向を引く
3. 配置は 3 種（標準 / 大星 / 突破）、中心図形は 12 種（六芒星・五芒星・螺旋・
   メタトロン・車輪など）、装飾は 6 種（星形・円鎖・光条・冠・網・数珠）から選ばれる
4. SVG を組み立て、[resvg](https://github.com/linebender/resvg) で PNG にして端末へ送る。
   外側の帯から内側へ、順に描き上がっていくコマを 16 枚流す

## 対応している端末

| 端末 | 方式 |
| --- | --- |
| Ghostty / Kitty | Kitty graphics protocol |
| iTerm2 / WezTerm | iTerm2 inline images |
| それ以外 / tmux の中 | 画像は出さず、`✦ 魔法陣展開: <コマンド>` の 1 行だけ |

stderr が端末でないとき（リダイレクト中など）は何も出しません。

| 環境変数 | 意味 |
| --- | --- |
| `MAHO_GRAPHICS=kitty\|iterm\|none` | 自動判定をやめて方式を固定する |
| `MAHO_ANIMATION=off` | 展開アニメーションをやめ、完成図 1 枚だけ出す |
| `MAHO_LOCALE=ja\|en` | 設定ファイルより優先して、この言語で表示する |

## README の画像を作り直す

`assets/` の GIF とギャラリーは、`maho` が実際に端末へ送った画像を抜き出して、
ターミナルの枠とコマンドの出力を描き足したものです（コマンド自体は偽物に差し替えて走らせません）。

```sh
python3 scripts/demo.py   # cargo, ImageMagick, script(1) が要る
```
