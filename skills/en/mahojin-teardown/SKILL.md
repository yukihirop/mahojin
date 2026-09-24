---
name: mahojin-teardown
description: Clean up what mahojin put in place when you stop using it. The `mahojin --init` line and aliases pointing at mahojin in the rc, mahojin's lines in `~/.tmux.conf`, `~/.config/mahojin/` (settings), `~/.local/share/mahojin/` (the grimoire), the skills in `~/.agents/skills/` and their symlinks, and the binary. Asks what to remove with AskUserQuestion, and shows the list for confirmation before removing anything. Invoke with `/mahojin-teardown`, or when asked to "uninstall mahojin" or "stop using mahojin".
---

# mahojin-teardown — clean up mahojin

Removes what `/mahojin-setup` and `mahojin` put in place.

**Ask what to remove with AskUserQuestion.** It can't be undone, so show the full list (paths and lines) and confirm once more before removing anything.
The grimoire is a collection, so by default it is moved aside, not deleted.

## What mahojin puts in place

| What | Where |
|---|---|
| rc lines | the `mahojin --init` line, aliases pointing at mahojin and `# mahojin: …` comments in `~/.zshrc` / `~/.bashrc` / `~/.config/fish/config.fish` |
| tmux | `# mahojin: …` in `~/.tmux.conf` and the `set -g allow-passthrough on` right after it |
| Settings | `$XDG_CONFIG_HOME/mahojin/` (or `~/.config/mahojin/`) |
| Grimoire | `$XDG_DATA_HOME/mahojin/` (or `~/.local/share/mahojin/`) |
| Skills | `~/.agents/skills/mahojin-{setup,teardown}/` and symlinks to them from `~/.claude/skills/` and `~/.codex/skills/` |
| Binary | `~/.cargo/bin/mahojin` (`cargo install`), or wherever a Releases binary was put (`~/.local/bin/mahojin` and so on) |

## Steps

### 1. Find

Collect locations only; remove nothing yet:

```
command -v mahojin; mahojin --version; mahojin --setup
grep -n 'mahojin' ~/.zshrc ~/.bashrc ~/.config/fish/config.fish ~/.tmux.conf 2>/dev/null
ls -d "${XDG_CONFIG_HOME:-$HOME/.config}/mahojin" "${XDG_DATA_HOME:-$HOME/.local/share}/mahojin" 2>/dev/null
ls -la ~/.agents/skills ~/.claude/skills ~/.codex/skills 2>/dev/null | grep mahojin
```

- If an rc is a symlink, look at the real file (`ls -l`)
- Pick only mahojin's own lines: the `mahojin --init` line, aliases whose value starts with `mahojin` (`alias m='mahojin'`, `alias push='mahojin git push'`), and `# mahojin: …` comments. **Leave other lines that merely contain mahojin.** Report lines you're unsure of and don't touch them
- Pick `allow-passthrough` only when it follows a `# mahojin: …` comment. Without the comment, another tool may rely on it: leave it and report it
- Only symlinks that point at mahojin's skills (check with `readlink`)

### 2. Ask what to clean up

Ask with AskUserQuestion (multiSelect), with what step 1 found:

- "rc and tmux lines (Recommended)": required if the binary goes (otherwise every new shell prints `mahojin: command not found`)
- "Skills and symlinks (Recommended)"
- "Settings and grimoire": mention how much of the grimoire is filled (the first line of `mahojin --grimoire`)
- "Binary"

If "Settings and grimoire" is chosen, ask next:

- "Move aside (Recommended)": move them to `~/mahojin-backup-<YYYYMMDD>/` as `config/` and `data/`, so they can be moved back
- "Delete": the grimoire goes too

### 3. Show the list and confirm

List every line to remove (file, line number, content) and every path to move or delete, and ask with AskUserQuestion: "Clean up as listed" or "Cancel". On "Cancel", stop without changing anything.

### 4. Clean up

In this order:

1. **rc and tmux lines**: copy the file to `<file>.mahojin-teardown.bak` first, then remove only the lines shown in 3. Check the rc's syntax (`zsh -n` / `bash -n` / `fish -n`); if it fails, restore the backup and stop
2. **Settings and grimoire**: `mv` to move aside, `rm -rf` to delete; only the paths confirmed in 1
3. **Skills and symlinks**: symlinks first, then the skills. **Remove this skill (mahojin-teardown) last**
4. **Binary**: `cargo uninstall mahojin` for `~/.cargo/bin/mahojin`; otherwise remove that file. If another tool such as Homebrew installed it, leave it and report where it is

### 5. Report

- Lines removed (file and content) and where the backups are
- Where things were moved, or the paths deleted
- Skills, symlinks and binary removed
- What was left (not chosen, or lines left because they were unclear)
- The current shell still has mahojin's functions: `exec zsh` (`exec bash`, `exec fish`) to reopen it
- If moved aside, how to restore: `mv ~/mahojin-backup-<YYYYMMDD>/config ~/.config/mahojin` and `mv ~/mahojin-backup-<YYYYMMDD>/data ~/.local/share/mahojin`
