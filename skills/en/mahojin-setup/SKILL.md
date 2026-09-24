---
name: mahojin-setup
description: Set mahojin up for the first time. Asks which shell to use it in, writes the `mahojin --init` line (chanting mode) and a short alias to that shell's rc, and inside tmux checks that images are passed through. Invoke with `/mahojin-setup`, or when asked to "set up mahojin" or "configure mahojin".
---

# mahojin-setup — first-time setup for mahojin

mahojin is a wrapper that unfolds a magic circle made for each command in the terminal before running it. `mahojin git status` works on its own, but chanting mode (`mahojin --on`) needs a `mahojin --init <shell>` line in the rc. This skill writes it.

**Ask with AskUserQuestion.** Don't decide for the user.
Read the rc before changing it, and don't add a line that is already there. If the rc is a symlink, edit the file it points to (check with `ls -l`).

## Steps

### 1. Is mahojin installed?

```
mahojin --version
```

If not, stop and point to `cargo install mahojin` (this skill doesn't install it).

### 2. Ask for the shell and aliases

Look at the login shell with `echo $SHELL` and make it the recommended option (first, with "(Recommended)" at the end of the label).

| Shell | rc | Line |
|---|---|---|
| zsh | `~/.zshrc` | `eval "$(mahojin --init zsh)"` |
| bash | `~/.bashrc` | `eval "$(mahojin --init bash)"` |
| fish | `~/.config/fish/config.fish` | `mahojin --init fish \| source` |

In the same AskUserQuestion, ask about aliases (multiSelect):

- "`m` (Recommended)": `alias m='mahojin'` (fish takes `alias` too)
- "Milestone aliases": `push` / `release` / `deploy` for `mahojin git push` / `mahojin git tag` / `mahojin make deploy`. **Skip any name already taken (check with `type push` and so on) and report it**
- "None"

### 3. Write the rc

If a line containing `mahojin --init` is already there, don't add another. Otherwise append:

```
# mahojin: mahojin --on unfolds a circle for every command you type
<the line from the table in 2>
alias m='mahojin'
```

Then check the syntax (`zsh -n ~/.zshrc`, `bash -n ~/.bashrc`, `fish -n ~/.config/fish/config.fish`).

### 4. Language

Look at `locale` in `mahojin --setup`. If it differs from the language of the conversation, ask, and save with `mahojin --setup --locale <ja|en>`.

### 5. tmux

Only when `$TMUX` is set:

```
tmux show -gv allow-passthrough
```

If it isn't `on`, ask whether to add these two lines to `~/.tmux.conf` (without them no image appears inside tmux). If added, run `tmux source-file ~/.tmux.conf`.

```
# mahojin: pass images through to the outer terminal
set -g allow-passthrough on
```

### 6. Report

- The rc you edited (the real path) and the lines you added; lines you didn't add, and why
- The language and the tmux setting
- The current shell doesn't have it yet: `exec zsh` (`exec bash`, `exec fish`) to reopen it
- Things to try: `mahojin git status`; `mahojin --on`, then type commands as usual (`mahojin --off` to stop); `mahojin --grimoire`
- The agent's shell is not a terminal, so casting here shows no circle. Try it in the user's terminal
- To undo all this: `/mahojin-teardown`
