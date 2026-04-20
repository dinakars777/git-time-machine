use anyhow::Result;
use serde_json;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use chrono::Local;
use std::io;

mod git;
use git::{GitEntry, GitManager};

#[derive(Parser)]
#[command(name = "git-time-machine")]
#[command(about = "🕰️  Undo DISASTROUS git mistakes in 3 seconds", long_about = None)]
#[command(after_help = "EXAMPLES:\n  \
    git-time-machine              # Show last 50 reflog entries\n  \
    git-time-machine --all        # Show all reflog entries\n  \
    git-time-machine --export-json # Export as JSON for automation\n\n\
CONTROLS:\n  \
    ↑/k, ↓/j    Navigate up/down\n  \
    Home/End    Jump to first/last entry\n  \
    gg/G        Jump to first/last entry (vim-style)\n  \
    PgUp/PgDn   Jump 10 entries\n  \
    Space       Toggle diff panel\n  \
    d           Switch between diff summary and full diff\n  \
    t           Toggle relative/absolute timestamps\n  \
    Enter       Restore to selected commit\n  \
    /           Search/filter commits by message\n  \
    Esc         Clear active filter (or quit if no filter)\n  \
    q           Quit\n\n\
SEARCH MODE:\n  \
    type        Filter commits (case-insensitive, multi-word AND)\n  \
    Enter       Apply filter and return to navigation\n  \
    Esc         Cancel search and clear filter\n  \
    Backspace   Delete last character")]
struct Cli {
    /// Show all reflog entries (max 1000, default: last 50)
    #[arg(short, long)]
    all: bool,

    /// Export reflog timeline as JSON
    #[arg(long)]
    export_json: bool,
}

struct App {
    git_manager: GitManager,
    entries: Vec<GitEntry>,
    list_state: ListState,
    show_confirmation: bool,
    show_diff: bool,
    show_full_diff: bool,
    diff_content: String,
    full_diff_content: String,
    diff_scroll_offset: u16,
    diff_visible_height: u16,
    has_uncommitted_changes: bool,
    search_mode: bool,
    search_query: String,
    filtered_entries: Vec<usize>,
    search_active: bool,
    show_absolute_time: bool,
    last_key_was_g: bool,
}

impl App {
    fn new(show_all: bool) -> Result<Self> {
        let git_manager = GitManager::new()?;
        let entries = git_manager.get_reflog_entries(show_all)?;
        let has_uncommitted_changes = git_manager.has_uncommitted_changes()?;
        
        let mut list_state = ListState::default();
        if !entries.is_empty() {
            list_state.select(Some(0));
        }

        let filtered_entries = (0..entries.len()).collect();

        Ok(Self {
            git_manager,
            entries,
            list_state,
            show_confirmation: false,
            show_diff: false,
            show_full_diff: false,
            diff_content: String::new(),
            full_diff_content: String::new(),
            diff_scroll_offset: 0,
            diff_visible_height: 10,
            has_uncommitted_changes,
            search_mode: false,
            search_query: String::new(),
            filtered_entries,
            search_active: false,
            show_absolute_time: false,
            last_key_was_g: false,
        })
    }

