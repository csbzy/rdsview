# rrm - Redis Terminal User Interface Client

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
git clone https://github.com/yourusername/rrm.git
cd rrm

# Build the project
cargo build --release

# Install the binary
cargo install --path .
```

## Usage

### Basic Usage
```bash
rrm --host <redis-host> --port <redis-port> --password <redis-password> --db <database-number>
```

### Connection via URL
```bash
rrm --url redis://:<password>@<host>:<port>/<db>
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
- `Arrow keys`/`j`/`k`: Navigate through keys
- `Enter`: Select a key to view details
- `/`: Enter search mode
- `Esc`: Exit search mode or clear search
- `q`/`Ctrl+C`: Quit the application

## Dependencies
- [tui-rs](https://github.com/fdehau/tui-rs) - Terminal UI library
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal handling
- [redis](https://github.com/mitsuhiko/redis-rs) - Redis client
- [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
- [anyhow](https://github.com/dtolnay/anyhow) - Error handling

## License
MIT