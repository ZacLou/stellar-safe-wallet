# Deployment Guide

## Prerequisites

- Rust with `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- Stellar CLI v22+: https://developers.stellar.org/docs/tools/developer-tools/stellar-cli

## Build

```bash
# Build all contracts
cargo build --target wasm32-unknown-unknown --release

# Artifacts land in:
# target/wasm32-unknown-unknown/release/safe_wallet.wasm
# target/wasm32-unknown-unknown/release/airdrop.wasm
```

## Testnet

```bash
# Configure network
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

# Generate and fund account
stellar keys generate deployer --network testnet
stellar keys fund deployer --network testnet

# Deploy SafeWallet
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/safe_wallet.wasm \
  --source deployer \
  --network testnet

# Deploy Airdrop
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/airdrop.wasm \
  --source deployer \
  --network testnet
```
