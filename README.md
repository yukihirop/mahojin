# maho

English | [日本語](README.ja.md)

**Cast your everyday commands as spells.**

`maho` is a wrapper you can put in front of any CLI command. Before running the command,
it unfolds a magic circle made just for that command, right in your terminal.

![maho demo](assets/demo.gif)

```console
$ maho git status
```

- **The same spell always summons the same circle.** The circle is derived from the SHA-256 of
  the command string. Cast `git status` and you get this purple triangle, any time, on any machine
- **Change one character and you get a completely different circle.** `ls` and `ls -la` look nothing alike
- **The writing on the rings is the command itself.** The invented magic script on the bands spells out
  the command one byte at a time. The same character has the same glyph in every circle
- **It stays out of your way.** The circle unfolds in about a second, and all of it goes to stderr.
  stdout, the exit code and pipes stay the command's own

![A magic circle for each command](assets/gallery.jpg)

## Install

```sh
cargo install --git https://github.com/yukihirop/maho
```

## Usage

```sh
maho git status
maho cargo build --release
maho "cargo build && ls"      # a single argument containing spaces goes to the shell (sh -c)
```

If you want it every time, an alias makes it easy.

```sh
alias git='maho git'
```

| Option | Meaning |
| --- | --- |
| `--explain` | Show the circle's hash and the parameters drawn from it |
| `--svg <file>` | Write the circle out as SVG |

Options go only before the command. In `maho cargo --explain E0308`, `--explain` belongs to cargo.

```console
$ maho --explain git status
✦ 魔法陣展開: git status
  hash      e62b04aadf39df1a47b771265e4ae5c452df3f1903d5c263ab00f088e86102f6
  layout 突破  shape 車輪  ornament 星形  rings 5  symmetry 3  runes 27  particles 398
  rotation 265.3°  hue 288.8°  右回り
```

(The output is in Japanese: 魔法陣展開 means "magic circle unfolded", 右回り means "clockwise".)

## How a circle is decided

1. Take the SHA-256 of the command string (the arguments joined with spaces)
2. Seed a random generator (ChaCha8) with it, and draw the layout, core shape, ornament, number of rings,
   symmetry, rune count, particle count, rotation, hue and spin direction
3. The layout is one of 3 (classic / grand star / breach), the core shape one of 12 (hexagram, pentagram,
   spiral, Metatron's cube, wheel, and more), and the ornament one of 6 (star, chain, rays, crown, web, beads)
4. Build an SVG, turn it into PNGs with [resvg](https://github.com/linebender/resvg), and send them to the
   terminal: 16 frames that draw the circle from the outer band inward

## Supported terminals

| Terminal | Method |
| --- | --- |
| Ghostty / Kitty | Kitty graphics protocol |
| iTerm2 / WezTerm | iTerm2 inline images |
| Anything else / inside tmux | No image, just the line `✦ 魔法陣展開: <command>` |

When stderr is not a terminal (for example when redirected), nothing is shown.

| Environment variable | Meaning |
| --- | --- |
| `MAHO_GRAPHICS=kitty\|iterm\|none` | Skip detection and force a method |
| `MAHO_ANIMATION=off` | Skip the unfolding animation and show only the finished circle |

## Regenerating the README images

The GIF and gallery in `assets/` are the images `maho` actually sent to the terminal, with the terminal
window and command output drawn around them (the commands are replaced with stubs and never really run).

```sh
python3 scripts/demo.py   # needs cargo, ImageMagick and script(1)
```
