# mahojin

[![CI](https://github.com/yukihirop/mahojin/actions/workflows/ci.yml/badge.svg)](https://github.com/yukihirop/mahojin/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/mahojin.svg)](https://crates.io/crates/mahojin)
[![GitHub release](https://img.shields.io/github/v/release/yukihirop/mahojin)](https://github.com/yukihirop/mahojin/releases/latest)
[![License: MIT](https://img.shields.io/crates/l/mahojin.svg)](LICENSE)

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

## Get started

| | |
| --- | --- |
| **Start** | `cargo install mahojin && mahojin --skills`, then **`/mahojin-setup`** in Claude Code or Codex |
| **Cast** | `mahojin git status` |
| **Chant every command** | `mahojin --on` … `mahojin --off` |
| **Collect** | `mahojin --grimoire` |
| **Show off** | `mahojin --share git status` |
| **Stop** | **`/mahojin-teardown`** |

`/mahojin-setup` asks which shell you use and writes the `mahojin --init` line and a short alias to its rc.
`/mahojin-teardown` cleans up what mahojin put in place. Prebuilt binaries are on [Releases](https://github.com/yukihirop/mahojin/releases).

## More

- [Usage](docs/usage.md): options, when to cast, failed spells, sharing, language
- [Setup](docs/setup.md): install, chanting mode, supported terminals and tmux, environment variables, stopping
- [Spells](docs/spells.md): tiers, the grimoire, how a circle is decided

---

<sub>The GIF and gallery in `assets/` are the images `mahojin` actually sent to the terminal, framed by `python3 scripts/demo.py` (needs cargo, ImageMagick and script(1)). License: [MIT](LICENSE).</sub>
