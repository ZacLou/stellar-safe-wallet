# Spending Policies

This guide explains how Soroban Safe enforces spending policies on Stellar. It is intended for wallet owners, integrators, and contributors who want to understand the policy engine without reading the contract source.

## Overview

Soroban Safe uses a small set of on-chain policy controls that are evaluated on every transfer:

| Policy | Purpose |
|---|---|
| **Daily cap** | Limit total outflows in a rolling 24-hour window |
| **Whitelist** | Restrict transfers to approved recipient addresses |
| **Freeze/unfreeze** | Halt all outflows in an emergency |
| **Recovery key** | A separate key that can freeze the wallet if the owner key is compromised |

These policies are stored in the wallet's instance storage and are checked by the `transfer` entrypoint before any token movement.

---

## Daily Cap

The daily cap is the maximum amount the wallet can transfer within a rolling 24-hour window. It is set during `initialize` and is denominated in **stroops** (1 XLM = 10,000,000 stroops).

### How it works

1. Each wallet stores `DailyCap`, `SpentToday`, and `LastResetTimestamp`.
2. Before a transfer, the contract checks whether the current ledger timestamp has moved past `LastResetTimestamp + 24 hours`.
3. If the window has rolled over, `SpentToday` is reset to `0` and `LastResetTimestamp` is updated.
4. The requested amount is added to `SpentToday` and compared against `DailyCap`.
5. If the new total would exceed the cap, the transfer returns `WalletError::DailyCapExceeded`.

### Example

```bash
# Initialize a wallet with a 100 XLM daily cap
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source owner \
  --network testnet \
  -- \
  initialize \
  --owner <OWNER_ADDRESS> \
  --daily_cap 1000000000 \
  --recovery_key <RECOVERY_ADDRESS>
```

```rust
// Rust SDK example
use soroban_sdk::{Address, Env};

let client = SafeWalletClient::new(&env, &contract_id);
client.initialize(
    &owner_address,      // Address
    &100_000_0000i128,   // 100 XLM daily cap in stroops
    &recovery_address,    // Address
);
```

---

## Whitelist

The whitelist is a list of `Address` values that are allowed to receive transfers. If the wallet is initialized with a non-empty whitelist (or one is added later), every `transfer` recipient must be present in the list.

### Managing the whitelist

Only the wallet owner can add or remove addresses.

```bash
# Add a recipient to the whitelist
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source owner \
  --network testnet \
  -- \
  add_whitelist \
  --address <RECIPIENT_ADDRESS>
```

```rust
// Rust SDK example
client.add_whitelist(&recipient_address);
```

A transfer to an address that is not on the whitelist returns `WalletError::AddressNotWhitelisted`.

### When to use the whitelist

- Restrict payroll or treasury outflows to known counter-parties.
- Reduce blast radius if the owner key is used without approval.
- Enforce vendor lists in multi-user organizations.

---

## Freeze and Unfreeze

The wallet can be frozen by the recovery key. While frozen, all transfers are rejected with `WalletError::WalletFrozen`. The owner can still read state, but no outflows are possible.

### Freeze flow

1. The recovery key calls `freeze(caller)`.
2. The contract verifies that `caller == RecoveryKey`.
3. `Frozen` is set to `true`.

```bash
# Freeze the wallet from the recovery key
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source recovery \
  --network testnet \
  -- \
  freeze \
  --caller <RECOVERY_ADDRESS>
```

```rust
// Rust SDK example
client.freeze(&recovery_address);
```

### Unfreeze

Unfreezing is an owner-only operation. The owner calls `unfreeze()` to set `Frozen` back to `false` and resume normal transfers.

```bash
# Unfreeze the wallet (owner only)
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source owner \
  --network testnet \
  -- \
  unfreeze
```

```rust
// Rust SDK example
client.unfreeze();
```

### When to freeze

- The owner key is suspected to be compromised.
- Unusual spending patterns are detected off-chain.
- A security incident requires an immediate circuit breaker.

---

## Recovery Key

The recovery key is a separate `Address` set during `initialize`. It has only one power: **freezing the wallet**. It cannot transfer funds, change the daily cap, or modify the whitelist.

### Responsibilities

- Store the recovery key in a different location or with a different custodian than the owner key.
- Use it only in emergencies.
- After freezing, coordinate with the owner to rotate keys or investigate before unfreezing.

### Why a separate key?

If the owner and recovery keys were the same, a compromised owner key could both steal funds and prevent anyone from stopping the theft. Splitting the roles means an attacker who owns the owner key still cannot prevent the recovery key from freezing the wallet.

---

## Transfer Enforcement Summary

The following checks are applied in order on every `transfer`:

1. **Owner authorization** — `owner.require_auth()`.
2. **Frozen state** — reject if `Frozen == true`.
3. **Whitelist** — reject if a whitelist exists and the recipient is not on it.
4. **Daily window reset** — roll `SpentToday` forward if 24 hours have passed.
5. **Daily cap** — reject if `SpentToday + amount > DailyCap`.
6. **Execute transfer** — move tokens to the recipient and increment `SpentToday`.

```text
transfer(to, amount)
  ├─ require owner auth
  ├─ check !frozen
  ├─ check to ∈ whitelist (if whitelist is set)
  ├─ maybe reset daily window
  ├─ check spent + amount ≤ daily_cap
  └─ execute token transfer
```

---

## Error Reference

| Error | Cause |
|---|---|
| `Unauthorized` | Caller is not the owner or recovery key for the requested operation. |
| `DailyCapExceeded` | Transfer would exceed the rolling 24-hour cap. |
| `AddressNotWhitelisted` | Recipient is not in the whitelist. |
| `WalletFrozen` | Wallet is frozen and no transfers are allowed. |
| `ZeroAmount` | Transfer amount is zero or negative. |
| `NotInitialised` | Wallet storage has not been initialized yet. |

---

## See Also

- [`architecture.md`](architecture.md) — contract layout and storage keys
- [`deployment.md`](deployment.md) — how to deploy the wallet to Testnet/Mainnet
- [`CONTRIBUTING.md`](../CONTRIBUTING.md) — contribution guidelines
