use crate::errors::CoreError;
use crate::events::{username_registered_event, REGISTER_EVENT};
use crate::storage::{self, PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD};
use crate::types::{Proof, PublicSignals};
use crate::{smt_root, zk_verifier};
use soroban_sdk::{contracttype, panic_with_error, Address, BytesN, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Commitment(BytesN<32>),
}

pub struct Registration;

impl Registration {
    pub fn submit_proof(env: Env, caller: Address, proof: Proof, public_signals: PublicSignals) {
        caller.require_auth();

        let commitment = public_signals.commitment.clone();
        let key = DataKey::Commitment(commitment.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, CoreError::AlreadyRegistered);
        }

        let current_root = smt_root::SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet));
        if public_signals.old_root != current_root {
            panic_with_error!(&env, CoreError::StaleRoot);
        }

        if !zk_verifier::ZkVerifier::verify_groth16_proof(&env, &proof, &public_signals) {
            panic_with_error!(&env, CoreError::InvalidProof);
        }

        env.storage().persistent().set(&key, &caller);
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );

        storage::set_created_at(&env, &commitment, env.ledger().timestamp());
        smt_root::SmtRoot::update_root(&env, public_signals.new_root);

        #[allow(deprecated)]
        env.events()
            .publish((username_registered_event(&env),), commitment);
    }

    pub fn register(env: Env, caller: Address, commitment: BytesN<32>) {
        caller.require_auth();

        let key = DataKey::Commitment(commitment.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, CoreError::AlreadyRegistered);
        }

        env.storage().persistent().set(&key, &caller);
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );

        storage::set_created_at(&env, &commitment, env.ledger().timestamp());

        #[allow(deprecated)]
        env.events()
            .publish((REGISTER_EVENT,), (commitment, caller));
    }

    pub fn get_owner(env: Env, commitment: BytesN<32>) -> Option<Address> {
        let key = DataKey::Commitment(commitment);
        env.storage().persistent().get(&key)
    }

    pub fn get_created_at(env: Env, commitment: BytesN<32>) -> Option<u64> {
        storage::get_created_at(&env, &commitment)
    }
}
