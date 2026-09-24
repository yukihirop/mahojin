# mahojin

English | [日本語](README.ja.md)

**Cast your everyday commands as spells.**

`mahojin` is a wrapper you can put in front of any CLI command. Before running the command,
it unfolds a magic circle made just for that command, right in your terminal.

![mahojin demo](https://raw.githubusercontent.com/yukihirop/mahojin/main/assets/demo.gif)

```console
$ mahojin git status
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

![A magic circle for each command](https://raw.githubusercontent.com/yukihirop/mahojin/main/assets/gallery.jpg)

## Install

With Rust installed:

```sh
cargo install mahojin
```

Or download a prebuilt binary for macOS (Apple Silicon / Intel) or Linux (x86_64 / aarch64) from
[Releases](https://github.com/yukihirop/mahojin/releases), and put `mahojin` somewhere on your `PATH`.

```sh
tar xzf mahojin-v0.1.0-aarch64-apple-darwin.tar.gz
mv mahojin-v0.1.0-aarch64-apple-darwin/mahojin ~/.local/bin/
```

The macOS binaries are not signed. If macOS refuses to open one you downloaded in a browser,
clear the quarantine flag once:

```sh
xattr -d com.apple.quarantine ~/.local/bin/mahojin
```

## Usage

```sh
mahojin git status
mahojin cargo build --release
mahojin "cargo build && ls"    # a single argument containing spaces goes to the shell (sh -c)
```

It's a bit long to type every time, so a short alias is a good idea.

```sh
alias m='mahojin'    # in ~/.zshrc or similar
m git status
```

| Option | Meaning |
| --- | --- |
| `--explain` | Show the circle's hash and the parameters drawn from it |
| `--svg <file>` | Write the circle out as SVG |
| `--no-run` | Unfold the circle but don't run the command |
| `--share` | Don't run the command; make a post text and image for X and the like |
| `--locale <ja\|en>` | Use this language for this run |
| `--setup` | Show settings; with `--locale`, save that language |
| `--grimoire` | Open the grimoire: see which kinds of circles you've collected |
| `--help`, `-h` | Show usage |

Options go only before the command. In `mahojin cargo --explain E0308`, `--explain` belongs to cargo.

```console
$ mahojin --explain git status
✦ Magic circle unfolded: git status
  hash      e62b04aadf39df1a47b771265e4ae5c452df3f1903d5c263ab00f088e86102f6
  tier      major spell
  layout breach  shape wheel  ornament star  rings 5  symmetry 3  runes 27  particles 398
  script dotted  size 1.12  spacing 0.37  separator none  band runes
  rotation 265.3°  hue 288.8°  clockwise
```

### Cast it when it counts

Coding agents type commands for us in a split second now. `mahojin` is a deliberate waste: a human casting
a command by hand and spending a second watching a circle unfold. Waste is a luxury only when it's
occasional, so rather than putting it in front of everything, save it for the commands that mark a moment.

```sh
alias push='mahojin git push'         # sending off work you've finished
alias release='mahojin git tag'       # tagging a release
alias deploy='mahojin make deploy'    # shipping to production
```

Arguments are part of the spell, so `release v1.2.0` and `release v1.3.0` get different circles and
tiers. A release that draws an ultimate spell is surely blessed. You can still `alias git='mahojin git'` if
you want a circle every time, but the seconds do add up.

### Chanting mode

Sometimes you want a circle on every command, just for the length of a release. Add one line to your
shell config, and between `mahojin --on` and `mahojin --off` the commands you type unfold circles.

```sh
eval "$(mahojin --init zsh)"     # ~/.zshrc
eval "$(mahojin --init bash)"    # ~/.bashrc
mahojin --init fish | source     # ~/.config/fish/config.fish
```

```console
$ mahojin --on
✦ Chanting: the commands you type unfold circles (mahojin --off to stop)
$ git tag v1.2.0 && git push --tags    # the whole line is one spell
$ mahojin --off
```

It doesn't put `mahojin` in front of your commands. It draws the circle just before each command runs and
leaves the running to the shell, so `cd`, aliases and pipes all work as usual. It only lasts for that
shell. The shells coding agents type into don't load your interactive config, so the circles only
appear when a human types.

Commands you type all the time, like `ls` and `cd`, get no circle. A line is skipped only when it is a
single command (no pipes, no `&&`) starting with one of the names below, so `cd src && make` still
gets one. Change the list with `chant_skip` in the config file (`mahojin --setup` shows where it is);
set it to `[]` to cast on every command.

```toml
chant_skip = ["cd", "ls", "ll", "la", "pwd", "exit", "history"]    # the default
```

bash has no pre-command hook like zsh and fish do, so mahojin builds one from the DEBUG trap. If
[bash-preexec](https://github.com/rcaloras/bash-preexec) is loaded first, it hooks into that instead.
If something else already uses the DEBUG trap, mahojin stays out and tells you so.

### Failed spells

When a command fails (exit code 1 to 127), its circle shatters: cracks run through it and the shards
drift apart and fall, in about a second. This happens both with `mahojin <command>` and in chanting mode.
Stopping a command with Ctrl-C or another signal doesn't shatter anything, and the exit code is still
the command's own.

To turn it off, put this in the config file:

```toml
shatter = false
```

### Showing off your circle

```console
$ mahojin --share git status | pbcopy
```

The command is not run. The post text goes to stdout, a 1200px image (`mahojin-<first 8 hex of the hash>.png`)
is written to the current directory, and a URL that opens X's composer with the text filled in goes to stderr.
Attach the image yourself.

```text
I cast "git status" and a major spell circle unfolded ✦

breach layout / wheel / star / 3-fold symmetry
Sigil e62b04aa

#mahojin
https://github.com/yukihirop/mahojin
```

### Language

Messages, names and the post text come in Japanese or English.

```sh
mahojin --setup --locale en    # save English as your language
mahojin --setup                # show the current language and where it came from
mahojin --locale ja ls         # Japanese just this once
```

The language is chosen in this order: `--locale` > `MAHOJIN_LOCALE` > the config file
(`$XDG_CONFIG_HOME/mahojin/config.toml`, or `~/.config/mahojin/config.toml`) > `LC_ALL` / `LC_MESSAGES` / `LANG`
(Japanese if it starts with `ja`) > English.

## Spell tiers

| Tier | Height | Chance |
| --- | --- | --- |
| minor spell | 8 rows | 35% |
| standard spell | 16 rows | 45% |
| major spell | 24 rows | 19% |
| ultimate spell | 36 rows | 1% |

The bigger the spell, the richer the circle. An ultimate spell is more than just a big circle; what it
becomes is for you to see when you draw one.

The tier comes from the hash too, so the same spell is the same tier no matter how many times you cast it.
Arguments are part of the spell, though, so different arguments can mean a different tier. Major and
ultimate spells announce themselves with a line like `✦ ultimate spell` under the circle.

There is one more tier that isn't in the table, and no amount of luck will draw it. It only appears when
you cast a spell you can't take back. Which spells, we won't say. The circle won't stop you, so think
before you cast.

## Grimoire

Every circle you cast is written into a grimoire. `mahojin --grimoire` shows how much of it you've filled:
the 38 kinds across tiers, shapes, layouts, ornaments, symmetries, scripts and bands, with the ones you
haven't met yet hidden as `???`.

```
✦ Grimoire  17 / 38  ████████░░░░░░░░░░░░  44%
  3 spells cast, 3 times in all

  tier          2/4  ???, standard spell 2, major spell 1, ???
  shape        2/12  ???, triangle, ???, ???, ???, ???, nested polygons, ???, ???, ???, ???, ???
  layout        3/3  classic, grand star, breach
  ...
  skeleton    3/144  shape × layout × tier
```

When a cast fills in something new, a line like `✦ New in the grimoire: major spell / nested polygons`
appears under the circle. Once all 38 are in, the skeletons (shape × layout × tier, 144 in all) are the
long game. An ornament only counts when you can see it, so a grand star layout doesn't fill one in.
Beyond the 38, the grimoire has a section that stays hidden until you cast a certain kind of spell, and
one that stays hidden until you cast at a certain time.

There are about 65 million + α combinations in all, so you'll hardly ever meet the same circle twice.
Collecting every skeleton takes around 15,000 different commands, so completing it is
practically impossible; take your time. Arguments are part of the spell, so every `git commit -m "..."` casts a new one.

The grimoire lives at `$XDG_DATA_HOME/mahojin/grimoire` (or `~/.local/share/mahojin/grimoire`). It keeps
only each command's hash, how many times you cast it, whether it drew the tier that isn't in the table,
and which of the hidden entries you've met; never the command itself or when you cast it.

## How a circle is decided

1. Take the SHA-256 of the command string (the arguments joined with spaces)
2. Seed a random generator (ChaCha8) with it, and draw the layout, core shape, ornament, number of rings,
   symmetry, rune count, particle count, rotation, hue, spin direction, how the spell is written, and the tier
3. The layout is one of 3, the core shape one of 12, and the ornament one of 6
4. The spell is written in one of 4 scripts, at its own glyph size and letter spacing, with one of 5
   separators between repeats. The outer band comes in 3 kinds
5. Build an SVG, turn it into PNGs with [resvg](https://github.com/linebender/resvg), and send them to the
   terminal: 16 frames that draw the circle from the outer band inward

What each kind looks like is for you to find out by casting. The ones you've met are in the grimoire
(`mahojin --grimoire`).

Cast on a special day or at a special hour, though, and the circle may change color while keeping its
shape. Which days, we won't say.

## Supported terminals

| Terminal | Method |
| --- | --- |
| Ghostty / Kitty | Kitty graphics protocol |
| iTerm2 / WezTerm | iTerm2 inline images |
| Inside tmux (3.3 or later) | Passed through to the terminal outside, if passthrough is on |
| Anything else | No image, just the line `✦ Magic circle unfolded: <command>` |

When stderr is not a terminal (for example when redirected), nothing is shown.

tmux drops image escape sequences unless you let them through. Add this to `~/.tmux.conf`:

```tmux
set -g allow-passthrough on
```

Inside tmux, Ghostty and Kitty get the image through Unicode placeholders, so it stays tied to its text
cells and scrolls and redraws with tmux. iTerm2 has no such mechanism, so mahojin works out where the pane
sits on the outer screen and draws there; the image may linger when you switch windows.

| Environment variable | Meaning |
| --- | --- |
| `MAHOJIN_GRAPHICS=kitty\|iterm\|none` | Skip detection and force a method |
| `MAHOJIN_ANIMATION=off` | Skip the animations and show one still: the finished circle, or the shattered one |
| `MAHOJIN_LOCALE=ja\|en` | Use this language, overriding the config file |
| `MAHOJIN_GRIMOIRE=off` | Don't write casts into the grimoire |

## Regenerating the README images

The GIF and gallery in `assets/` are the images `mahojin` actually sent to the terminal, with the terminal
window and command output drawn around them (the commands are replaced with stubs and never really run).

```sh
python3 scripts/demo.py    # needs cargo, ImageMagick and script(1)
```

## License

[MIT](LICENSE)
