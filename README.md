# 🕰️ git-time-machine

> **Undo DISASTROUS git mistakes in 3 seconds with a beautiful TUI**

Never lose work again. `git-time-machine` makes git reflog visual, interactive, and actually usable.

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/git-time-machine.svg)](https://crates.io/crates/git-time-machine)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

### [🌐 View Landing Page](https://dinakars777.github.io/git-time-machine/)

</div>

## ✨ The Problem

You just:
- 💥 Accidentally moved HEAD and lost track of your commits
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

### Scenario 1: "I Don't Know What I Did, But It's Broken"
```bash
# You ran a bunch of git commands, something broke
# You don't remember the exact sequence
# git reflog shows 50+ cryptic entries

# With git-time-machine:
git-time-machine
# Visually scan the timeline
# See "2h ago - before I started messing around"
# Press Enter, back to working state ✨
```

**Why not `git reflog`?** You'd need to:
1. Read through cryptic hashes and messages
2. Guess which one is "before you broke it"
3. Manually `git reset --hard <hash>`
4. Hope you picked the right one
5. Repeat if wrong

### Scenario 2: Recovering Lost Work After Complex Operations
```bash
# You did: rebase, merge, reset, amend, rebase again
# Now you need code from 3 operations ago
# But you don't remember the exact hash

# With git-time-machine:
git-time-machine
# Scroll through visual timeline
# See relative timestamps: "15m ago", "1h ago"
# Find the state you need
# Press Enter to restore
```

**Why not `git reflog | grep`?** You'd need to:
1. Know what to grep for
2. Parse timestamps manually
3. Cross-reference multiple entries
4. Still guess which hash is correct

### Scenario 3: Accidental Branch Deletion
```bash
# Deleted a branch recently but forgot the commit hash
# git branch -D feature-branch

# With git-time-machine:
git-time-machine --all
# Scroll back through your history
# Find "checkout: moving from feature-branch"
# Copy the short hash displayed in the UI
# Hit 'q' to exit, then run: git branch feature-branch <hash>
```

**Why not `git reflog --all`?** You'd need to:
1. Scroll through hundreds of lines of text
2. Find the right branch name in the noise
3. Extract the hash manually
4. Remember the git commands to restore it

### Scenario 4: "Undo" After You've Already Committed
```bash
# You committed to the wrong branch
# Then made 3 more commits
# Then realized the mistake
# git revert won't help - you need to go back in time

# With git-time-machine:
git-time-machine
# Find "before I committed to wrong branch"
# Press Enter
# Cherry-pick the commits to the right branch
```

**Why not `git reset`?** You'd need to:
1. Count how many commits back
2. Remember if it's `--soft`, `--mixed`, or `--hard`
3. Hope you counted right
4. Manually re-apply commits if you messed up

## ⚠️ What Git-Time-Machine Cannot Fix

- **Uncommitted Changes:** If you haven't committed or stashed your work, the reflog cannot see it. Accidentally running `git clean -fd` or wiping uncommitted files via `git reset --hard` is permanent.
- **Garbage Collected Commits:** By default, git periodically cleans up unreachable commits. If a commit has been permanently garbage-collected, it is gone forever.

## 🛠️ How It Works

`git-time-machine` is a wrapper around `git reflog` that:

1. Parses your reflog history
2. Displays it in an interactive TUI
3. Lets you preview and restore any state
4. Executes `git reset --hard <hash>` when you press Enter

**It's just git under the hood** - no magic, no risk.

## 🤔 Why Not Just Use Git Commands?

**You absolutely can!** But here's the reality:

| Task | With git commands | With git-time-machine |
|------|------------------|---------------------|
| Find state from "before I broke it" | `git reflog`, scan 50+ lines, guess hash, `git reset --hard <hash>`, hope it's right | Scroll, press Enter |
| Recover a deleted branch's commit hash | `git reflog --all \| grep branch-name`, find hash, `git checkout -b`, verify | `--all` flag, scroll, copy hash |
| Undo complex operation sequence | Remember exact commands, count commits, pick right reset flag | Visual timeline, click the "before" state |
| Explore "what if" scenarios | Multiple `git reset` attempts, risk losing more work | Navigate freely, restore is one keypress |

**git-time-machine doesn't replace git** - it makes reflog actually usable for humans who:
- Don't memorize commit hashes
- Don't want to grep through 200 lines of text
- Want to see their history visually
- Need to undo mistakes quickly without googling

Think of it as `git reflog` with a UI that doesn't require a PhD.

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
