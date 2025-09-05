use anyhow::{Context, Result};
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
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::{
    fs::File,
    io::{self, BufReader},
    path::PathBuf,
    sync::Arc,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to keypair file (defaults to ~/.config/solana/id.json)
    #[arg(short, long)]
    keypair: Option<PathBuf>,

    /// RPC URL (defaults to mainnet)
    #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,
}

#[derive(Debug)]
enum AppState {
    Home,
    Wallet,
    Transactions,
    Settings,
}

struct WalletInfo {
    address: Pubkey,
    balance: f64,
}

struct App {
    state: AppState,
    selected_menu_item: usize,
    wallet: Option<WalletInfo>,
    rpc_client: Arc<RpcClient>,
    rpc_url: String,
}

impl App {
    fn new(wallet: Option<WalletInfo>, rpc_client: Arc<RpcClient>, rpc_url: String) -> Self {
        Self {
            state: AppState::Home,
            selected_menu_item: 0,
            wallet,
            rpc_client,
            rpc_url,
        }
    }
    
    async fn refresh_balance(&mut self) -> Result<()> {
        if let Some(ref mut wallet_info) = self.wallet {
            let balance = self.rpc_client
                .get_balance(&wallet_info.address)
                .context("Failed to fetch balance")?;
            wallet_info.balance = balance as f64 / 1_000_000_000.0; // Convert lamports to SOL
        }
        Ok(())
    }
}

fn load_keypair(path: &PathBuf) -> Result<Keypair> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open keypair file: {}", path.display()))?;
    let reader = BufReader::new(file);
    let keypair_bytes: Vec<u8> = serde_json::from_reader(reader)
        .with_context(|| format!("Failed to parse keypair file: {}", path.display()))?;
    
    Keypair::try_from(&keypair_bytes[..])
        .with_context(|| format!("Invalid keypair in file: {}", path.display()))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Determine keypair path
    let keypair_path = if let Some(path) = args.keypair {
        path
    } else {
        // Use default Solana CLI path
        let mut default_path = dirs::home_dir()
            .context("Could not find home directory")?;
        default_path.push(".config");
        default_path.push("solana");
        default_path.push("id.json");
        default_path
    };
    
    // Try to load the keypair
    let wallet_info = match load_keypair(&keypair_path) {
        Ok(keypair) => {
            let address = keypair.pubkey();
            eprintln!("Loaded wallet: {}", address);
            Some(WalletInfo {
                address,
                balance: 0.0,
            })
        }
        Err(e) => {
            eprintln!("Warning: Could not load keypair: {}", e);
            eprintln!("Running in read-only mode.");
            None
        }
    };
    
    // Create RPC client
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        args.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    ));
    
    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(wallet_info, rpc_client, args.rpc_url);
    
    // Get initial balance if wallet is loaded
    if app.wallet.is_some() {
        let _ = app.refresh_balance().await;
    }
    
    let res = run_app(&mut terminal, app).await;

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

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('r') if matches!(app.state, AppState::Wallet) => {
                    // Refresh balance
                    let _ = app.refresh_balance().await;
                }
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
                        1 => {
                            // Refresh balance when entering wallet view
                            let _ = app.refresh_balance().await;
                            AppState::Wallet
                        },
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
        AppState::Wallet => render_wallet(&app),
        AppState::Transactions => render_transactions(),
        AppState::Settings => render_settings(&app),
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

fn render_wallet(app: &App) -> Paragraph<'static> {
    let lines = if let Some(ref wallet) = app.wallet {
        vec![
            Line::from("Wallet Overview"),
            Line::from(""),
            Line::from(format!("Address: {}", wallet.address)),
            Line::from(format!("Balance: {:.9} SOL", wallet.balance)),
            Line::from(""),
            Line::from("Press 'r' to refresh balance"),
        ]
    } else {
        vec![
            Line::from("Wallet Overview"),
            Line::from(""),
            Line::from("No wallet loaded"),
            Line::from(""),
            Line::from("Run with --keypair <path> to load a wallet"),
            Line::from("Or create ~/.config/solana/id.json"),
        ]
    };
    
    Paragraph::new(lines)
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

fn render_settings(app: &App) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from("Settings"),
        Line::from(""),
        Line::from(format!("RPC Endpoint: {}", app.rpc_url)),
        Line::from("Network: Mainnet Beta"),
        Line::from(""),
        if app.wallet.is_some() {
            Line::from("Wallet: Loaded ✓")
        } else {
            Line::from("Wallet: Not loaded")
        },
    ])
    .style(Style::default().fg(Color::Magenta))
    .block(Block::default().borders(Borders::ALL).title("Settings"))
}