#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    token, Address, Env, Vec,
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
    TokenAddress,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
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

    /// Set the token contract address used for transfers. Owner only.
    pub fn set_token_address(env: Env, token: Address) -> Result<(), WalletError> {
        Self::require_owner(&env)?;
        env.storage().instance().set(&DataKey::TokenAddress, &token);
        Ok(())
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

    /// Transfer tokens to a whitelisted address, enforcing owner auth,
    /// wallet freeze state, and daily spending cap.
    pub fn transfer(env: Env, to: Address, amount: i128) -> Result<(), WalletError> {
        Self::require_owner(&env)?;

        if Self::is_frozen(env.clone()) {
            return Err(WalletError::WalletFrozen);
        }

        if amount <= 0 {
            return Err(WalletError::ZeroAmount);
        }

        // Ensure `to` is in the whitelist.
        let list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Whitelist)
            .unwrap_or_else(|| Vec::new(&env));
        if !Self::vec_contains(&list, &to) {
            return Err(WalletError::AddressNotWhitelisted);
        }

        // Reset daily spend counter if the 24h window has elapsed.
        let now = env.ledger().timestamp();
        let last_reset: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LastResetTimestamp)
            .unwrap_or(now);
        let spent_today: i128 = if now >= last_reset + 86_400 {
            env.storage().instance().set(&DataKey::LastResetTimestamp, &now);
            0_i128
        } else {
            env.storage()
                .instance()
                .get(&DataKey::SpentToday)
                .unwrap_or(0_i128)
        };

        let daily_cap: i128 = env
            .storage()
            .instance()
            .get(&DataKey::DailyCap)
            .ok_or(WalletError::NotInitialised)?;
        if spent_today + amount > daily_cap {
            return Err(WalletError::DailyCapExceeded);
        }

        // Execute the token transfer.
        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .ok_or(WalletError::NotInitialised)?;
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&env.current_contract_address(), &to, &amount);

        // Record spend.
        env.storage()
            .instance()
            .set(&DataKey::SpentToday, &(spent_today + amount));

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

    fn vec_contains(list: &Vec<Address>, target: &Address) -> bool {
        for i in 0..list.len() {
            if let Some(item) = list.get(i) {
                if &item == target {
                    return true;
                }
            }
        }
        false
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

    #[test]
    fn test_transfer_fails_when_frozen() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SafeWallet, ());
        let client = SafeWalletClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let recovery = Address::generate(&env);
        let to = Address::generate(&env);
        client.initialize(&owner, &1000, &recovery);
        client.add_whitelist(&to);
        client.freeze(&recovery);

        assert_eq!(
            client.try_transfer(&to, &100),
            Err(Ok(WalletError::WalletFrozen))
        );
    }

    #[test]
    fn test_transfer_fails_when_not_whitelisted() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SafeWallet, ());
        let client = SafeWalletClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let recovery = Address::generate(&env);
        let to = Address::generate(&env);
        client.initialize(&owner, &1000, &recovery);

        assert_eq!(
            client.try_transfer(&to, &100),
            Err(Ok(WalletError::AddressNotWhitelisted))
        );
    }

    #[test]
    fn test_transfer_fails_when_exceeds_daily_cap() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SafeWallet, ());
        let client = SafeWalletClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let recovery = Address::generate(&env);
        let to = Address::generate(&env);
        client.initialize(&owner, &1000, &recovery);
        client.add_whitelist(&to);

        assert_eq!(
            client.try_transfer(&to, &1500),
            Err(Ok(WalletError::DailyCapExceeded))
        );
    }

    #[test]
    fn test_transfer_fails_when_token_not_set() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SafeWallet, ());
        let client = SafeWalletClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let recovery = Address::generate(&env);
        let to = Address::generate(&env);
        client.initialize(&owner, &1000, &recovery);
        client.add_whitelist(&to);

        assert_eq!(
            client.try_transfer(&to, &100),
            Err(Ok(WalletError::NotInitialised))
        );
    }
}
