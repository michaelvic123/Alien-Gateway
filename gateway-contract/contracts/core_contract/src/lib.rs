#![no_std]

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

use address_manager::AddressManager;
use errors::CoreError;
use events::REGISTER_EVENT;
use registration::Registration;
use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Bytes, BytesN, Env};
use types::{ChainType, PublicSignals, ResolveData};

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
    }

    /// Register a username commitment, mapping it to the caller's address.
    pub fn register(env: Env, caller: Address, commitment: BytesN<32>) {
        Registration::register(env, caller, commitment);
    }

    /// Get the owner address for a registered commitment.
    pub fn get_owner(env: Env, commitment: BytesN<32>) -> Option<Address> {
        Registration::get_owner(env, commitment)
    }

    /// Link an external chain address (EVM / Bitcoin / Solana) to a username commitment.
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
}
