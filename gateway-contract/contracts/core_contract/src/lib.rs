#![no_std]

pub mod address_manager;
pub mod errors;
pub mod events;
pub mod registration;
pub mod smt_root;
pub mod storage;
pub mod types;

#[cfg(test)]
mod test;

use address_manager::AddressManager;
use errors::CoreError;
use registration::Registration;
use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Bytes, BytesN, Env};
use types::{ChainType, ResolveData};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Get the current SMT root.
    /// Returns the current root if set, otherwise panics with RootNotSet error.
    pub fn get_smt_root(env: Env) -> BytesN<32> {
        smt_root::SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet))
    }

    pub fn register_resolver(env: Env, commitment: BytesN<32>, wallet: Address, memo: Option<u64>) {
        let data = ResolveData { wallet, memo };
        env.storage()
            .persistent()
            .set(&storage::DataKey::Resolver(commitment), &data);
    }

    pub fn set_memo(env: Env, commitment: BytesN<32>, memo_id: u64) {
        let mut data = env
            .storage()
            .persistent()
            .get::<storage::DataKey, ResolveData>(&storage::DataKey::Resolver(commitment.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));

        data.memo = Some(memo_id);
        env.storage()
            .persistent()
            .set(&storage::DataKey::Resolver(commitment), &data);
    }

    pub fn resolve(env: Env, commitment: BytesN<32>) -> (Address, Option<u64>) {
        match env
            .storage()
            .persistent()
            .get::<storage::DataKey, ResolveData>(&storage::DataKey::Resolver(commitment))
        {
            Some(data) => (data.wallet, data.memo),
            None => panic_with_error!(&env, CoreError::NotFound),
        }
    }

    /// Register a username commitment, mapping it to the caller's address.
    pub fn register(env: Env, caller: Address, commitment: BytesN<32>) {
        Registration::register(env, caller, commitment);
    }

    /// Get the owner address for a registered commitment.
    pub fn get_owner(env: Env, commitment: BytesN<32>) -> Option<Address> {
        Registration::get_owner(env, commitment)
    }

    /// Link an external chain address (EVM / Bitcoin / Solana / Cosmos) to a username commitment.
    /// Only the registered owner of the commitment may call this.
    pub fn add_chain_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        chain: ChainType,
        address: Bytes,
    ) {
        AddressManager::add_chain_address(env, caller, username_hash, chain, address);
    }

    /// Retrieve a previously stored chain address for a commitment.
    pub fn get_chain_address(
        env: Env,
        username_hash: BytesN<32>,
        chain: ChainType,
    ) -> Option<Bytes> {
        AddressManager::get_chain_address(env, username_hash, chain)
    }

    /// Remove a chain address for a username commitment.
    /// Only the registered owner of the commitment may call this.
    pub fn remove_chain_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        chain: ChainType,
    ) {
        AddressManager::remove_chain_address(env, caller, username_hash, chain);
    }

    /// Link a primary Stellar address to a registered username hash.
    /// Only the registered owner of the commitment may call this.
    pub fn add_stellar_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        stellar_address: Address,
    ) {
        caller.require_auth();

        // Verify the commitment is registered and caller is the owner.
        let owner = Registration::get_owner(env.clone(), username_hash.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));

        if owner != caller {
            panic_with_error!(&env, CoreError::NotFound);
        }

        env.storage().persistent().set(
            &storage::DataKey::StellarAddress(username_hash),
            &stellar_address,
        );
    }

    /// Resolve a username hash to its primary linked Stellar address.
    ///
    /// Returns `NotFound` if the username hash is not registered.
    /// Returns `NoAddressLinked` if registered but no primary Stellar address has been set.
    pub fn resolve_stellar(env: Env, username_hash: BytesN<32>) -> Address {
        // Step 1: verify the commitment is registered at all.
        if Registration::get_owner(env.clone(), username_hash.clone()).is_none() {
            panic_with_error!(&env, CoreError::NotFound);
        }

        // Step 2: return the linked primary Stellar address, or error if absent.
        env.storage()
            .persistent()
            .get::<storage::DataKey, Address>(&storage::DataKey::StellarAddress(username_hash))
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NoAddressLinked))
    }
}
