use anyhow::Result;
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
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;

#[derive(Debug)]
enum AppState {
    Home,
    Wallet,
    Transactions,
    Settings,
}

struct App {
    state: AppState,
    selected_menu_item: usize,
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState::Home,
            selected_menu_item: 0,
        }
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new();
    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Up => {
                    if app.selected_menu_item > 0 {
                        app.selected_menu_item -= 1;
                    }
                }
                KeyCode::Down => {
                    if app.selected_menu_item < 3 {
                        app.selected_menu_item += 1;
                    }
                }
                KeyCode::Enter => {
                    app.state = match app.selected_menu_item {
                        0 => AppState::Home,
                        1 => AppState::Wallet,
                        2 => AppState::Transactions,
                        3 => AppState::Settings,
                        _ => AppState::Home,
                    };
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(f.area());

    let menu_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(chunks[0]);

    let title = Paragraph::new("🦊 FURRYBAIT")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, menu_chunks[0]);

    let menu_items = vec!["Home", "Wallet", "Transactions", "Settings"];
    let menu: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.selected_menu_item {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(*item, style)))
        })
        .collect();

    let menu_list = List::new(menu)
        .block(Block::default().borders(Borders::ALL).title("Menu"));
    f.render_widget(menu_list, menu_chunks[1]);

    let content = match app.state {
        AppState::Home => render_home(),
        AppState::Wallet => render_wallet(),
        AppState::Transactions => render_transactions(),
        AppState::Settings => render_settings(),
    };
    f.render_widget(content, chunks[1]);
}

fn render_home() -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from("Welcome to Furrybait! 🚀"),
        Line::from(""),
        Line::from("A Solana wallet with a terminal UI"),
        Line::from(""),
        Line::from("Navigate with ↑↓ arrows"),
        Line::from("Press Enter to select"),
        Line::from("Press 'q' to quit"),
    ])
    .style(Style::default().fg(Color::White))
    .block(Block::default().borders(Borders::ALL).title("Home"))
}

fn render_wallet() -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from("Wallet Overview"),
        Line::from(""),
        Line::from("Address: [Not connected]"),
        Line::from("Balance: 0 SOL"),
        Line::from(""),
        Line::from("Token accounts: 0"),
    ])
    .style(Style::default().fg(Color::Green))
    .block(Block::default().borders(Borders::ALL).title("Wallet"))
}

fn render_transactions() -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from("Recent Transactions"),
        Line::from(""),
        Line::from("No transactions yet"),
    ])
    .style(Style::default().fg(Color::Blue))
    .block(Block::default().borders(Borders::ALL).title("Transactions"))
}

fn render_settings() -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from("Settings"),
        Line::from(""),
        Line::from("RPC Endpoint: https://api.mainnet-beta.solana.com"),
        Line::from("Network: Mainnet"),
    ])
    .style(Style::default().fg(Color::Magenta))
    .block(Block::default().borders(Borders::ALL).title("Settings"))
}