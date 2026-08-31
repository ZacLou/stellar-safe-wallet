# Architecture

## Overview

Soroban Safe is a two-contract system deployed on Stellar:

1. **SafeWallet** (`contracts/safe-wallet`) — the programmable account abstraction wallet
2. **AirdropContract** (`contracts/airdrop`) — Merkle-tree-based token distribution for rewarding contributors

---

## SafeWallet

Stores all wallet state in Soroban's instance storage:

| Key | Type | Description |
|---|---|---|
| `Owner` | `Address` | Primary controller |
| `DailyCap` | `i128` | Max spend per 24h window (stroops) |
| `SpentToday` | `i128` | Accumulated spend in current window |
| `LastResetTimestamp` | `u64` | Ledger timestamp of last daily reset |
| `Whitelist` | `Vec<Address>` | Allowed recipient addresses |
| `RecoveryKey` | `Address` | Can freeze the wallet in emergencies |
| `Frozen` | `bool` | If true, all transfers are blocked |

### Policy Enforcement Flow

```
transfer(to, amount)
  │
  ├─ require owner auth
  ├─ check !frozen
  ├─ check to ∈ whitelist
  ├─ maybe reset daily window
  ├─ check spent + amount ≤ daily_cap
  └─ execute token transfer
```

---

## AirdropContract

Uses a standard binary Merkle tree for gas-efficient proofs.

### Leaf construction

```
leaf = SHA-256(to_xdr(claimant) ++ amount.to_le_bytes())
```

### Proof verification

Siblings are combined in lexicographic order (smaller hash on left) to produce a deterministic tree regardless of insertion order.
