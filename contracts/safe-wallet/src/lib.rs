#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    token, Address, Env, Vec,
};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[contracttype]
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

    /// Transfer `amount` of `token` to `to`, enforcing wallet policies.
    ///
    /// 1. Only the owner may call.
    /// 2. Rejected if the wallet is frozen.
    /// 3. Rejected if `to` is not whitelisted.
    /// 4. Resets the daily spend counter when the 24h window has elapsed.
    /// 5. Rejected if the transfer would exceed the daily cap.
    pub fn transfer(
        env: Env,
        token: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), WalletError> {
        Self::require_owner(&env)?;

        if amount <= 0 {
            return Err(WalletError::ZeroAmount);
        }

        let frozen: bool = env.storage().instance().get(&DataKey::Frozen).unwrap_or(false);
        if frozen {
            return Err(WalletError::WalletFrozen);
        }

        let whitelist: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Whitelist)
            .unwrap_or_else(|| Vec::new(&env));
        if !whitelist.contains(&to) {
            return Err(WalletError::AddressNotWhitelisted);
        }

        let daily_cap: i128 = env
            .storage()
            .instance()
            .get(&DataKey::DailyCap)
            .ok_or(WalletError::NotInitialised)?;

        let now = env.ledger().timestamp();
        let last_reset: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LastResetTimestamp)
            .unwrap_or(now);
        let mut spent_today: i128 = env
            .storage()
            .instance()
            .get(&DataKey::SpentToday)
            .unwrap_or(0_i128);

        const DAY_IN_SECONDS: u64 = 86_400;
        if now >= last_reset + DAY_IN_SECONDS {
            spent_today = 0;
            env.storage()
                .instance()
                .set(&DataKey::LastResetTimestamp, &now);
        }

        if spent_today + amount > daily_cap {
            return Err(WalletError::DailyCapExceeded);
        }

        token::Client::new(&env, &token).transfer(&env.current_contract_address(), &to, &amount);

        env.storage()
            .instance()
            .set(&DataKey::SpentToday, &(spent_today + amount));

        Ok(())
    }

    /// Emergency freeze — callable by the recovery key only.
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
    use soroban_sdk::token::{self, StellarAssetClient};
    use soroban_sdk::testutils::Ledger as _;

    #[test]
    fn test_wallet_not_frozen_by_default() {
        let env = Env::default();
        let contract_id = env.register(SafeWallet, ());
        let client = SafeWalletClient::new(&env, &contract_id);
        assert!(!client.is_frozen());
    }

    fn setup_wallet(env: &Env, daily_cap: i128) -> (Address, SafeWalletClient<'static>, Address, Address) {
        env.mock_all_auths();
        let contract_id = env.register(SafeWallet, ());
        let client = SafeWalletClient::new(env, &contract_id);
        let owner = Address::generate(env);
        let recovery = Address::generate(env);
        let token_admin = Address::generate(env);
        let token = env.register_stellar_asset_contract(token_admin.clone());
        client.initialize(&owner, &daily_cap, &recovery);
        (token, client, owner, recovery)
    }

    #[test]
    fn test_transfer_happy_path() {
        let env = Env::default();
        let (token, client, owner, _recovery) = setup_wallet(&env, 1_000_000);
        let recipient = Address::generate(&env);

        client.add_whitelist(&recipient);
        StellarAssetClient::new(&env, &token).mint(&owner, &500_000);
        token::Client::new(&env, &token).transfer(&owner, &client.address, &500_000);

        client.transfer(&token, &recipient, &100_000);
        assert_eq!(token::Client::new(&env, &token).balance(&recipient), 100_000);
        assert_eq!(token::Client::new(&env, &token).balance(&client.address), 400_000);
    }

    #[test]
    fn test_transfer_rejects_frozen_wallet() {
        let env = Env::default();
        let (token, client, _owner, recovery) = setup_wallet(&env, 1_000_000);
        let recipient = Address::generate(&env);

        client.add_whitelist(&recipient);
        client.freeze(&recovery);

        assert_eq!(
            client.try_transfer(&token, &recipient, &100_000),
            Err(Ok(WalletError::WalletFrozen))
        );
    }

    #[test]
    fn test_transfer_rejects_non_whitelisted_recipient() {
        let env = Env::default();
        let (token, client, _owner, _recovery) = setup_wallet(&env, 1_000_000);
        let recipient = Address::generate(&env);

        assert_eq!(
            client.try_transfer(&token, &recipient, &100_000),
            Err(Ok(WalletError::AddressNotWhitelisted))
        );
    }

    #[test]
    fn test_transfer_rejects_daily_cap_exceeded() {
        let env = Env::default();
        let (token, client, owner, _recovery) = setup_wallet(&env, 100_000);
        let recipient = Address::generate(&env);

        client.add_whitelist(&recipient);
        StellarAssetClient::new(&env, &token).mint(&owner, &200_000);
        token::Client::new(&env, &token).transfer(&owner, &client.address, &200_000);

        // A transfer within cap works.
        client.transfer(&token, &recipient, &50_000);
        // Exceeding the remaining cap is rejected.
        assert_eq!(
            client.try_transfer(&token, &recipient, &60_000),
            Err(Ok(WalletError::DailyCapExceeded))
        );
    }

    #[test]
    fn test_transfer_resets_daily_spend_after_24h() {
        let env = Env::default();
        let (token, client, owner, _recovery) = setup_wallet(&env, 100_000);
        let recipient = Address::generate(&env);

        client.add_whitelist(&recipient);
        StellarAssetClient::new(&env, &token).mint(&owner, &200_000);
        token::Client::new(&env, &token).transfer(&owner, &client.address, &200_000);

        client.transfer(&token, &recipient, &100_000);

        // Advance time by 24 hours + 1 second.
        env.ledger().set_timestamp(env.ledger().timestamp() + 86_401);
        // The next transfer should succeed because the counter has reset.
        client.transfer(&token, &recipient, &100_000);
    }

    #[test]
    fn test_transfer_rejects_zero_amount() {
        let env = Env::default();
        let (token, client, _owner, _recovery) = setup_wallet(&env, 1_000_000);
        let recipient = Address::generate(&env);

        client.add_whitelist(&recipient);

        assert_eq!(
            client.try_transfer(&token, &recipient, &0),
            Err(Ok(WalletError::ZeroAmount))
        );
    }
}
