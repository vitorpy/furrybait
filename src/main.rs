use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use qrcode::{render::unicode, QrCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::system_instruction;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use std::{
    fs::File,
    io::{self, BufReader},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to keypair file (defaults to ~/.config/solana/id.json)
    #[arg(short, long)]
    keypair: Option<PathBuf>,

    /// Cluster to connect to (mainnet/testnet/devnet or custom RPC URL)
    #[arg(short, long, default_value = "mainnet")]
    cluster: String,
}

#[derive(Debug, Clone)]
enum AppState {
    Home,
    Wallet,
    Send,
    Receive,
    Transactions,
    Settings,
}

#[derive(Debug, Clone)]
struct SendState {
    recipient: String,
    amount: String,
    input_mode: SendInputMode,
    status: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum SendInputMode {
    EditingRecipient,
    EditingAmount,
    Confirming,
}

impl Default for SendState {
    fn default() -> Self {
        Self {
            recipient: String::new(),
            amount: String::new(),
            input_mode: SendInputMode::EditingRecipient,
            status: None,
            error: None,
        }
    }
}

struct WalletInfo {
    keypair: Arc<Keypair>,
    address: Pubkey,
    balance: f64,
}

struct App {
    state: AppState,
    selected_menu_item: usize,
    wallet: WalletInfo,
    rpc_client: Arc<RpcClient>,
    rpc_url: String,
    send_state: SendState,
    last_tx_signature: Option<Signature>,
}

impl App {
    fn new(wallet: WalletInfo, rpc_client: Arc<RpcClient>, rpc_url: String) -> Self {
        Self {
            state: AppState::Home,
            selected_menu_item: 0,
            wallet,
            rpc_client,
            rpc_url,
            send_state: SendState::default(),
            last_tx_signature: None,
        }
    }

    async fn refresh_balance(&mut self) -> Result<()> {
        let balance = self
            .rpc_client
            .get_balance(&self.wallet.address)
            .context("Failed to fetch balance")?;
        self.wallet.balance = balance as f64 / LAMPORTS_PER_SOL as f64;
        Ok(())
    }

    async fn send_transaction(&mut self) -> Result<()> {
        let recipient =
            Pubkey::from_str(&self.send_state.recipient).context("Invalid recipient address")?;

        let amount = self
            .send_state
            .amount
            .parse::<f64>()
            .context("Invalid amount")?;

        let lamports = (amount * LAMPORTS_PER_SOL as f64) as u64;

        // Create transfer instruction
        let transfer_ix = system_instruction::transfer(&self.wallet.address, &recipient, lamports);

        // Get recent blockhash
        let recent_blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .context("Failed to get recent blockhash")?;

        // Build transaction
        let transaction = Transaction::new_signed_with_payer(
            &[transfer_ix],
            Some(&self.wallet.address),
            &[&*self.wallet.keypair],
            recent_blockhash,
        );

        // Send transaction
        let signature = self
            .rpc_client
            .send_and_confirm_transaction(&transaction)
            .context("Failed to send transaction")?;

        self.last_tx_signature = Some(signature);
        self.send_state.status = Some(format!("Transaction sent: {}", signature));

        // Refresh balance
        let _ = self.refresh_balance().await;

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

fn resolve_rpc_url(cluster: &str) -> String {
    // Check if cluster is a known preset or a custom URL
    match cluster.to_lowercase().as_str() {
        "mainnet" | "mainnet-beta" => "https://api.mainnet-beta.solana.com".to_string(),
        "testnet" => "https://api.testnet.solana.com".to_string(),
        "devnet" => "https://api.devnet.solana.com".to_string(),
        "localhost" | "localnet" => "http://localhost:8899".to_string(),
        // If not a preset, assume it's a custom RPC URL
        custom => {
            // Add https:// if no protocol is specified
            if custom.starts_with("http://") || custom.starts_with("https://") {
                custom.to_string()
            } else {
                format!("https://{}", custom)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Determine keypair path
    let keypair_path = if let Some(path) = args.keypair {
        path
    } else {
        // Use default Solana CLI path
        let mut default_path = dirs::home_dir().context("Could not find home directory")?;
        default_path.push(".config");
        default_path.push("solana");
        default_path.push("id.json");
        default_path
    };

    // Load the keypair (required)
    let keypair = load_keypair(&keypair_path).with_context(|| {
        format!(
            "Failed to load keypair from {}. 
Please ensure the file exists and contains a valid Solana keypair.
You can create one with: solana-keygen new -o {}",
            keypair_path.display(),
            keypair_path.display()
        )
    })?;

    let address = keypair.pubkey();
    eprintln!("Loaded wallet: {}", address);

    let wallet_info = WalletInfo {
        keypair: Arc::new(keypair),
        address,
        balance: 0.0,
    };

    // Resolve RPC URL from cluster
    let rpc_url = resolve_rpc_url(&args.cluster);
    eprintln!("Connecting to RPC: {}", rpc_url);

    // Create RPC client
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        rpc_url.clone(),
        CommitmentConfig::confirmed(),
    ));

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(wallet_info, rpc_client, rpc_url);

    // Get initial balance
    let _ = app.refresh_balance().await;

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
            // Handle Send state input
            if matches!(app.state, AppState::Send) {
                match handle_send_input(&mut app, key).await {
                    Ok(should_continue) => {
                        if !should_continue {
                            app.state = AppState::Wallet;
                            app.send_state = SendState::default();
                        }
                    }
                    Err(e) => {
                        app.send_state.error = Some(e.to_string());
                    }
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('r') if matches!(app.state, AppState::Wallet) => {
                    // Refresh balance
                    let _ = app.refresh_balance().await;
                }
                KeyCode::Esc if matches!(app.state, AppState::Receive) => {
                    app.state = AppState::Wallet;
                }
                KeyCode::Up => {
                    if app.selected_menu_item > 0 {
                        app.selected_menu_item -= 1;
                    }
                }
                KeyCode::Down => {
                    if app.selected_menu_item < 5 {
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
                        }
                        2 => {
                            app.send_state = SendState::default();
                            AppState::Send
                        }
                        3 => AppState::Receive,
                        4 => AppState::Transactions,
                        5 => AppState::Settings,
                        _ => AppState::Home,
                    };
                }
                _ => {}
            }
        }
    }
}

async fn handle_send_input(app: &mut App, key: KeyEvent) -> Result<bool> {
    match app.send_state.input_mode {
        SendInputMode::EditingRecipient => match key.code {
            KeyCode::Char(c) => {
                app.send_state.recipient.push(c);
            }
            KeyCode::Backspace => {
                app.send_state.recipient.pop();
            }
            KeyCode::Enter => {
                if !app.send_state.recipient.is_empty() {
                    app.send_state.input_mode = SendInputMode::EditingAmount;
                    app.send_state.error = None;
                }
            }
            KeyCode::Esc => return Ok(false),
            _ => {}
        },
        SendInputMode::EditingAmount => match key.code {
            KeyCode::Char(c) if c.is_digit(10) || c == '.' => {
                app.send_state.amount.push(c);
            }
            KeyCode::Backspace => {
                app.send_state.amount.pop();
            }
            KeyCode::Enter => {
                if !app.send_state.amount.is_empty() {
                    app.send_state.input_mode = SendInputMode::Confirming;
                    app.send_state.error = None;
                }
            }
            KeyCode::Esc => {
                app.send_state.input_mode = SendInputMode::EditingRecipient;
            }
            _ => {}
        },
        SendInputMode::Confirming => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.send_state.status = Some("Sending transaction...".to_string());
                app.send_transaction().await?;
                return Ok(false);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.send_state.input_mode = SendInputMode::EditingAmount;
            }
            _ => {}
        },
    }
    Ok(true)
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

    let title = Paragraph::new("☀️ SOLACE")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, menu_chunks[0]);

    let menu_items = vec![
        "Home",
        "Wallet",
        "Send",
        "Receive",
        "Transactions",
        "Settings",
    ];
    let menu: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.selected_menu_item {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(*item, style)))
        })
        .collect();

    let menu_list = List::new(menu).block(Block::default().borders(Borders::ALL).title("Menu"));
    f.render_widget(menu_list, menu_chunks[1]);

    let content = match app.state {
        AppState::Home => render_home(),
        AppState::Wallet => render_wallet(&app),
        AppState::Send => render_send(&app),
        AppState::Receive => render_receive(&app),
        AppState::Transactions => render_transactions(),
        AppState::Settings => render_settings(&app),
    };
    f.render_widget(content, chunks[1]);
}

