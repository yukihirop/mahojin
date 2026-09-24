# Setup

Everything here is what `/mahojin-setup` does for you, done by hand. [Back to the README](../README.md)

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

## Skills

```sh
mahojin --skills    # in the language from --locale, or else the display language
```

This puts the `/mahojin-setup` and `/mahojin-teardown` skills in `~/.agents/skills/`, and links to them from `~/.claude/skills/` and `~/.codex/skills/` if those exist.
`/mahojin-setup` writes the chanting mode line below and `alias m='mahojin'` to your rc, and inside tmux checks the passthrough setting.

## Chanting mode

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

## Stopping

Call `/mahojin-teardown` in Claude Code or Codex to clean up what mahojin put in place: the rc lines (the `mahojin --init` line and
aliases pointing at mahojin), mahojin's lines in `~/.tmux.conf`, the skills and their symlinks, the settings, the grimoire and the binary.
It asks what to remove and shows the list before removing anything. By default the grimoire is moved to `~/mahojin-backup-<date>/`, not deleted.