    fn selected_index(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    fn selected_entry_idx(&self) -> Option<usize> {
        let sel = self.list_state.selected()?;
        self.filtered_entries.get(sel).copied()
    }

    fn update_filter(&mut self) {
        let query_lower = self.search_query.to_lowercase();
        let tokens: Vec<&str> = query_lower.split_whitespace().collect();
        if tokens.is_empty() {
            self.filtered_entries = (0..self.entries.len()).collect();
        } else {
            self.filtered_entries = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    let msg = e.message.to_lowercase();
                    tokens.iter().all(|t| msg.contains(t))
                })
                .map(|(i, _)| i)
                .collect();
        }
        if self.filtered_entries.is_empty() {
            self.list_state.select(None);
        } else {
            let sel = self.list_state.selected().unwrap_or(0);
            if sel >= self.filtered_entries.len() {
                self.list_state.select(Some(self.filtered_entries.len() - 1));
            } else if self.list_state.selected().is_none() {
                self.list_state.select(Some(0));
            }
        }
    }

    fn clear_filter(&mut self) {
        self.search_query.clear();
        self.search_active = false;
        self.search_mode = false;
        self.filtered_entries = (0..self.entries.len()).collect();
        if !self.entries.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    fn update_diff_if_visible(&mut self) -> Result<()> {
        if let Some(idx) = self.selected_entry_idx() {
            if let Some(entry) = self.entries.get(idx) {
                if self.show_diff {
                    self.diff_content = self.git_manager.get_diff_stat(&entry.hash)?;
                }
                if self.show_full_diff {
                    self.full_diff_content = self.git_manager.get_full_diff(&entry.hash)?;
                }
                self.diff_scroll_offset = 0;
            }
        }
        Ok(())
    }

    fn next(&mut self) -> Result<()> {
        if self.filtered_entries.is_empty() {
            return Ok(());
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.filtered_entries.len() - 1 {
                    i // Clamp at bottom instead of wrap-around
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.update_diff_if_visible()?;
        Ok(())
    }

    fn previous(&mut self) -> Result<()> {
        if self.filtered_entries.is_empty() {
            return Ok(());
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0 // Clamp at top instead of wrap-around
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.update_diff_if_visible()?;
        Ok(())
    }

    fn toggle_diff(&mut self) -> Result<()> {
        self.show_diff = !self.show_diff;
        if self.show_diff {
            self.show_full_diff = false;
            if let Some(idx) = self.selected_entry_idx() {
                if let Some(entry) = self.entries.get(idx) {
                    self.diff_content = self.git_manager.get_diff_stat(&entry.hash)?;
                }
            }
        }
        self.diff_scroll_offset = 0;
        Ok(())
    }

    fn toggle_diff_mode(&mut self) -> Result<()> {
        if !self.show_diff {
            return Ok(());
        }
        self.show_full_diff = !self.show_full_diff;
        if self.show_full_diff {
            if let Some(idx) = self.selected_entry_idx() {
                if let Some(entry) = self.entries.get(idx) {
                    self.full_diff_content = self.git_manager.get_full_diff(&entry.hash)?;
                }
            }
        }
        self.diff_scroll_offset = 0;
        Ok(())
    }

    fn scroll_diff_up(&mut self) {
        self.diff_scroll_offset = self.diff_scroll_offset.saturating_sub(1);
    }

    fn active_diff_content(&self) -> &str {
        if self.show_full_diff {
            &self.full_diff_content
        } else {
            &self.diff_content
        }
    }

    fn scroll_diff_down(&mut self) {
        let line_count = self.active_diff_content().lines().count() as u16;
        let max_scroll = line_count.saturating_sub(self.diff_visible_height);
        self.diff_scroll_offset = (self.diff_scroll_offset + 1).min(max_scroll);
    }

    fn show_confirmation_dialog(&mut self) {
        self.show_confirmation = true;
    }

    fn cancel_confirmation(&mut self) {
        self.show_confirmation = false;
    }

    fn restore_selected(&self) -> Result<Option<(String, String)>> {
        let Some(idx) = self.selected_entry_idx() else {
            return Ok(None);
        };
        if let Some(entry) = self.entries.get(idx) {
            self.git_manager.restore_to_commit(&entry.hash)?;
            Ok(Some((entry.hash[..7].to_string(), entry.message.clone())))
        } else {
            Ok(None)
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    if cli.export_json {
        let git_manager = GitManager::new()?;
        let entries = git_manager.get_reflog_entries(cli.all)?;
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    // Setup panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new(cli.all)?;
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    match res {
        Ok(Some((hash, message))) => {
            println!("✅ Restored to {} - {}", hash, message);
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(err) => {
            println!("Error: {:?}", err);
            Err(err)
        }
    }
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<Option<(String, String)>> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            // Only handle key press events, ignore key release to prevent double-triggering on Windows
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if app.search_mode {
                match key.code {
                    KeyCode::Esc => {
                        app.search_mode = false;
                        app.search_query.clear();
                        app.search_active = false;
                        app.update_filter();
                        if !app.filtered_entries.is_empty() {
                            app.list_state.select(Some(0));
                        }
                        app.update_diff_if_visible()?;
                    }
                    KeyCode::Enter => {
                        app.search_mode = false;
                        app.search_active = !app.search_query.is_empty();
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                        app.update_filter();
                        app.update_diff_if_visible()?;
                    }
                    KeyCode::Char(c) => {
                        app.search_query.push(c);
                        app.update_filter();
                        app.update_diff_if_visible()?;
                    }
                    _ => {}
                }
                continue;
            }

            if app.show_confirmation {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        return app.restore_selected();
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        app.cancel_confirmation();
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => return Ok(None),
                    KeyCode::Esc => {
                        if app.search_active {
                            app.clear_filter();
                            app.update_diff_if_visible()?;
                        } else {
                            return Ok(None);
                        }
                    }
                    KeyCode::Char('/') => {
                        app.search_mode = true;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.show_diff && key.modifiers.contains(event::KeyModifiers::SHIFT) {
                            app.scroll_diff_down();
                        } else {
                            app.next()?;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.show_diff && key.modifiers.contains(event::KeyModifiers::SHIFT) {
                            app.scroll_diff_up();
                        } else {
                            app.previous()?;
                        }
                    }
                    KeyCode::Char('J') => {
                        if app.show_diff {
                            app.scroll_diff_down();
                        }
                    }
                    KeyCode::Char('K') => {
                        if app.show_diff {
                            app.scroll_diff_up();
                        }
                    }
                    KeyCode::Home => {
                        if !app.filtered_entries.is_empty() {
                            app.list_state.select(Some(0));
                            app.update_diff_if_visible()?;
                        }
                    }
                    KeyCode::End => {
                        if !app.filtered_entries.is_empty() {
                            let last = app.filtered_entries.len() - 1;
                            app.list_state.select(Some(last));
                            app.update_diff_if_visible()?;
                        }
                    }
                    KeyCode::Char('g') => {
                        if app.last_key_was_g && !app.filtered_entries.is_empty() {
                            app.list_state.select(Some(0));
                            app.update_diff_if_visible()?;
                            app.last_key_was_g = false;
                        } else {
                            app.last_key_was_g = true;
                        }
                        continue;
                    }
                    KeyCode::Char('G') => {
                        if !app.filtered_entries.is_empty() {
                            let last = app.filtered_entries.len() - 1;
                            app.list_state.select(Some(last));
                            app.update_diff_if_visible()?;
                        }
                    }
                    KeyCode::PageDown => {
                        if !app.filtered_entries.is_empty() {
                            let current = app.list_state.selected().unwrap_or(0);
                            let next = (current + 10).min(app.filtered_entries.len() - 1);
                            app.list_state.select(Some(next));
                            app.update_diff_if_visible()?;
                        }
                    }
                    KeyCode::PageUp => {
                        if !app.filtered_entries.is_empty() {
                            let current = app.list_state.selected().unwrap_or(0);
                            let prev = current.saturating_sub(10);
                            app.list_state.select(Some(prev));
                            app.update_diff_if_visible()?;
                        }
                    }
                    KeyCode::Char(' ') => {
                        app.toggle_diff()?;
                    }
                    KeyCode::Char('d') => {
                        app.toggle_diff_mode()?;
                    }
                    KeyCode::Char('t') => {
                        app.show_absolute_time = !app.show_absolute_time;
                    }
                    KeyCode::Enter => {
                        if app.selected_entry_idx().is_some() {
                            app.show_confirmation_dialog();
                        }
                    }
                    _ => {}
                }
                app.last_key_was_g = false;
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header with warning if uncommitted changes
    let header_text = if app.has_uncommitted_changes {
        vec![
            Line::from(vec![
                Span::styled("⚠️  ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled("UNCOMMITTED CHANGES WILL BE LOST", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw("  |  "),
                Span::styled("Navigate: ↑↓/jk", Style::default().fg(Color::Gray)),
                Span::raw("  |  "),
                Span::styled("Diff: Space", Style::default().fg(Color::Cyan)),
                Span::raw("  |  "),
                Span::styled("Restore: Enter", Style::default().fg(Color::Green)),
                Span::raw("  |  "),
                Span::styled("Quit: q", Style::default().fg(Color::Red)),
            ])
        ]
    } else {
        vec![Line::from(vec![
            Span::styled("🕰️  ", Style::default().fg(Color::Cyan)),
            Span::styled("Git Time Machine", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  |  "),
            Span::styled("Navigate: ↑↓/jk", Style::default().fg(Color::Gray)),
            Span::raw("  |  "),
            Span::styled("Diff: Space", Style::default().fg(Color::Cyan)),
            Span::raw("  |  "),
            Span::styled("Restore: Enter", Style::default().fg(Color::Green)),
            Span::raw("  |  "),
            Span::styled("Quit: q", Style::default().fg(Color::Red)),
        ])]
    };

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).border_style(
            if app.has_uncommitted_changes {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Cyan)
            }
        ));
    f.render_widget(header, chunks[0]);

    // Main content area - split if showing diff
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if app.show_diff {
            vec![Constraint::Percentage(50), Constraint::Percentage(50)]
        } else {
            vec![Constraint::Percentage(100)]
        })
        .split(chunks[1]);

    // Timeline list
    let selected_idx = app.selected_index();
    let query_lower = app.search_query.to_lowercase();
    let query_tokens: Vec<String> = query_lower
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let highlight_query = (app.search_active || app.search_mode) && !query_tokens.is_empty();
    let items: Vec<ListItem> = app
        .filtered_entries
        .iter()
        .enumerate()
        .filter_map(|(i, &entry_idx)| {
            let entry = app.entries.get(entry_idx)?;
            let is_selected = i == selected_idx;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let time_style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let mut spans = vec![
                Span::styled(prefix, style),
                Span::styled(
                    if app.show_absolute_time {
                        entry.timestamp.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()
                    } else {
                        entry.relative_time.clone()
                    },
                    time_style,
                ),
                Span::raw("  "),
                Span::styled(&entry.hash[..7], Style::default().fg(Color::Yellow)),
                Span::raw("  "),
            ];

            if highlight_query {
                let msg = &entry.message;
                let msg_lower = msg.to_lowercase();
                let highlight_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
                // Collect all match ranges from all tokens, then merge overlaps
                let mut ranges: Vec<(usize, usize)> = Vec::new();
                for token in &query_tokens {
                    let mut start = 0;
                    while let Some(pos) = msg_lower[start..].find(token.as_str()) {
                        let abs = start + pos;
                        ranges.push((abs, abs + token.len()));
                        start = abs + token.len();
                    }
                }
                ranges.sort_by_key(|r| r.0);
                let mut merged: Vec<(usize, usize)> = Vec::new();
                for r in ranges {
                    if let Some(last) = merged.last_mut() {
                        if r.0 <= last.1 {
                            last.1 = last.1.max(r.1);
                            continue;
                        }
                    }
                    merged.push(r);
                }
                let mut cursor = 0;
                for (s, e) in merged {
                    if s > cursor {
                        spans.push(Span::styled(msg[cursor..s].to_string(), style));
                    }
                    spans.push(Span::styled(msg[s..e].to_string(), highlight_style));
                    cursor = e;
                }
                if cursor < msg.len() {
                    spans.push(Span::styled(msg[cursor..].to_string(), style));
                }
            } else {
                spans.push(Span::styled(&entry.message, style));
            }

            Some(ListItem::new(Line::from(spans)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Timeline (newest first) ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(list, main_chunks[0], &mut app.list_state);

    // Diff preview pane
    if app.show_diff {
        let diff_area = main_chunks[1];
        app.diff_visible_height = diff_area.height.saturating_sub(2);

        if app.show_full_diff {
            let lines: Vec<Line> = if app.full_diff_content.is_empty() {
                vec![Line::raw("Loading diff...")]
            } else {
                app.full_diff_content
                    .lines()
                    .map(|line| {
                        if line.starts_with('+') && !line.starts_with("+++") {
                            Line::styled(line, Style::default().fg(Color::Green))
                        } else if line.starts_with('-') && !line.starts_with("---") {
                            Line::styled(line, Style::default().fg(Color::Red))
                        } else if line.starts_with("@@") {
                            Line::styled(line, Style::default().fg(Color::Cyan))
                        } else if line.starts_with("diff ") || line.starts_with("index ") {
                            Line::styled(line, Style::default().fg(Color::Yellow))
                        } else {
                            Line::raw(line)
                        }
                    })
                    .collect()
            };

            let diff = Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Full Diff (d: back to summary | Shift+↑↓/J/K: scroll) ")
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .scroll((app.diff_scroll_offset, 0))
                .wrap(ratatui::widgets::Wrap { trim: false });

            f.render_widget(diff, diff_area);
        } else {
            let diff_text = if app.diff_content.is_empty() {
                "Loading diff..."
            } else {
                &app.diff_content
            };

            let diff = Paragraph::new(diff_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Diff Summary (d: full diff | Shift+↑↓/J/K: scroll) ")
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .style(Style::default().fg(Color::White))
                .scroll((app.diff_scroll_offset, 0))
                .wrap(ratatui::widgets::Wrap { trim: false });

            f.render_widget(diff, diff_area);
        }
    }

    // Footer with preview or confirmation dialog
    if app.show_confirmation {
        let confirm_text = if let Some(entry) = app
            .selected_entry_idx()
            .and_then(|i| app.entries.get(i))
        {
            if app.has_uncommitted_changes {
                format!(
                    "⚠️  CONFIRM: Reset to {} - {}? This will discard uncommitted changes! [y/N]",
                    &entry.hash[..7], entry.message
                )
            } else {
                format!(
                    "⚠️  CONFIRM: Reset to {} - {}? [y/N]",
                    &entry.hash[..7], entry.message
                )
            }
        } else {
            "No entry selected".to_string()
        };

        let footer = Paragraph::new(confirm_text)
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Red)));
        f.render_widget(footer, chunks[2]);
    } else if app.search_mode {
        let match_count = app.filtered_entries.len();
        let footer_line = Line::from(vec![
            Span::styled("🔍 Search: ", Style::default().fg(Color::Cyan)),
            Span::styled(app.search_query.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(Color::Yellow)),
            Span::raw(" "),
            Span::styled(format!("({} matches)", match_count), Style::default().fg(Color::Gray)),
            Span::raw("  |  "),
            Span::styled("Enter: apply", Style::default().fg(Color::Green)),
            Span::raw("  |  "),
            Span::styled("Esc: cancel", Style::default().fg(Color::Red)),
        ]);
        let footer = Paragraph::new(footer_line)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
        f.render_widget(footer, chunks[2]);
    } else if app.search_active {
        let match_count = app.filtered_entries.len();
        let text = format!(
            "🔍 Filtered: {} ({} matches) | / to edit | Esc to clear",
            app.search_query, match_count
        );
        let footer = Paragraph::new(text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
        f.render_widget(footer, chunks[2]);
    } else {
        let entry_idx = app.selected_entry_idx().unwrap_or(0);
        let footer_text = if let Some(entry) = app.entries.get(entry_idx) {
            format!("📍 Will restore to: {} - {} | / to search | Space for diff", &entry.hash[..7], entry.message)
        } else {
            "No entries found".to_string()
        };

        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Green))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
        f.render_widget(footer, chunks[2]);
    }
}
