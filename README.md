# 🕰️ git-time-machine

> **Undo ANY git mistake in 3 seconds with a beautiful TUI**

Never lose work again. `git-time-machine` makes git reflog visual, interactive, and actually usable.

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/git-time-machine.svg)](https://crates.io/crates/git-time-machine)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

### [🌐 View Landing Page](https://dinakars777.github.io/git-time-machine/)

</div>

## ✨ The Problem

You just:
- 🔥 Force pushed and lost commits
- 💥 Did `git reset --hard` by accident  
- 🗑️ Deleted a branch you needed
- 🤦 Rebased wrong and broke everything
- 😱 Can't remember what you did 5 minutes ago

**Current solution:** Dig through `git reflog`, copy cryptic hashes, pray you picked the right one.

**Better solution:** `git-time-machine` 🎯

## 🚀 Demo

![Demo GIF](demo.gif)

*Navigate your git history like a time traveler. One key to restore.*

## 📦 Installation

### Cargo (Recommended)
```bash
cargo install git-time-machine
```

### From Source
```bash
git clone https://github.com/dinakars777/git-time-machine
cd git-time-machine
cargo install --path .
```

### Homebrew (Coming Soon)
```bash
brew install git-time-machine
```

## 🎮 Usage

```bash
# Launch in any git repository
git-time-machine

# Show all reflog entries (default: last 50)
git-time-machine --all
```

### Controls

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Enter` | Restore to selected state |
| `q` / `Esc` | Quit |

## 🎯 Features

- ✅ **Visual Timeline** - See your entire git history at a glance
- ✅ **Relative Timestamps** - "5m ago", "2h ago", "yesterday"
- ✅ **One-Key Restore** - Press Enter, done
- ✅ **Vim Keybindings** - j/k navigation
- ✅ **Beautiful TUI** - Built with Ratatui
- ✅ **Lightning Fast** - Written in Rust
- ✅ **Zero Config** - Just works™

## 🔥 Use Cases

### Scenario 1: Accidental Reset
```bash
# Oh no! You did this:
git reset --hard HEAD~5

# No problem:
git-time-machine
# Navigate to "6m ago", press Enter
# All commits restored ✨
```

### Scenario 2: Deleted Branch
```bash
# Deleted the wrong branch
git branch -D feature-branch

# Recover it:
git-time-machine
# Find the last commit on that branch
# Press Enter, then:
git checkout -b feature-branch
```

### Scenario 3: Bad Rebase
```bash
# Rebase went wrong
git rebase main
# Conflicts everywhere...

# Undo it:
git-time-machine
# Go back to before rebase
# Press Enter, start over
```

## 🛠️ How It Works

`git-time-machine` is a wrapper around `git reflog` that:

1. Parses your reflog history
2. Displays it in an interactive TUI
3. Lets you preview and restore any state
4. Executes `git reset --hard <hash>` when you press Enter

**It's just git under the hood** - no magic, no risk.

## 🤝 Contributing

Contributions welcome!

**Ideas for future versions:**
- [ ] Show file diffs inline
- [ ] "Panic mode" - undo last N minutes
- [ ] Branch visualization
- [ ] Stash recovery
- [ ] Search/filter commits
- [ ] Export timeline as JSON

## 📝 License

MIT © [Dinakar Sarbada](https://github.com/dinakars777)

## 🌟 Star History

If this saved you once, give it a star! ⭐

---

**Made with ❤️ and Rust** | [Report Bug](https://github.com/dinakars777/git-time-machine/issues) | [Request Feature](https://github.com/dinakars777/git-time-machine/issues)
