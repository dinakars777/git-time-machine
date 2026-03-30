use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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
use std::io;

mod git;
use git::{GitEntry, GitManager};

#[derive(Parser)]
#[command(name = "git-time-machine")]
#[command(about = "🕰️  Undo ANY git mistake in 3 seconds", long_about = None)]
#[command(after_help = "EXAMPLES:\n  \
    git-time-machine              # Show last 50 reflog entries\n  \
    git-time-machine --all        # Show all reflog entries\n\n\
CONTROLS:\n  \
    ↑/k, ↓/j    Navigate up/down\n  \
    Home/End    Jump to first/last entry\n  \
    PgUp/PgDn   Jump 10 entries\n  \
    Space       Toggle diff preview\n  \
    Enter       Restore to selected commit\n  \
    q/Esc       Quit")]
struct Cli {
    /// Show all reflog entries (max 1000, default: last 50)
    #[arg(short, long)]
    all: bool,
}

struct App {
    git_manager: GitManager,
    entries: Vec<GitEntry>,
    list_state: ListState,
    show_confirmation: bool,
    show_diff: bool,
    diff_content: String,
    diff_scroll_offset: u16,
    diff_visible_height: u16,
    has_uncommitted_changes: bool,
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

        Ok(Self {
            git_manager,
            entries,
            list_state,
            show_confirmation: false,
            show_diff: false,
            diff_content: String::new(),
            diff_scroll_offset: 0,
            diff_visible_height: 10,
            has_uncommitted_changes,
        })
    }

    fn selected_index(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    fn update_diff_if_visible(&mut self) -> Result<()> {
        if self.show_diff {
            let idx = self.selected_index();
            if let Some(entry) = self.entries.get(idx) {
                self.diff_content = self.git_manager.get_diff_stat(&entry.hash)?;
                self.diff_scroll_offset = 0;
            }
        }
        Ok(())
    }

    fn next(&mut self) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.entries.len() - 1 {
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
        if self.entries.is_empty() {
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
            let idx = self.selected_index();
            if let Some(entry) = self.entries.get(idx) {
                self.diff_content = self.git_manager.get_diff_stat(&entry.hash)?;
            }
        }
        self.diff_scroll_offset = 0;
        Ok(())
    }

    fn scroll_diff_up(&mut self) {
        self.diff_scroll_offset = self.diff_scroll_offset.saturating_sub(1);
    }

    fn scroll_diff_down(&mut self) {
        let line_count = self.diff_content.lines().count() as u16;
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
        let idx = self.list_state.selected().unwrap_or(0);
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
    
    // Setup panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new(cli.all)?;
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
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
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
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
                        if !app.entries.is_empty() {
                            app.list_state.select(Some(0));
                            app.update_diff_if_visible()?;
                        }
                    }
                    KeyCode::End => {
                        if !app.entries.is_empty() {
                            let last = app.entries.len() - 1;
                            app.list_state.select(Some(last));
                            app.update_diff_if_visible()?;
                        }
                    }
                    KeyCode::PageDown => {
                        if !app.entries.is_empty() {
                            let current = app.list_state.selected().unwrap_or(0);
                            let next = (current + 10).min(app.entries.len() - 1);
                            app.list_state.select(Some(next));
                            app.update_diff_if_visible()?;
                        }
                    }
                    KeyCode::PageUp => {
                        if !app.entries.is_empty() {
                            let current = app.list_state.selected().unwrap_or(0);
                            let prev = current.saturating_sub(10);
                            app.list_state.select(Some(prev));
                            app.update_diff_if_visible()?;
                        }
                    }
                    KeyCode::Char(' ') => {
                        app.toggle_diff()?;
                    }
                    KeyCode::Enter => {
                        app.show_confirmation_dialog();
                    }
                    _ => {}
                }
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
    let items: Vec<ListItem> = app
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let selected_idx = app.selected_index();
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

            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&entry.relative_time, time_style),
                Span::raw("  "),
                Span::styled(&entry.hash[..7], Style::default().fg(Color::Yellow)),
                Span::raw("  "),
                Span::styled(&entry.message, style),
            ]))
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
        app.diff_visible_height = diff_area.height.saturating_sub(2); // Subtract borders
        
        let diff_text = if app.diff_content.is_empty() {
            "Loading diff..."
        } else {
            &app.diff_content
        };

        let diff = Paragraph::new(diff_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Diff Preview (Shift+↑↓ or J/K to scroll) ")
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().fg(Color::White))
            .scroll((app.diff_scroll_offset, 0))
            .wrap(ratatui::widgets::Wrap { trim: false });

        f.render_widget(diff, diff_area);
    }

    // Footer with preview or confirmation dialog
    if app.show_confirmation {
        let selected_idx = app.selected_index();
        let confirm_text = if let Some(entry) = app.entries.get(selected_idx) {
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
    } else {
        let selected_idx = app.selected_index();
        let footer_text = if let Some(entry) = app.entries.get(selected_idx) {
            format!("📍 Will restore to: {} - {} | Press Space for diff preview", &entry.hash[..7], entry.message)
        } else {
            "No entries found".to_string()
        };

        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Green))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
        f.render_widget(footer, chunks[2]);
    }
}
