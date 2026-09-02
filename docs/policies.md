# Spending Policies

This guide explains how Soroban Safe's spending policies work and how to configure them. The policies are enforced by the `SafeWallet` contract in `contracts/safe-wallet`.

---

## Overview

A SafeWallet is governed by four interlocking policies:

| Policy | Purpose | Controlled By |
|---|---|---|
| **Daily cap** | Limit total outflows in any 24-hour window | Owner |
| **Whitelist** | Restrict transfers to approved recipient addresses | Owner |
| **Freeze/unfreeze** | Emergency circuit breaker for the wallet | Recovery key |
| **Recovery key** | Trust-minimized safety net that can freeze — but not spend from — the wallet | Set at initialization |

All policy state lives in Soroban instance storage. Only the `Owner` can change the daily cap or whitelist. The `RecoveryKey` can only toggle the freeze flag.

---

## Daily Cap

The daily cap limits how much value can leave the wallet in a rolling 24-hour window. It is stored as an `i128` value in **stroops** (1 XLM = 10,000,000 stroops).

### How it works

1. `DailyCap` is set when the wallet is initialized.
2. Every time a transfer is initiated, the contract checks the current ledger timestamp against `LastResetTimestamp`.
3. If more than 24 hours have passed, `SpentToday` is reset to `0` and `LastResetTimestamp` is updated to the current timestamp.
4. The requested amount is added to `SpentToday`. If the total exceeds `DailyCap`, the transfer fails with `DailyCapExceeded`.

This creates a **rolling 24-hour window**: the cap is enforced over the most recent 24 hours of ledger time, not per calendar day.

### Example: initialize a wallet with a 100 XLM daily cap

Using the Soroban CLI:

```bash
soroban contract invoke \
  --id <SAFE_WALLET_CONTRACT_ID> \
  --source-account <OWNER_SECRET_KEY> \
  --network testnet \
  -- \
  initialize \
  --owner <OWNER_ADDRESS> \
  --daily-cap 1000000000 \
  --recovery-key <RECOVERY_ADDRESS>
```

The `daily-cap` value is in stroops, so `1000000000` equals 100 XLM.

---

## Whitelist

The whitelist restricts outgoing transfers to a known set of recipient addresses. A transfer to any address not on the list fails with `AddressNotWhitelisted`.

### How it works

1. The whitelist is stored as a `Vec<Address>` in instance storage.
2. The owner can add addresses at any time with `add_whitelist`.
3. Before executing a transfer, the contract verifies `to ∈ whitelist`.
4. There is no on-chain limit to the number of whitelisted addresses, but each addition increases storage cost.

> **Note:** The current contract exposes `add_whitelist`. Removing an address and performing transfers are handled in the same storage layer and follow the same owner-authorization rules.

### Example: add a recipient to the whitelist

Using the Soroban CLI:

```bash
soroban contract invoke \
  --id <SAFE_WALLET_CONTRACT_ID> \
  --source-account <OWNER_SECRET_KEY> \
  --network testnet \
  -- \
  add_whitelist \
  --address <RECIPIENT_ADDRESS>
```

After this transaction confirms, `<RECIPIENT_ADDRESS>` can receive transfers from the wallet. Attempts to transfer to any other address will revert with `AddressNotWhitelisted`.

---

## Freeze and Unfreeze

The freeze flag is an emergency circuit breaker. When `Frozen` is `true`, all transfers are blocked until the wallet is unfrozen.

### When to use it

- **Compromised owner key:** If you suspect the owner's signing key is exposed, freeze the wallet immediately to stop outflows.
- **Suspicious activity:** If you see unauthorized whitelist additions or transfer attempts, freeze first and investigate.
- **Upgrade or migration:** Freeze the wallet before a sensitive contract upgrade or ownership transfer.

### How it works

1. `Frozen` defaults to `false` at initialization.
2. Only the address stored in `RecoveryKey` can call `freeze`.
3. Calling `freeze` sets `Frozen` to `true`.
4. The recovery key (or owner, depending on implementation) can later call `unfreeze` to restore normal operation.
5. While frozen, any transfer attempt fails with `WalletFrozen`.

The recovery key is intentionally **not** authorized to spend or change policies. Its only power is to halt and resume operations, providing a separation-of-duties safety net.

### Example: freeze the wallet in an emergency

Using the Soroban CLI:

```bash
soroban contract invoke \
  --id <SAFE_WALLET_CONTRACT_ID> \
  --source-account <RECOVERY_SECRET_KEY> \
  --network testnet \
  -- \
  freeze \
  --caller <RECOVERY_ADDRESS>
```

To check the current freeze status:

```bash
soroban contract invoke \
  --id <SAFE_WALLET_CONTRACT_ID> \
  --network testnet \
  -- \
  is_frozen
```

A response of `true` means all transfers are currently blocked.

---

## Recovery Key

The recovery key is a secondary address set during initialization. It exists to reduce the blast radius of a lost or compromised owner key.

### Responsibilities

- **Can freeze** the wallet at any time.
- **Cannot spend** from the wallet.
- **Cannot change** the daily cap, whitelist, or owner.
- **Cannot unfreeze** the wallet unless the implementation also grants that permission (default design: recovery key toggles freeze).

### Best practices

- Store the recovery key in a different hardware wallet or custodian than the owner key.
- Practice the freeze flow on testnet before mainnet deployment.
- Do **not** make the recovery key a hot wallet or a shared multi-sig with the owner; the value comes from separation.

### Example: initialize with a recovery key

```bash
soroban contract invoke \
  --id <SAFE_WALLET_CONTRACT_ID> \
  --source-account <OWNER_SECRET_KEY> \
  --network testnet \
  -- \
  initialize \
  --owner <OWNER_ADDRESS> \
  --daily-cap 1000000000 \
  --recovery-key <RECOVERY_ADDRESS>
```

Choose `<RECOVERY_ADDRESS>` as an address you can access quickly in an emergency but do not use for routine operations.

---

## Policy Interaction

The policies combine in the following order during a transfer:

```
transfer(to, amount)
  │
  ├─ require owner auth
  ├─ check !frozen                       ← recovery key / freeze policy
  ├─ check to ∈ whitelist                ← whitelist policy
  ├─ maybe reset daily window            ← daily cap rolling window
  ├─ check spent + amount ≤ daily_cap    ← daily cap policy
  └─ execute token transfer
```

A transfer must satisfy **every** check. Failing any single check reverts the entire transaction, keeping the wallet state unchanged.

---

## See Also

- [`architecture.md`](architecture.md) — contract layout and storage keys
- [`deployment.md`](deployment.md) — how to deploy and upgrade SafeWallet
- `contracts/safe-wallet/src/lib.rs` — contract source code
