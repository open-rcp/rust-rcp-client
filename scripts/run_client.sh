#!/bin/bash
# Build and run the RCP client with custom options

# Set default environment variables
export RUST_LOG=info

# Parse command line arguments
AUTH_METHOD="native"
SERVER=""
USERNAME=""
BACKGROUND_CONNECT=false
VERBOSE=false
EVENT_BASED=false
GUI=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --auth=*)
      AUTH_METHOD="${1#*=}"
      shift
      ;;
    --server=*)
      SERVER="${1#*=}"
      shift
      ;;
    --username=*)
      USERNAME="${1#*=}"
      shift
      ;;
    --background)
      BACKGROUND_CONNECT=true
      shift
      ;;
    --verbose)
      VERBOSE=true
      export RUST_LOG=debug
      shift
      ;;
    --event-based)
      EVENT_BASED=true
      shift
      ;;
    --gui)
      GUI=true
      shift
      ;;
    --help)
      echo "Usage: $0 [options]"
      echo "Options:"
      echo "  --auth=METHOD       Authentication method (native, password, psk)"
      echo "  --server=SERVER     Server address to connect to"
      echo "  --username=USER     Username for authentication"
      echo "  --background        Don't connect automatically on startup"
      echo "  --event-based       Use the event-based UI implementation"
      echo "  --gui               Use the graphical user interface"
      echo "  --verbose           Enable verbose logging"
      echo "  --help              Show this help message"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      echo "Use --help for usage information"
      exit 1
      ;;
  esac
done

# Build the client
cargo build --release

# Build the command line arguments
ARGS=("--auth-method" "$AUTH_METHOD")

if [ -n "$SERVER" ]; then
  ARGS+=("--server" "$SERVER")
fi

if [ -n "$USERNAME" ]; then
  ARGS+=("--username" "$USERNAME")
fi

if [ "$BACKGROUND_CONNECT" = true ]; then
  ARGS+=("--background-connect")
fi

if [ "$VERBOSE" = true ]; then
  ARGS+=("--verbose")
fi

if [ "$EVENT_BASED" = true ]; then
  ARGS+=("--event-based")
fi

if [ "$GUI" = true ]; then
  ARGS+=("--gui")
fi

# Run the client with the specified options
echo "Running with options: ${ARGS[@]}"
./target/release/rust_rcp_client "${ARGS[@]}"
