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
- **Some spells are bigger than others.** Each command is a minor, standard, major or ultimate spell,
  and the circle grows from 8 to 36 terminal rows. The bigger, the rarer
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
| `--share` | Don't run the command; make a post text and image for X and the like |
| `--locale <ja\|en>` | Use this language for this run |
| `--setup` | Show settings; with `--locale`, save that language |

Options go only before the command. In `maho cargo --explain E0308`, `--explain` belongs to cargo.

```console
$ maho --explain git status
✦ Magic circle unfolded: git status
  hash      e62b04aadf39df1a47b771265e4ae5c452df3f1903d5c263ab00f088e86102f6
  tier      major spell
  layout breach  shape wheel  ornament star  rings 5  symmetry 3  runes 27  particles 398
  script dotted  size 1.12  spacing 0.37  separator none  band runes
  rotation 265.3°  hue 288.8°  clockwise
```

### Showing off your circle

```console
$ maho --share git status | pbcopy
```

The command is not run. The post text goes to stdout, a 1200px image (`maho-<first 8 hex of the hash>.png`)
is written to the current directory, and a URL that opens X's composer with the text filled in goes to stderr.
Attach the image yourself.

```text
I cast "git status" and a major spell circle unfolded ✦

breach layout / wheel / star / 3-fold symmetry
Sigil e62b04aa

#maho
https://github.com/yukihirop/maho
```

### Language

Messages, names and the post text come in Japanese or English.

```sh
maho --setup --locale en   # save English as your language
maho --setup               # show the current language and where it came from
maho --locale ja ls        # Japanese just this once
```

The language is chosen in this order: `--locale` > `MAHO_LOCALE` > the config file
(`$XDG_CONFIG_HOME/maho/config`, or `~/.config/maho/config`) > `LC_ALL` / `LC_MESSAGES` / `LANG`
(Japanese if it starts with `ja`) > English.

## Spell tiers

| Tier | Height | Chance |
| --- | --- | --- |
| minor spell | 8 rows | 35% |
| standard spell | 16 rows | 45% |
| major spell | 24 rows | 19% |
| ultimate spell | 36 rows | 1% |

The bigger the spell, the richer the circle. A minor spell is a simpler sketch with fewer rings, one
band of script and fewer sparks. A major spell lays a second circle behind the first, faint and turning
the other way. An ultimate spell is more than a big circle: another circle is laid out as a frame, the command's own
circle sits shrunk in the middle, and small circles orbit on the frame's band, unfolding one after another.

The tier comes from the hash too, so a command is always the same tier (`cargo run` is an ultimate spell). Major and ultimate spells
announce themselves with a line like `✦ ultimate spell` under the circle.

## How a circle is decided

1. Take the SHA-256 of the command string (the arguments joined with spaces)
2. Seed a random generator (ChaCha8) with it, and draw the layout, core shape, ornament, number of rings,
   symmetry, rune count, particle count, rotation, hue, spin direction, how the spell is written, and the tier
3. The layout is one of 3 (classic / grand star / breach), the core shape one of 12 (hexagram, pentagram,
   spiral, Metatron's cube, wheel, and more), and the ornament one of 6 (star, chain, rays, crown, web, beads)
4. The spell is written in one of 4 scripts (runic / angular / flowing / dotted), at its own glyph size and
   letter spacing, with one of 5 separators between repeats. The outer band holds runes with dots, the spell
   in large glyphs, or runes with clock-like ticks
5. Build an SVG, turn it into PNGs with [resvg](https://github.com/linebender/resvg), and send them to the
   terminal: 16 frames that draw the circle from the outer band inward

## Supported terminals

| Terminal | Method |
| --- | --- |
| Ghostty / Kitty | Kitty graphics protocol |
| iTerm2 / WezTerm | iTerm2 inline images |
| Anything else / inside tmux | No image, just the line `✦ Magic circle unfolded: <command>` |

When stderr is not a terminal (for example when redirected), nothing is shown.

| Environment variable | Meaning |
| --- | --- |
| `MAHO_GRAPHICS=kitty\|iterm\|none` | Skip detection and force a method |
| `MAHO_ANIMATION=off` | Skip the unfolding animation and show only the finished circle |
| `MAHO_LOCALE=ja\|en` | Use this language, overriding the config file |

## Regenerating the README images

The GIF and gallery in `assets/` are the images `maho` actually sent to the terminal, with the terminal
window and command output drawn around them (the commands are replaced with stubs and never really run).

```sh
python3 scripts/demo.py   # needs cargo, ImageMagick and script(1)
```
