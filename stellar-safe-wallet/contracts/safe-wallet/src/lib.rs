#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype,
    Address, Env, Vec,
};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Owner,
    DailyCap,
    SpentToday,
    LastResetTimestamp,
    Whitelist,
    RecoveryKey,
    Frozen,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
pub enum WalletError {
    Unauthorized = 1,
    DailyCapExceeded = 2,
    AddressNotWhitelisted = 3,
    WalletFrozen = 4,
    ZeroAmount = 5,
    NotInitialised = 6,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct SafeWallet;

#[contractimpl]
impl SafeWallet {
    /// Initialise the wallet.
    pub fn initialize(
        env: Env,
        owner: Address,
        daily_cap: i128,
        recovery_key: Address,
    ) {
        owner.require_auth();
        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage().instance().set(&DataKey::DailyCap, &daily_cap);
        env.storage().instance().set(&DataKey::RecoveryKey, &recovery_key);
        env.storage().instance().set(&DataKey::Frozen, &false);
        env.storage().instance().set(&DataKey::SpentToday, &0_i128);
        env.storage()
            .instance()
            .set(&DataKey::LastResetTimestamp, &env.ledger().timestamp());
    }

    /// Add an address to the whitelist.
    pub fn add_whitelist(env: Env, address: Address) -> Result<(), WalletError> {
        Self::require_owner(&env)?;
        let mut list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Whitelist)
            .unwrap_or_else(|| Vec::new(&env));
        list.push_back(address);
        env.storage().instance().set(&DataKey::Whitelist, &list);
        Ok(())
    }

    /// Emergency freeze — callable by recovery key only.
    pub fn freeze(env: Env, caller: Address) -> Result<(), WalletError> {
        caller.require_auth();
        let recovery_key: Address = env
            .storage()
            .instance()
            .get(&DataKey::RecoveryKey)
            .ok_or(WalletError::NotInitialised)?;
        if caller != recovery_key {
            return Err(WalletError::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Frozen, &true);
        Ok(())
    }

    /// Returns `true` if the wallet is frozen.
    pub fn is_frozen(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Frozen)
            .unwrap_or(false)
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn require_owner(env: &Env) -> Result<Address, WalletError> {
        let owner: Address = env
            .storage()
            .instance()
            .get(&DataKey::Owner)
            .ok_or(WalletError::NotInitialised)?;
        owner.require_auth();
        Ok(owner)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_wallet_not_frozen_by_default() {
        let env = Env::default();
        let contract_id = env.register(SafeWallet, ());
        let client = SafeWalletClient::new(&env, &contract_id);
        assert!(!client.is_frozen());
    }
}
