use soroban_sdk::{contracttype, Address, BytesN, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    CurrentMerkleRoot,
    Verifier,
    Commitment(BytesN<32>),
}

pub fn is_initialized(env: &Env) -> bool {
    env.storage().persistent().has(&DataKey::CurrentMerkleRoot)
        && env.storage().instance().has(&DataKey::Verifier)
}

pub fn get_merkle_root(env: &Env) -> Option<BytesN<32>> {
    env.storage().persistent().get(&DataKey::CurrentMerkleRoot)
}

pub fn set_merkle_root(env: &Env, root: &BytesN<32>) {
    env.storage()
        .persistent()
        .set(&DataKey::CurrentMerkleRoot, root);
}

pub fn get_verifier(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Verifier)
}

pub fn set_verifier(env: &Env, verifier: &Address) {
    env.storage().instance().set(&DataKey::Verifier, verifier);
}

pub fn has_commitment(env: &Env, commitment: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Commitment(commitment.clone()))
}

pub fn store_commitment(env: &Env, commitment: &BytesN<32>) {
    env.storage()
        .persistent()
        .set(&DataKey::Commitment(commitment.clone()), &true);
use soroban_sdk::{contracttype, BytesN};

/// Storage keys for the Core contract's persistent and instance storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Key for resolver data, indexed by commitment.
    Resolver(BytesN<32>),
    /// Key for the SMT root in instance storage.
    SmtRoot,
    /// Key for the primary Stellar address linked to a username hash.
    StellarAddress(BytesN<32>),
}
