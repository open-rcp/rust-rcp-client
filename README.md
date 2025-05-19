# RCP Rust Client

A Rust-based client application for the Remote Control Protocol (RCP) project.

## Features

- Native operating system authentication support
- Skia-based graphics rendering
- Cross-platform support (Windows, macOS, Linux)
- Secure client-server communication
- Flexible connection handling (auto-connect or user-initiated)
- Multiple UI implementation options

## Building

### Prerequisites

- Rust toolchain (1.75 or newer)
- Cargo
- Build tools for your platform

### Build Steps

```bash
# Clone the repository
git clone https://github.com/open-rcp/rust_rcp_client.git
cd rust_rcp_client

# Build the client
cargo build --release
```

## Running

```bash
# Run with default settings
./target/release/rust_rcp_client

# Run with verbose logging
RUST_LOG=debug ./target/release/rust_rcp_client

# Connect to a specific server
./target/release/rust_rcp_client --server 192.168.1.100

# Use a specific authentication method
./target/release/rust_rcp_client --auth-method native

# Don't connect automatically on startup
./target/release/rust_rcp_client --background-connect

# Use the event-based UI implementation
./target/release/rust_rcp_client --event-based

# Use the provided script with options
./scripts/run_client.sh --auth=native --server=192.168.1.100 --username=user --background --event-based
```

## Configuration

The client can be configured using a TOML configuration file. By default, the client looks for the configuration file at `~/.config/rcp_client/config.toml` (on Linux/macOS) or `%APPDATA%\rcp_client\config.toml` (on Windows).

You can specify a custom configuration file path with the `--config` option:

```bash
./target/release/rust_rcp_client --config /path/to/my-config.toml
```

### Example Configuration

```toml
# Server configuration
[server]
address = "192.168.1.100"
port = 5555
use_tls = true
verify_server = true
client_cert_path = "/path/to/client.crt"
client_key_path = "/path/to/client.key"

# Authentication configuration
[auth]
method = "native"
username = "user"
save_credentials = true
use_native_auth = true

# UI configuration
[ui]
dark_mode = true
start_minimized = false
scale_factor = 1.0
theme = "default"
auto_connect = true  # Whether to connect automatically on startup
```

## UI Implementations

The client supports two different UI implementations:

1. **Simple UI** (default): A basic UI implementation that connects directly and handles simple interaction
2. **Event-Based UI**: A more complex event-driven implementation that provides more flexibility

You can switch between implementations using the `--event-based` flag when running the client.

## Command-Line Options

| Option | Description |
|--------|-------------|
| `--config FILE` | Path to the configuration file |
| `--server ADDRESS` | Server address to connect to |
| `--username USER` | Username for authentication |
| `--auth-method METHOD` | Authentication method (password, psk, native) |
| `--background-connect` | Don't connect automatically on startup |
| `--event-based` | Use the event-based UI implementation |
| `--verbose` | Enable verbose logging (can be repeated for more detail) |
| `--help` | Show help information |
| `--version` | Show version information |

## License

[MIT License](LICENSE)
