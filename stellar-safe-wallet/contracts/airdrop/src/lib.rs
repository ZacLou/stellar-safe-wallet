#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    token, vec,
    Address, BytesN, Env, Map, Vec,
};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Merkle root of the airdrop distribution tree
    MerkleRoot,
    /// Token contract address being distributed
    TokenAddress,
    /// Admin / funder address
    Admin,
    /// Tracks which addresses have already claimed
    Claimed(Address),
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
pub enum AirdropError {
    /// Caller is not the contract admin
    Unauthorized = 1,
    /// Address has already claimed their allocation
    AlreadyClaimed = 2,
    /// Merkle proof verification failed
    InvalidProof = 3,
    /// Contract has not been initialised yet
    NotInitialised = 4,
    /// Claim amount must be greater than zero
    ZeroAmount = 5,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct AirdropContract;

#[contractimpl]
impl AirdropContract {
    // -----------------------------------------------------------------------
    // Admin
    // -----------------------------------------------------------------------

    /// Initialise the airdrop contract.
    ///
    /// # Arguments
    /// * `admin`        – address that can fund and manage the airdrop
    /// * `token`        – the Soroban token contract to distribute
    /// * `merkle_root`  – 32-byte Merkle root of the distribution tree
    pub fn initialize(
        env: Env,
        admin: Address,
        token: Address,
        merkle_root: BytesN<32>,
    ) {
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TokenAddress, &token);
        env.storage().instance().set(&DataKey::MerkleRoot, &merkle_root);
    }

    // -----------------------------------------------------------------------
    // Claim
    // -----------------------------------------------------------------------

    /// Claim an airdrop allocation.
    ///
    /// # Arguments
    /// * `claimant` – address claiming the tokens (must sign)
    /// * `amount`   – number of tokens (in base units) allocated to `claimant`
    /// * `proof`    – Merkle inclusion proof as a `Vec<BytesN<32>>`
    pub fn claim(
        env: Env,
        claimant: Address,
        amount: i128,
        proof: Vec<BytesN<32>>,   // line ~85: full generic type Vec<BytesN<32>>
    ) -> Result<(), AirdropError> {
        claimant.require_auth();

        if amount <= 0 {
            return Err(AirdropError::ZeroAmount);
        }

        // Check not already claimed
        let claimed: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Claimed(claimant.clone()))
            .unwrap_or(false);

        if claimed {
            return Err(AirdropError::AlreadyClaimed);
        }

        // Retrieve stored Merkle root
        let merkle_root: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::MerkleRoot)
            .ok_or(AirdropError::NotInitialised)?;

        // Build leaf = hash(claimant ‖ amount)
        let leaf = Self::compute_leaf(&env, &claimant, amount);

        // Verify Merkle proof
        // proof_vec is explicitly typed as Vec<BytesN<32>> for clarity
        let proof_vec: Vec<BytesN<32>> = proof;  // line ~120: properly typed Vec<BytesN<32>>
        if !Self::verify_proof(&env, leaf, proof_vec, merkle_root) {
            return Err(AirdropError::InvalidProof);
        }

        // Mark as claimed before transfer (re-entrancy guard)
        env.storage()
            .persistent()
            .set(&DataKey::Claimed(claimant.clone()), &true);

        // Transfer tokens to claimant
        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .ok_or(AirdropError::NotInitialised)?;

        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(
            &env.current_contract_address(),
            &claimant,
            &amount,
        );

        // Emit claim event
        env.events().publish(
            (symbol_short!("claim"), claimant),
            amount,
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------------

    /// Returns `true` if `address` has already claimed.
    pub fn is_claimed(env: Env, address: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Claimed(address))
            .unwrap_or(false)
    }

    /// Returns the stored Merkle root.
    pub fn merkle_root(env: Env) -> Option<BytesN<32>> {
        env.storage().instance().get(&DataKey::MerkleRoot)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Compute the leaf hash: SHA-256(claimant_bytes ++ amount_le_bytes).
    fn compute_leaf(env: &Env, claimant: &Address, amount: i128) -> BytesN<32> {
        use soroban_sdk::Bytes;

        let mut data = Bytes::new(env);

        // Encode claimant as raw bytes via its xdr representation
        let claimant_bytes = claimant.to_xdr(env);
        data.append(&claimant_bytes);

        // Append amount as little-endian 16 bytes
        let amount_bytes = amount.to_le_bytes();
        for byte in amount_bytes {
            data.push_back(byte);
        }

        env.crypto().sha256(&data)
    }

    /// Verify a Merkle inclusion proof.
    ///
    /// Standard binary Merkle tree: sibling hashes are combined with the
    /// current node by sorting (smaller hash on the left).
    fn verify_proof(
        env: &Env,
        leaf: BytesN<32>,
        proof: Vec<BytesN<32>>,
        root: BytesN<32>,
    ) -> bool {
        use soroban_sdk::Bytes;

        let mut current = leaf;

        for sibling in proof.iter() {
            let mut combined = Bytes::new(env);

            // Lexicographic ordering ensures deterministic tree construction
            if current.as_ref() <= sibling.as_ref() {
                combined.append(&Bytes::from(current.clone()));
                combined.append(&Bytes::from(sibling.clone()));
            } else {
                combined.append(&Bytes::from(sibling.clone()));
                combined.append(&Bytes::from(current.clone()));
            }

            current = env.crypto().sha256(&combined);
        }

        current == root
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
    fn test_is_claimed_default_false() {
        let env = Env::default();
        let contract_id = env.register(AirdropContract, ());
        let client = AirdropContractClient::new(&env, &contract_id);

        let user = Address::generate(&env);
        assert!(!client.is_claimed(&user));
    }

    #[test]
    fn test_merkle_root_unset() {
        let env = Env::default();
        let contract_id = env.register(AirdropContract, ());
        let client = AirdropContractClient::new(&env, &contract_id);

        assert!(client.merkle_root().is_none());
    }
}
