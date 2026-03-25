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
use events::REGISTER_EVENT;
use registration::Registration;
use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Bytes, BytesN, Env};
use types::{ChainType, PublicSignals, ResolveData};

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
    /// Get the current SMT root.
    /// Returns the current root if set, otherwise panics with RootNotSet error.
    pub fn get_smt_root(env: Env) -> BytesN<32> {
        smt_root::SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet))
    }

    /// Register a commitment with ZK proof verification and SMT root consistency check.
    ///
    /// Steps performed (in order):
    /// 1. Authenticate `caller` via `require_auth`.
    /// 2. Reject duplicate commitments.
    /// 3. Verify `public_signals.old_root` matches the current on-chain SMT root.
    /// 4. Verify the Groth16 non-inclusion proof (Phase 4 stub — always passes for now).
    /// 5. Store the resolver record, advance the SMT root to `public_signals.new_root`,
    ///    and emit a `REGISTER` event.
    pub fn register_resolver(
        env: Env,
        caller: Address,
        commitment: BytesN<32>,
        proof: Bytes,
        public_signals: PublicSignals,
    ) {
        // 1. Auth gate
        caller.require_auth();

        // 2. Reject duplicate commitments
        let key = storage::DataKey::Resolver(commitment.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, CoreError::DuplicateCommitment);
        }

        // 3. SMT root consistency — old_root must match current on-chain root
        let current_root = smt_root::SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet));
        if public_signals.old_root != current_root {
            panic_with_error!(&env, CoreError::StaleRoot);
        }

        // 4. ZK proof verification (Phase 4 stub)
        if !zk_verifier::ZkVerifier::verify_groth16_proof(&env, &proof, &public_signals) {
            panic_with_error!(&env, CoreError::InvalidProof);
        }

        // 5a. Persist resolver record
        let data = ResolveData {
            wallet: caller.clone(),
            memo: None,
        };
        env.storage().persistent().set(&key, &data);

        // 5b. Advance SMT root
        smt_root::SmtRoot::update_root(&env, public_signals.new_root);

        // 5c. Emit REGISTER event
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

    pub fn resolve(env: Env, commitment: BytesN<32>) -> (Address, Option<u64>) {
        match env
            .storage()
            .persistent()
            .get::<storage::DataKey, ResolveData>(&storage::DataKey::Resolver(commitment))
        {
            Some(data) => (data.wallet, data.memo),
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
