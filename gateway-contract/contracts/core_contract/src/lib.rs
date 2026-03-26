#![no_std]

mod errors;
mod events;
mod storage;
mod types;
pub mod address_manager;
pub mod errors;
pub mod events;
pub mod registration;
pub mod smt_root;
pub mod storage;
pub mod types;
pub mod zk_verifier;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractclient, contractimpl, panic_with_error, Address, BytesN, Env};

pub use crate::errors::ContractError;
pub use crate::events::{MerkleRootUpdated, UsernameRegistered};
pub use crate::types::{Proof, PublicSignals};
use address_manager::AddressManager;
use errors::CoreError;
use events::{REGISTER_EVENT, TRANSFER_EVENT};
use registration::Registration;
use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Bytes, BytesN, Env};
use types::{ChainType, PrivacyMode, PublicSignals, ResolveData};

#[contract]
pub struct CoreContract;

#[contractclient(name = "VerifierContractClient")]
pub trait VerifierContract {
    fn verify_proof(env: Env, proof: Proof, public_signals: PublicSignals) -> bool;
}

fn current_merkle_root(env: &Env) -> BytesN<32> {
    storage::get_merkle_root(env)
        .unwrap_or_else(|| panic_with_error!(env, ContractError::NotInitialized))
}

fn update_merkle_root(env: &Env, old_root: BytesN<32>, new_root: BytesN<32>) {
    storage::set_merkle_root(env, &new_root);

    MerkleRootUpdated { old_root, new_root }.publish(env);
}

#[contractimpl]
impl CoreContract {
    pub fn init(env: Env, verifier: Address, root: BytesN<32>) {
        if storage::is_initialized(&env) {
            panic_with_error!(&env, ContractError::AlreadyInitialized);
        }

        storage::set_verifier(&env, &verifier);
        storage::set_merkle_root(&env, &root);
    }

