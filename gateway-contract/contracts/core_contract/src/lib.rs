#![no_std]

pub mod events;
pub mod types;
pub mod registration;
pub mod address_manager;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, BytesN, Env,
    String,
};
use types::ResolveData;
use registration::Registration;
use address_manager::AddressManager;

#[contract]
pub struct Contract;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Resolver(BytesN<32>),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResolverError {
    NotFound = 1,
}

#[contractimpl]
impl Contract {
    // ============================================
    // Resolver Functions
    // ============================================

    pub fn register_resolver(env: Env, commitment: BytesN<32>, wallet: Address, memo: Option<u64>) {
        let key = DataKey::Resolver(commitment);
        let data = ResolveData { wallet, memo };

        env.storage().persistent().set(&key, &data);
    }

    pub fn set_memo(env: Env, commitment: BytesN<32>, memo_id: u64) {
        let key = DataKey::Resolver(commitment);
        let mut data = env
            .storage()
            .persistent()
            .get::<DataKey, ResolveData>(&key)
            .unwrap_or_else(|| panic_with_error!(&env, ResolverError::NotFound));

        data.memo = Some(memo_id);
        env.storage().persistent().set(&key, &data);
    }

    pub fn resolve(env: Env, commitment: BytesN<32>) -> (Address, Option<u64>) {
        let key = DataKey::Resolver(commitment);

        match env.storage().persistent().get::<DataKey, ResolveData>(&key) {
            Some(data) => (data.wallet, data.memo),
            None => panic_with_error!(&env, ResolverError::NotFound),
        }
    }

    // ============================================
    // Registration Functions
    // ============================================

    pub fn register(env: Env, caller: Address, commitment: BytesN<32>) {
        Registration::register(env, caller, commitment);
    }

    pub fn get_owner(env: Env, commitment: BytesN<32>) -> Option<Address> {
        Registration::get_owner(env, commitment)
    }

    // ============================================
    // Address Manager Functions
    // ============================================

    pub fn init_address_manager(env: Env, owner: Address) {
        AddressManager::init(&env, owner);
    }

    pub fn set_master_stellar_address(
        env: Env,
        caller: Address,
        commitment: BytesN<32>,
        stellar_address: String,
    ) {
        AddressManager::set_master_stellar_address(&env, caller, commitment, stellar_address);
    }

    pub fn add_stellar_address(
        env: Env,
        caller: Address,
        commitment: BytesN<32>,
        stellar_address: String,
    ) {
        AddressManager::add_stellar_address(&env, caller, commitment, stellar_address);
    }

    pub fn register_address(env: Env, caller: Address, address: Address) {
        AddressManager::register_address(&env, caller, address);
    }

    pub fn get_master(env: Env) -> Option<String> {
        AddressManager::get_master(&env)
    }

    pub fn get_stellar_address(env: Env, commitment: BytesN<32>) -> Option<String> {
        AddressManager::get_stellar_address(&env, commitment)
    }

    pub fn is_address_registered(env: Env, address: Address) -> bool {
        AddressManager::is_registered(&env, address)
    }
}

#[cfg(test)]
mod test;
