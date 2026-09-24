# Usage

[Back to the README](../README.md)

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
| `--skills` | Install the `/mahojin-setup` and `/mahojin-teardown` skills ([Setup](setup.md#skills)) |
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

## Cast it when it counts

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

## Failed spells

When a command fails (exit code 1 to 127), its circle shatters: cracks run through it and the shards
drift apart and fall, in about a second. This happens both with `mahojin <command>` and in chanting mode.
Stopping a command with Ctrl-C or another signal doesn't shatter anything, and the exit code is still
the command's own.

To turn it off, put this in the config file:

```toml
shatter = false
```

## Showing off your circle

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

## Language

Messages, names and the post text come in Japanese or English.

```sh
mahojin --setup --locale en    # save English as your language
mahojin --setup                # show the current language and where it came from
mahojin --locale ja ls         # Japanese just this once
```

The language is chosen in this order: `--locale` > `MAHOJIN_LOCALE` > the config file
(`$XDG_CONFIG_HOME/mahojin/config.toml`, or `~/.config/mahojin/config.toml`) > `LC_ALL` / `LC_MESSAGES` / `LANG`
(Japanese if it starts with `ja`) > English.