    pub fn submit_proof(env: Env, proof: Proof, public_signals: PublicSignals) {
        let current_root = current_merkle_root(&env);

        if current_root != public_signals.old_root.clone() {
            panic_with_error!(&env, ContractError::RootMismatch);
        }

        if storage::has_commitment(&env, &public_signals.commitment) {
            panic_with_error!(&env, ContractError::DuplicateCommitment);
        }

        let verifier = storage::get_verifier(&env)
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::NotInitialized));
        let verifier_client = VerifierContractClient::new(&env, &verifier);
        let is_valid = verifier_client.verify_proof(&proof, &public_signals);
        if !is_valid {
            panic_with_error!(&env, ContractError::InvalidProof);
#[contractimpl]
impl Contract {
    pub fn get_smt_root(env: Env) -> BytesN<32> {
        smt_root::SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet))
    }

    pub fn register_resolver(
        env: Env,
        caller: Address,
        commitment: BytesN<32>,
        proof: Bytes,
        public_signals: PublicSignals,
    ) {
        caller.require_auth();

        let key = storage::DataKey::Resolver(commitment.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, CoreError::DuplicateCommitment);
        }

        let current_root = smt_root::SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet));
        if public_signals.old_root != current_root {
            panic_with_error!(&env, CoreError::StaleRoot);
        }

        if !zk_verifier::ZkVerifier::verify_groth16_proof(&env, &proof, &public_signals) {
            panic_with_error!(&env, CoreError::InvalidProof);
        }

        let data = ResolveData {
            wallet: caller.clone(),
            memo: None,
        };
        env.storage().persistent().set(&key, &data);

        smt_root::SmtRoot::update_root(&env, public_signals.new_root);

        #[allow(deprecated)]
        env.events()
            .publish((REGISTER_EVENT,), (commitment, caller));
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

    pub fn set_privacy_mode(env: Env, username_hash: BytesN<32>, mode: PrivacyMode) {
        AddressManager::set_privacy_mode(env, username_hash, mode);
    }

    pub fn get_privacy_mode(env: Env, username_hash: BytesN<32>) -> PrivacyMode {
        AddressManager::get_privacy_mode(env, username_hash)
    }

    pub fn resolve(env: Env, commitment: BytesN<32>) -> (Address, Option<u64>) {
        match env
            .storage()
            .persistent()
            .get::<storage::DataKey, ResolveData>(&storage::DataKey::Resolver(commitment.clone()))
        {
            Some(data) => {
                if AddressManager::get_privacy_mode(env.clone(), commitment) == PrivacyMode::Private
                {
                    (env.current_contract_address(), data.memo)
                } else {
                    (data.wallet, data.memo)
                }
            }
            None => panic_with_error!(&env, CoreError::NotFound),
        }

        storage::store_commitment(&env, &public_signals.commitment);
        update_merkle_root(&env, current_root, public_signals.new_root.clone());

        UsernameRegistered {
            commitment: public_signals.commitment,
        }
        .publish(&env);
    }

    pub fn get_merkle_root(env: Env) -> BytesN<32> {
        current_merkle_root(&env)
    }

    pub fn get_verifier(env: Env) -> Option<Address> {
        storage::get_verifier(&env)
    }

    pub fn has_commitment(env: Env, commitment: BytesN<32>) -> bool {
        storage::has_commitment(&env, &commitment)
    /// Register a username commitment, mapping it to the caller's address.
    pub fn register(env: Env, caller: Address, commitment: BytesN<32>) {
        Registration::register(env, caller, commitment);
    }

    pub fn get_owner(env: Env, commitment: BytesN<32>) -> Option<Address> {
        Registration::get_owner(env, commitment)
    }

    pub fn add_chain_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        chain: ChainType,
        address: Bytes,
    ) {
        AddressManager::add_chain_address(env, caller, username_hash, chain, address);
    }

    pub fn get_chain_address(
        env: Env,
        username_hash: BytesN<32>,
        chain: ChainType,
    ) -> Option<Bytes> {
        AddressManager::get_chain_address(env, username_hash, chain)
    }

    pub fn remove_chain_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        chain: ChainType,
    ) {
        AddressManager::remove_chain_address(env, caller, username_hash, chain);
    }

    pub fn add_stellar_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        stellar_address: Address,
    ) {
        caller.require_auth();

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

    /// Transfer ownership of a commitment to a new owner.
    /// The caller must be the current registered owner.
    /// Panics with `Unauthorized` if caller is not the owner.
    /// Panics with `SameOwner` if new_owner equals the current owner.
    pub fn transfer_ownership(
        env: Env,
        caller: Address,
        commitment: BytesN<32>,
        new_owner: Address,
    ) {
        caller.require_auth();

        let key = registration::DataKey::Commitment(commitment.clone());
        let current_owner: Address = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));

        if caller != current_owner {
            panic_with_error!(&env, CoreError::Unauthorized);
        }

        if new_owner == current_owner {
            panic_with_error!(&env, CoreError::SameOwner);
        }

        env.storage().persistent().set(&key, &new_owner);

        #[allow(deprecated)]
        env.events()
            .publish((TRANSFER_EVENT,), (commitment, caller, new_owner));
    }

    /// Transfer ownership of a commitment with ZK proof verification and SMT root update.
    /// The caller must be the current registered owner.
    /// Panics with `Unauthorized` if caller is not the owner.
    /// Panics with `SameOwner` if new_owner equals the current owner.
    /// Panics with `StaleRoot` if public_signals.old_root does not match the on-chain root.
    pub fn transfer(
        env: Env,
        caller: Address,
        commitment: BytesN<32>,
        new_owner: Address,
        proof: Bytes,
        public_signals: PublicSignals,
    ) {
        caller.require_auth();

        let key = registration::DataKey::Commitment(commitment.clone());
        let current_owner: Address = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));

        if caller != current_owner {
            panic_with_error!(&env, CoreError::Unauthorized);
        }

        if new_owner == current_owner {
            panic_with_error!(&env, CoreError::SameOwner);
        }

        // SMT root consistency
        let current_root = smt_root::SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet));
        if public_signals.old_root != current_root {
            panic_with_error!(&env, CoreError::StaleRoot);
        }

        // ZK proof verification (Phase 4 stub)
        if !zk_verifier::ZkVerifier::verify_groth16_proof(&env, &proof, &public_signals) {
            panic_with_error!(&env, CoreError::InvalidProof);
        }

        // Update ownership
        env.storage().persistent().set(&key, &new_owner);

        // Advance SMT root
        smt_root::SmtRoot::update_root(&env, public_signals.new_root);

        // Emit TRANSFER event
        #[allow(deprecated)]
        env.events()
            .publish((TRANSFER_EVENT,), (commitment, caller, new_owner));
    }

    /// Resolve a username hash to its primary linked Stellar address.
    ///
    /// Returns `NotFound` if the username hash is not registered.
    /// Returns `NoAddressLinked` if registered but no primary Stellar address has been set.
    pub fn resolve_stellar(env: Env, username_hash: BytesN<32>) -> Address {
        if Registration::get_owner(env.clone(), username_hash.clone()).is_none() {
            panic_with_error!(&env, CoreError::NotFound);
        }

        env.storage()
            .persistent()
            .get::<storage::DataKey, Address>(&storage::DataKey::StellarAddress(username_hash))
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NoAddressLinked))
    }
}
