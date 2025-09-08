# ☀️ Solace

A Solana wallet with a beautiful terminal user interface (TUI). Send, receive, and manage your SOL directly from your terminal.

[![CI](https://github.com/vitorpy/solace/actions/workflows/ci.yml/badge.svg)](https://github.com/vitorpy/solace/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/solace.svg)](https://crates.io/crates/solace)
[![Documentation](https://docs.rs/solace/badge.svg)](https://docs.rs/solace)
[![License](https://img.shields.io/crates/l/solace.svg)](https://github.com/vitorpy/solace#license)

## Features

- 🔐 **Secure Wallet Management** - Load keypairs from Solana CLI standard locations
- 💸 **Send SOL** - Interactive flow with address validation and confirmation
- 📱 **Receive SOL** - Display wallet address with QR code for easy sharing
- 🌐 **Multi-Network Support** - Connect to Mainnet, Testnet, Devnet, or custom RPC endpoints
- 🎨 **Beautiful TUI** - Clean, intuitive terminal interface built with Ratatui
- ⚡ **Real-time Updates** - Live balance refresh and transaction status

## Installation

### From Crates.io

```bash
cargo install solace
```

### From Source

```bash
git clone https://github.com/vitorpy/solace
cd solace
cargo build --release
```

## Usage

### Basic Usage

```bash
# Use default Solana CLI keypair (~/.config/solana/id.json)
solace

# Use custom keypair
solace --keypair /path/to/keypair.json

# Connect to different networks
solace --cluster testnet
solace --cluster devnet
solace --cluster https://api.mainnet-beta.solana.com
```

### Navigation

- **Arrow Keys** - Navigate menu
- **Enter** - Select menu item
- **Esc** - Go back / Cancel
- **q** - Quit application
- **r** - Refresh balance (in Wallet view)

### Sending SOL

1. Select "Send" from the menu
2. Enter recipient's wallet address
3. Enter amount in SOL
4. Confirm transaction details
5. Transaction will be signed and sent

### Receiving SOL

1. Select "Receive" from the menu
2. Your wallet address and QR code will be displayed
3. Share the address or QR code with the sender
4. Press Esc to return to menu

## Requirements

- Rust 1.70.0 or later
- A Solana keypair file (can be generated with `solana-keygen new`)

## Configuration

### Keypair

By default, solace looks for a keypair at `~/.config/solana/id.json` (Solana CLI standard location). You can specify a different keypair using the `--keypair` flag.

### Network

Available network presets:
- `mainnet` (default) - Mainnet Beta
- `testnet` - Testnet
- `devnet` - Devnet  
- `localhost` - Local validator (http://localhost:8899)

You can also provide a custom RPC URL:
```bash
solace --cluster https://your-rpc-endpoint.com
```

## Security

- Private keys never leave your local machine
- Keypair files are loaded securely from disk
- All transactions require explicit confirmation
- Compatible with hardware wallets via keypair file

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running locally

```bash
cargo run -- --cluster devnet
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the GNU General Public License v3.0 or later - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) for the TUI
- Uses [Solana SDK](https://github.com/solana-labs/solana) for blockchain interaction
- QR codes generated with [qrcode-rust](https://github.com/kennytm/qrcode-rust)

## Disclaimer

This software is provided "as is", without warranty of any kind. Use at your own risk. Always verify transaction details before confirming.