fn render_home() -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from("Welcome to Solace! 🚀"),
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
    let lines = vec![
        Line::from("Wallet Overview"),
        Line::from(""),
        Line::from(format!("Address: {}", app.wallet.address)),
        Line::from(format!("Balance: {:.9} SOL", app.wallet.balance)),
        Line::from(""),
        Line::from("Press 'r' to refresh balance"),
    ];

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

fn render_send(app: &App) -> Paragraph<'static> {
    let mut lines = vec![Line::from("Send SOL"), Line::from("")];

    match app.send_state.input_mode {
        SendInputMode::EditingRecipient => {
            lines.push(Line::from("Enter recipient address:"));
            lines.push(Line::from(Span::styled(
                format!("{}█", app.send_state.recipient),
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from("Press Enter to continue, Esc to cancel"));
        }
        SendInputMode::EditingAmount => {
            lines.push(Line::from(format!("To: {}", app.send_state.recipient)));
            lines.push(Line::from(""));
            lines.push(Line::from("Enter amount (SOL):"));
            lines.push(Line::from(Span::styled(
                format!("{}█", app.send_state.amount),
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "Available balance: {:.9} SOL",
                app.wallet.balance
            )));
            lines.push(Line::from(""));
            lines.push(Line::from("Press Enter to continue, Esc to go back"));
        }
        SendInputMode::Confirming => {
            lines.push(Line::from("Confirm Transaction"));
            lines.push(Line::from(""));
            lines.push(Line::from(format!("To: {}", app.send_state.recipient)));
            lines.push(Line::from(format!("Amount: {} SOL", app.send_state.amount)));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press Y to confirm, N to cancel",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
        }
    }

    if let Some(ref error) = app.send_state.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Error: {}", error),
            Style::default().fg(Color::Red),
        )));
    }

    if let Some(ref status) = app.send_state.status {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            status.clone(),
            Style::default().fg(Color::Green),
        )));
    }

    Paragraph::new(lines)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Send SOL"))
}

