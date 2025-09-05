# rrm - Redis Terminal User Interface Client  [![Release](https://github.com/csbzy/rdsview/actions/workflows/rust.yml/badge.svg)](https://github.com/csbzy/rdsview/actions/workflows/rust.yml)
A lightweight Redis client with a Terminal User Interface (TUI) built with Rust.
## Features
- Connect to Redis using command-line parameters
- View all Redis keys with real-time filtering/search
- Display detailed key information including:
  - Key type (string, hash, list, set, zset)
  - TTL (time to live)
  - Values in appropriate format based on type
- Intuitive keyboard navigation
- Search functionality for keys

## Installation

### From source
```bash
# Clone the repository
git clone https://github.com/csbzy/rdsview
cd rdsview

# Build the project
cargo build --release

# Install the binary
cargo install --path .
```

## Usage

### Basic Usage
```bash
rdsview --host <redis-host> --port <redis-port> --password <redis-password> --db <database-number>
```

### Connection via URL
```bash
rdsview --url redis://:<password>@<host>:<port>/<db>
```

### Command-line Options
| Option | Description | Default |
|--------|-------------|---------|
| `--host` | Redis server hostname | `localhost` |
| `--port` | Redis server port | `6379` |
| `--password` | Redis authentication password | None |
| `--db` | Database number to connect to | `0` |
| `--url` | Redis connection URL (overrides other connection params) | None |

## Keyboard Shortcuts
- `Arrow keys`: Navigate through keys
- `Enter`: Select a key to view details
- `any char`: Enter search mode
- `Esc`: Exit search mode or clear search
- `q`/`Ctrl+C`: Quit the application

## Dependencies
- [ratatui](https://github.com/ratatui/ratatui)) - Terminal UI library
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal handling
- [redis](https://github.com/mitsuhiko/redis-rs) - Redis client
- [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
- [anyhow](https://github.com/dtolnay/anyhow) - Error handling

## License
MIT
