# RCP Rust Client

A Rust-based client application for the Remote Control Protocol (RCP) project.

## Features

- Native operating system authentication support
- Skia-based graphics rendering
- Cross-platform support (Windows, macOS, Linux)
- Secure client-server communication

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
```

## Authentication Methods

### Native OS Authentication

Uses the host operating system's authentication mechanisms:
- On Windows: Windows Security Support Provider Interface (SSPI)
- On macOS: Directory Services API and optional Touch ID
- On Linux: Pluggable Authentication Modules (PAM)

```bash
./target/release/rust_rcp_client --auth-method native
```

### Password Authentication

Simple username/password authentication:

```bash
./target/release/rust_rcp_client --auth-method password --username your_username
```

### Pre-Shared Key (PSK) Authentication

Use a pre-shared key for authentication:

```bash
./target/release/rust_rcp_client --auth-method psk
```

## Configuration

The client can be configured using a TOML configuration file. By default, the client looks for a configuration file at:

- Windows: `%APPDATA%\rcp_client\config.toml`
- macOS: `~/Library/Application Support/rcp_client/config.toml`
- Linux: `~/.config/rcp_client/config.toml`

You can also specify a configuration file using the `--config` option:

```bash
./target/release/rust_rcp_client --config /path/to/config.toml
```

Example configuration:

```toml
[server]
address = "192.168.1.100"
port = 8717
use_tls = false

[auth]
method = "native"
username = "user"
save_credentials = true
use_native_auth = true

[ui]
dark_mode = true
scale_factor = 1.0
```

## License

[MIT License](LICENSE)
