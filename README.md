# Steam Deck Controller Light Show Control

A Tauri application that captures Steam Deck controller inputs and sends them to a light show server. Features auto-updates via GitHub releases.

## Features

- 🎮 Native gamepad support using gilrs
- 🔄 Automatic updates via GitHub releases  
- 🌐 Send controller events to HTTP endpoints
- 🖥️ Cross-platform (focus on Steam Deck/Linux)
- ⚡ Real-time input visualization
- 🎯 AppImage distribution for easy Steam Deck installation

## Prerequisites

- Rust 1.70+
- Linux dependencies for Tauri (Steam Deck has these)

## Development Setup

1. Install dependencies:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Tauri CLI
cargo install tauri-cli --version "^2.0"

# Install system dependencies (Ubuntu/Debian - Steam Deck already has these)
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libudev-dev
```

2. Clone and run:
```bash
git clone https://github.com/YOUR_USERNAME/steam-deck-controller
cd steam-deck-controller
cargo tauri dev
```

**No Node.js or pnpm required!** This project uses pure Rust with Dioxus compiling to WebAssembly.

## Quick Start

```bash
# Install Tauri CLI (one time)
cargo install tauri-cli --version "^2.0"

# Clone and run
git clone https://your-repo-url/steam-deck-controller
cd steam-deck-controller
cargo tauri dev

# In another terminal, start test server
python3 test-server.py
```

## Building

### Build for current platform:
```bash
cargo tauri build
```

### Build AppImage for Steam Deck:
```bash
cargo tauri build --target x86_64-unknown-linux-gnu
```

## Setting Up Auto-Updates

1. Generate update keys:
```bash
./generate-update-key.sh
```

2. Add to GitHub repository secrets:
   - `TAURI_SIGNING_PRIVATE_KEY` - Contents of `.tauri/myapp.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` - Password you entered

3. Update `src-tauri/tauri.conf.json`:
   - Replace `{{owner}}/{{repo}}` with your GitHub username/repository
   - Add the public key to the updater configuration

## Creating a Release

1. Update version in:
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
   - `package.json`

2. Commit and tag:
```bash
git add .
git commit -m "Release v0.1.0"
git tag v0.1.0
git push origin main --tags
```

The GitHub Action will automatically build and create a release with:
- Linux AppImage
- Update manifest (latest.json)

## Steam Deck Installation

See [STEAM_DECK_INSTALL.md](STEAM_DECK_INSTALL.md) for detailed installation instructions.

## Usage

1. Launch the application
2. Configure your light show server endpoint
3. Connect a controller (Steam Deck's built-in controls work automatically)
4. Press buttons and move sticks to send commands to your server

### API Format

The app sends POST requests to your configured endpoint with:
```json
{
  "controller_id": 0,
  "action": "button:A",
  "timestamp": 1234567890
}
```

## Project Structure

```
├── src/                  # Frontend (Dioxus)
│   ├── app.rs           # Main UI component
│   └── main.rs          # Entry point
├── src-tauri/           # Backend (Tauri + Rust)
│   ├── src/
│   │   ├── commands.rs  # Tauri commands
│   │   ├── gamepad.rs   # Controller handling
│   │   └── lib.rs       # Main app logic
│   └── tauri.conf.json  # Tauri configuration
└── assets/              # Static assets
```

## Troubleshooting

### Controller not detected
- Ensure udev rules are set up for gamepad access
- Try running with `sudo` to test permissions
- Check if controller works in other applications

### Build errors
- Update Rust: `rustup update`
- Clear cache: `cargo clean`
- Reinstall dependencies: `rm -rf node_modules && pnpm install`

## Contributing

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Open a Pull Request

## License

MIT