# 使い方

[README に戻る](../README.ja.md)

```sh
mahojin git status
mahojin cargo build --release
mahojin "cargo build && ls"    # 空白を含む引数 1 つはシェル（sh -c）に渡す
```

毎回打つには少し長いので、短いエイリアスを付けておくのがおすすめです。

```sh
alias m='mahojin'    # ~/.zshrc など
m git status
```

| オプション | 意味 |
| --- | --- |
| `--explain` | 魔法陣のハッシュと、そこから引いたパラメータを表示する |
| `--svg <file>` | 魔法陣を SVG に書き出す |
| `--no-run` | 魔法陣だけ出して、コマンドは実行しない |
| `--share` | コマンドは実行せず、X などに貼る投稿文と画像を作る |
| `--locale <ja\|en>` | 今回だけ表示の言語を変える |
| `--setup` | 設定を表示する。`--locale` と一緒なら、その言語を保存する |
| `--grimoire` | 図鑑を開く。これまでに集めた魔法陣の種類を見る |
| `--skills` | `/mahojin-setup` と `/mahojin-teardown` のスキルを置く（[セットアップ](setup.ja.md#スキル)） |
| `--help`, `-h` | 使い方を表示する |

オプションはコマンドより前にだけ置けます。`mahojin cargo --explain E0308` の `--explain` は cargo のものです。

```console
$ mahojin --explain git status
✦ 魔法陣展開: git status
  hash      e62b04aadf39df1a47b771265e4ae5c452df3f1903d5c263ab00f088e86102f6
  tier      大魔法
  layout 突破  shape 車輪  ornament 星形  rings 5  symmetry 3  runes 27  particles 398
  script 点文字  size 1.12  spacing 0.37  separator なし  band ルーン
  rotation 265.3°  hue 288.8°  右回り
```

## ここぞというときに唱える

コマンドはエージェントが秒で打ってくれる時代です。`mahojin` は、人間があえて手で唱え、1 秒待って
魔法陣を眺めるための無駄です。無駄はたまにやるから贅沢なので、全部のコマンドに付けるより、
節目のコマンドにだけ付けるのがおすすめです。

```sh
alias push='mahojin git push'         # 仕上げた変更を送り出すとき
alias release='mahojin git tag'       # リリースのタグを打つとき
alias deploy='mahojin make deploy'    # 本番に出すとき
```

引数も呪文のうちなので、`release v1.2.0` と `release v1.3.0` では魔法陣も格も変わります。
極大魔法を引いたリリースは、きっと縁起がいいはずです。どうしても毎回見たいなら
`alias git='mahojin git'` もできますが、1 秒の待ちが積み重なることはお忘れなく。

## 失敗した呪文

唱えたコマンドが失敗すると（終了コード 1〜127）、魔法陣が砕けます。ひびが走り、破片がばらけて落ちるまで、
およそ 1 秒です。`mahojin <コマンド>` でも詠唱モードでも同じです。Ctrl-C などシグナルで止めたときは砕けません。
砕けても、終了コードはコマンドのものがそのまま返ります。

砕けるのをやめたいときは、設定ファイルに書きます。

```toml
shatter = false
```

## 魔法陣を見せびらかす

```console
$ mahojin --share git status | pbcopy
```

コマンドは実行しません。投稿文を stdout に、1200px の画像（`mahojin-<ハッシュ先頭8桁>.png`）を
今のディレクトリに書き出し、投稿文が入った X の投稿画面の URL を stderr に出します。画像は手で添えてください。

```text
「git status」を唱えたら、大魔法の魔法陣が展開した ✦

突破の陣 / 車輪 / 星形 / 3 回対称
呪紋 e62b04aa

#mahojin
https://github.com/yukihirop/mahojin
```

## 表示の言語

メッセージ・図形の名前・投稿文は、日本語と英語を切り替えられます。

```sh
mahojin --setup --locale ja    # 日本語を保存する
mahojin --setup                # 今の言語と、それがどこから決まったかを表示する
mahojin --locale en ls         # 今回だけ英語
```

決まる順番は `--locale` > `MAHOJIN_LOCALE` > 設定ファイル
（`$XDG_CONFIG_HOME/mahojin/config.toml`、無ければ `~/.config/mahojin/config.toml`）> `LC_ALL` / `LC_MESSAGES` / `LANG`
（`ja` で始まれば日本語）> 英語 です。
