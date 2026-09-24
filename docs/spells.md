# Spells

[Back to the README](../README.md)

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
