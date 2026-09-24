# mahojin

[![CI](https://github.com/yukihirop/mahojin/actions/workflows/ci.yml/badge.svg)](https://github.com/yukihirop/mahojin/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/mahojin.svg)](https://crates.io/crates/mahojin)
[![GitHub release](https://img.shields.io/github/v/release/yukihirop/mahojin)](https://github.com/yukihirop/mahojin/releases/latest)
[![License: MIT](https://img.shields.io/crates/l/mahojin.svg)](LICENSE)

[English](README.md) | 日本語

**いつものコマンドを、魔法として唱える。**

`mahojin` はどんな CLI コマンドの前にも付けられるラッパーです。コマンドを実行する前に、
そのコマンド専用の魔法陣を端末に展開します。

![mahojin のデモ](https://raw.githubusercontent.com/yukihirop/mahojin/main/assets/demo.gif)

```console
$ mahojin git status
```

- **同じ呪文からは、必ず同じ魔法陣が出ます。**魔法陣はコマンド文字列の SHA-256 から決まります。
  `git status` を唱えれば、いつでもどのマシンでもこの紫の三角が出ます
- **1 文字違えば、まったく別の魔法陣になります。**`ls` と `ls -la` は似ても似つきません
- **円に書かれた文字はコマンドそのものです。**帯に並ぶ架空の魔法文字は、
  唱えたコマンドを 1 バイトずつ書いたものです。同じ文字はどの魔法陣でも同じ字形になります
- **魔法には格があります。**コマンドごとに小魔法・中魔法・大魔法・極大魔法のどれかが決まり、
  魔法陣は端末の 8 行から 36 行まで大きくなります。大きいものほど出にくくなっています
- **仕事の邪魔はしません。**展開はおよそ 1 秒で、演出はすべて stderr に出ます。
  stdout・終了コード・パイプはそのままコマンドのものです

![コマンドごとの魔法陣](https://raw.githubusercontent.com/yukihirop/mahojin/main/assets/gallery.jpg)

## はじめ方

| | |
| --- | --- |
| **始める** | `cargo install mahojin && mahojin --skills` のあと、Claude Code か Codex で **`/mahojin-setup`** |
| **唱える** | `mahojin git status` |
| **全部のコマンドで唱える** | `mahojin --on` … `mahojin --off` |
| **集める** | `mahojin --grimoire` |
| **見せびらかす** | `mahojin --share git status` |
| **やめる** | **`/mahojin-teardown`** |

`/mahojin-setup` はどのシェルで使うかを聞いて、その rc に `mahojin --init` の行と短いエイリアスを書きます。
`/mahojin-teardown` は mahojin が置いたものを片付けます。ビルド済みバイナリは [Releases](https://github.com/yukihirop/mahojin/releases) にあります。

## もっと詳しく

- [使い方](docs/usage.ja.md): オプション、ここぞというときに唱える、失敗した呪文、見せびらかす、表示の言語
- [セットアップ](docs/setup.ja.md): インストール、詠唱モード、対応している端末と tmux、環境変数、やめ方
- [魔法](docs/spells.ja.md): 魔法の格、図鑑、魔法陣の決まり方

---

<sub>`assets/` の GIF とギャラリーは、`mahojin` が実際に端末へ送った画像に、`python3 scripts/demo.py` で枠を描き足したもの（cargo、ImageMagick、script(1) が要る）。ライセンス: [MIT](LICENSE)。</sub>
