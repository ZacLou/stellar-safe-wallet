#![no_std]

//! Shared test utilities for stellar-safe-wallet contracts.

use soroban_sdk::{Address, BytesN, Env};

/// Generate a deterministic 32-byte value for use as a Merkle root in tests.
pub fn dummy_merkle_root(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[1u8; 32])
}

/// Generate a fresh random address in the test environment.
pub fn fresh_address(env: &Env) -> Address {
    use soroban_sdk::testutils::Address as _;
    Address::generate(env)
}
