#!/bin/bash
# Build and run the RCP client with native authentication

# Set environment variables
export RUST_LOG=info

# Build the client
cargo build --release

# Run the client with native authentication
./target/release/rcp_client_rust --auth-method native