fn render_receive(app: &App) -> Paragraph<'static> {
    let mut lines = vec![
        Line::from("Receive SOL"),
        Line::from(""),
        Line::from("Your wallet address:"),
        Line::from(Span::styled(
            app.wallet.address.to_string(),
            Style::default().fg(Color::Green),
        )),
        Line::from(""),
    ];

    // Generate QR code
    match QrCode::new(&app.wallet.address.to_string()) {
        Ok(code) => {
            let qr = code
                .render::<unicode::Dense1x2>()
                .dark_color(unicode::Dense1x2::Light)
                .light_color(unicode::Dense1x2::Dark)
                .build();

            for line in qr.lines() {
                lines.push(Line::from(line.to_string()));
            }
        }
        Err(_) => {
            lines.push(Line::from("Failed to generate QR code"));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Press Esc to go back"));

    Paragraph::new(lines)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Receive SOL"))
}

fn render_settings(app: &App) -> Paragraph<'static> {
    let network = if app.rpc_url.contains("mainnet") {
        "Mainnet Beta"
    } else if app.rpc_url.contains("testnet") {
        "Testnet"
    } else if app.rpc_url.contains("devnet") {
        "Devnet"
    } else if app.rpc_url.contains("localhost") || app.rpc_url.contains("127.0.0.1") {
        "Localnet"
    } else {
        "Custom"
    };

    Paragraph::new(vec![
        Line::from("Settings"),
        Line::from(""),
        Line::from(format!("RPC Endpoint: {}", app.rpc_url)),
        Line::from(format!("Network: {}", network)),
        Line::from(""),
        Line::from(format!("Wallet: {}", app.wallet.address)),
    ])
    .style(Style::default().fg(Color::Magenta))
    .block(Block::default().borders(Borders::ALL).title("Settings"))
}
