#!/usr/bin/env bash
# Deploy the safe-wallet and airdrop contracts to Stellar Testnet.
#
# Usage:
#   ./scripts/deploy.sh [--network NETWORK]
#
# Defaults:
#   NETWORK=testnet
#
# Prerequisites:
#   - Rust toolchain with wasm32-unknown-unknown target
#   - Stellar CLI v22+ (https://developers.stellar.org/docs/tools/developer-tools/stellar-cli)

set -euo pipefail

NETWORK="testnet"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --network)
      NETWORK="$2"
      shift 2
      ;;
    --network=*)
      NETWORK="${1#*=}"
      shift
      ;;
    -h|--help)
      echo "Usage: $0 [--network NETWORK]"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Usage: $0 [--network NETWORK]" >&2
      exit 1
      ;;
  esac
done

# Stellar Testnet defaults; overwritten for custom networks.
RPC_URL="${RPC_URL:-https://soroban-testnet.stellar.org}"
PASSPHRASE="${PASSPHRASE:-Test SDF Network ; September 2015}"

DEPLOYER_KEY="deployer"
SAFE_WALLET_WASM="target/wasm32-unknown-unknown/release/safe_wallet.wasm"
AIRDROP_WASM="target/wasm32-unknown-unknown/release/airdrop.wasm"

echo "==> Target network: $NETWORK"

# Ensure the target network is configured (idempotent).
if ! stellar network add "$NETWORK" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$PASSPHRASE" 2>/dev/null; then
  echo "Network '$NETWORK' already configured or add failed; continuing..."
fi

# Generate a deployer key if it does not already exist.
if ! stellar keys address "$DEPLOYER_KEY" --network "$NETWORK" >/dev/null 2>&1; then
  echo "==> Generating deployer key '$DEPLOYER_KEY'"
  stellar keys generate "$DEPLOYER_KEY" --network "$NETWORK"
fi

# Fund the deployer account (friendbot on testnet; safe to re-run for funded accounts).
echo "==> Funding deployer account"
stellar keys fund "$DEPLOYER_KEY" --network "$NETWORK" || true

echo "==> Building contracts"
cargo build --target wasm32-unknown-unknown --release

if [[ ! -f "$SAFE_WALLET_WASM" ]]; then
  echo "Missing SafeWallet wasm: $SAFE_WALLET_WASM" >&2
  exit 1
fi

if [[ ! -f "$AIRDROP_WASM" ]]; then
  echo "Missing Airdrop wasm: $AIRDROP_WASM" >&2
  exit 1
fi

echo "==> Deploying SafeWallet"
SAFE_WALLET_ID=$(stellar contract deploy \
  --wasm "$SAFE_WALLET_WASM" \
  --source "$DEPLOYER_KEY" \
  --network "$NETWORK")
echo "SafeWallet contract ID: $SAFE_WALLET_ID"

echo "==> Deploying Airdrop"
AIRDROP_ID=$(stellar contract deploy \
  --wasm "$AIRDROP_WASM" \
  --source "$DEPLOYER_KEY" \
  --network "$NETWORK")
echo "Airdrop contract ID: $AIRDROP_ID"

echo ""
echo "Deployment complete!"
echo "  Network:          $NETWORK"
echo "  SafeWallet ID:    $SAFE_WALLET_ID"
echo "  Airdrop ID:       $AIRDROP_ID"
