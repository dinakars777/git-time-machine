# Changelog

## [0.2.3] - 2026-03-24

### Added
- Scrollable diff pane - Use Shift+↑/↓ to scroll through large diffs
- Terminal panic recovery - Terminal is properly restored even if the app panics
- Success feedback - Shows confirmation message after successful restore

### Fixed
- Commit messages containing `|` character now parse correctly (switched to null byte delimiter)
- Diff scroll resets to top when navigating to a different commit

## [0.2.2] - 2026-03-24

(Skipped version)

## [0.2.1] - 2026-03-23

### Added
- [y/N] confirmation dialog before destructive reset
- Diff preview pane with Space key (shows `git diff --stat`)
- Uncommitted changes warning in header
- Home/End/PgUp/PgDn keyboard shortcuts
- Conditional confirmation message based on uncommitted changes

### Fixed
- Command injection prevention with hash validation
- Fixed repo_path usage in all git commands
- Removed wrap-around navigation (now clamps at top/bottom)
- Removed duplicate selected_index state
- Fixed string clone in UI rendering

## [0.2.0] - 2026-03-22

### Added
- Initial release with TUI interface
- Reflog navigation with vim-style keybindings
- Visual timeline of git history
- One-key restore to any commit
