#!/usr/bin/env bash
#
# Deploy the SafeWallet and Airdrop contracts to Stellar Testnet (or another network).
#
# Usage:
#   ./scripts/deploy.sh [--network <network-name>]
#
# Defaults:
#   --network testnet
#
# The script is idempotent for network configuration and key generation: running it
# multiple times with the same network will not create duplicates.

set -euo pipefail

NETWORK="testnet"
DEPLOYER_KEY="deployer"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --network)
      NETWORK="$2"
      shift 2
      ;;
    --help|-h)
      echo "Usage: $0 [--network <network-name>]"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      echo "Usage: $0 [--network <network-name>]"
      exit 1
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Network configuration for known networks.
RPC_URL=""
PASSPHRASE=""
case "$NETWORK" in
  testnet)
    RPC_URL="https://soroban-testnet.stellar.org"
    PASSPHRASE="Test SDF Network ; September 2015"
    ;;
  futurenet)
    RPC_URL="https://rpc-futurenet.stellar.org"
    PASSPHRASE="Test SDF Future Network ; October 2022"
    ;;
  *)
    echo "Unknown network '$NETWORK'. Please configure it manually with:"
    echo "  stellar network add $NETWORK --rpc-url <url> --network-passphrase <passphrase>"
    exit 1
    ;;
esac

# Ensure the Stellar CLI is available.
if ! command -v stellar >/dev/null 2>&1; then
  echo "Error: stellar CLI not found. Install it from https://developers.stellar.org/docs/tools/developer-tools/stellar-cli"
  exit 1
fi

# Add the network if it is not already configured (idempotent).
if ! stellar network ls | grep -qx "$NETWORK"; then
  echo "Configuring Stellar network: $NETWORK"
  stellar network add "$NETWORK" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$PASSPHRASE"
else
  echo "Network '$NETWORK' already configured."
fi

# Generate the deployer key if it does not exist (idempotent).
if ! stellar keys ls | grep -qx "$DEPLOYER_KEY"; then
  echo "Generating deployer key: $DEPLOYER_KEY"
  stellar keys generate "$DEPLOYER_KEY" --network "$NETWORK"
else
  echo "Deployer key '$DEPLOYER_KEY' already exists."
fi

# Fund the deployer account. Friendbot only works on testnet/futurenet.
echo "Funding deployer account via friendbot..."
stellar keys fund "$DEPLOYER_KEY" --network "$NETWORK" || echo "Friendbot funding skipped or account already funded."

# Build the contracts.
echo "Building contracts..."
cd "$PROJECT_ROOT"
cargo build --target wasm32-unknown-unknown --release

SAFE_WALLET_WASM="$PROJECT_ROOT/target/wasm32-unknown-unknown/release/safe_wallet.wasm"
AIRDROP_WASM="$PROJECT_ROOT/target/wasm32-unknown-unknown/release/airdrop.wasm"

if [[ ! -f "$SAFE_WALLET_WASM" ]]; then
  echo "Error: safe_wallet.wasm not found at $SAFE_WALLET_WASM"
  exit 1
fi
if [[ ! -f "$AIRDROP_WASM" ]]; then
  echo "Error: airdrop.wasm not found at $AIRDROP_WASM"
  exit 1
fi

# Deploy contracts and capture contract IDs.
echo "Deploying SafeWallet contract..."
SAFE_WALLET_ID=$(stellar contract deploy \
  --wasm "$SAFE_WALLET_WASM" \
  --source "$DEPLOYER_KEY" \
  --network "$NETWORK" \
  --alias safe-wallet)

echo "Deploying Airdrop contract..."
AIRDROP_ID=$(stellar contract deploy \
  --wasm "$AIRDROP_WASM" \
  --source "$DEPLOYER_KEY" \
  --network "$NETWORK" \
  --alias airdrop)

echo ""
echo "========================================"
echo "Deployment complete on network: $NETWORK"
echo "SafeWallet contract ID: $SAFE_WALLET_ID"
echo "Airdrop contract ID:    $AIRDROP_ID"
echo "========================================"
