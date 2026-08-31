# Contributing to Soroban Safe

Thank you for your interest in contributing! This project is part of the **[Stellar Wave Program](https://www.drips.network/wave/stellar)** — a monthly open-source sprint where contributors earn rewards for merged work.

---

## 🌊 Stellar Wave Contributors

If you're here via the Stellar Wave Program:

1. Browse issues labeled [`good first issue`](https://github.com/ogenyialice120/stellar-safe-wallet/issues?q=label%3A%22good+first+issue%22) or [`Stellar Wave`](https://github.com/ogenyialice120/stellar-safe-wallet/issues?q=label%3A%22Stellar+Wave%22)
2. Apply on the [Drips Wave dashboard](https://www.drips.network/wave/stellar) — don't start coding until assigned
3. You have the duration of the active Wave (typically 7 days) to submit a PR
4. PR must be merged before the Wave ends to earn Points

---

## 🛠️ Development Setup

### Prerequisites

- Rust 1.74+ with `wasm32-unknown-unknown` target
- Stellar CLI v22+
- Node.js 18+ (for TypeScript client)

```bash
# Clone
git clone https://github.com/ogenyialice120/stellar-safe-wallet.git
cd stellar-safe-wallet

# Add Soroban Wasm target
rustup target add wasm32-unknown-unknown

# Build contracts
cargo build --target wasm32-unknown-unknown --release

# Run tests
cargo test
```

---

## 📋 Contribution Workflow

1. **Fork** the repository
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   # or
   git checkout -b fix/issue-description
   ```
3. **Write your code** — see coding standards below
4. **Write or update tests** for your changes
5. **Run tests locally** and make sure they pass
6. **Commit** with a clear message (see commit conventions)
7. **Open a Pull Request** against `main`

---

## ✅ Coding Standards

### Rust / Soroban Contracts

- Follow standard Rust formatting: run `cargo fmt` before committing
- Run `cargo clippy` and resolve all warnings
- All public functions must have doc comments (`///`)
- Avoid `unwrap()` in contract code — use proper error handling with `Result`
- Keep contract state minimal; prefer stateless logic where possible

---

## 📝 Commit Conventions

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add daily spending cap enforcement
fix: correct whitelist removal logic
test: add unit tests for recovery key flow
docs: update deployment guide
refactor: extract policy engine to separate module
chore: update dependencies
```

---

## 🧪 Testing Requirements

- Unit tests required for all new contract logic
- Integration tests required for end-to-end flows
- Target: >80% test coverage on contract code

```bash
cargo test
cargo test -- --nocapture
```

---

## 🔍 Pull Request Checklist

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo fmt` and `cargo clippy` are clean
- [ ] New functionality has tests
- [ ] Doc comments added for public functions
- [ ] PR description explains what and why

---

## 📄 License

By contributing, you agree your contributions will be licensed under the [MIT License](LICENSE).