use soroban_sdk::{contracttype, Address, BytesN, Env, String};
use crate::events::{MASTER_SET, ADDR_ADDED};

#[contracttype]
#[derive(Clone)]
pub enum AddressKey {
    Owner,
    Master,
    StellarAddress(BytesN<32>),
    AddressRegistered(Address),
}

pub struct AddressManager;

impl AddressManager {
    /// Initialize the address manager with an owner.
    /// Must be called once before any other operations.
    pub fn init(env: &Env, owner: Address) {
        owner.require_auth();

        if env.storage().instance().has(&AddressKey::Owner) {
            panic!("Already initialized");
        }

        env.storage().instance().set(&AddressKey::Owner, &owner);
    }

    /// Get the owner address.
    pub fn get_owner(env: &Env) -> Option<Address> {
        env.storage().instance().get(&AddressKey::Owner)
    }

    /// Set the master Stellar address for the commitment.
    /// Only the owner can call this.
    /// The commitment must be registered first.
    pub fn set_master_stellar_address(
        env: &Env,
        caller: Address,
        commitment: BytesN<32>,
        stellar_address: String,
    ) {
        caller.require_auth();
        Self::require_owner(env, &caller);

        // Verify commitment is registered
        let addr_key = AddressKey::AddressRegistered(caller.clone());
        if !env.storage().persistent().has(&addr_key) {
            panic!("Address not registered");
        }

        let key = AddressKey::Master;
        env.storage().persistent().set(&key, &stellar_address);

        #[allow(deprecated)]
        env.events()
            .publish((MASTER_SET,), (commitment, stellar_address));
    }

    /// Add a Stellar address for a given commitment.
    /// Only the owner can call this.
    /// Rejects duplicate addresses.
    pub fn add_stellar_address(
        env: &Env,
        caller: Address,
        commitment: BytesN<32>,
        stellar_address: String,
    ) {
        caller.require_auth();
        Self::require_owner(env, &caller);

        let key = AddressKey::StellarAddress(commitment.clone());

        // Check for duplicate
        if env.storage().persistent().has(&key) {
            panic!("Address already exists");
        }

        env.storage().persistent().set(&key, &stellar_address);

        #[allow(deprecated)]
        env.events()
            .publish((ADDR_ADDED,), (commitment, stellar_address));
    }

    /// Register an address with the address manager.
    /// Only the owner can call this.
    pub fn register_address(env: &Env, caller: Address, address: Address) {
        caller.require_auth();
        Self::require_owner(env, &caller);

        let key = AddressKey::AddressRegistered(address.clone());

        if env.storage().persistent().has(&key) {
            panic!("Address already registered");
        }

        env.storage().persistent().set(&key, &true);
    }

    /// Get the master Stellar address.
    pub fn get_master(env: &Env) -> Option<String> {
        env.storage().persistent().get(&AddressKey::Master)
    }

    /// Get a Stellar address by commitment.
    pub fn get_stellar_address(env: &Env, commitment: BytesN<32>) -> Option<String> {
        let key = AddressKey::StellarAddress(commitment);
        env.storage().persistent().get(&key)
    }

    /// Check if an address is registered.
    pub fn is_registered(env: &Env, address: Address) -> bool {
        let key = AddressKey::AddressRegistered(address);
        env.storage().persistent().has(&key)
    }

    /// Internal: require caller to be owner
    fn require_owner(env: &Env, caller: &Address) {
        let owner: Address = env
            .storage()
            .instance()
            .get(&AddressKey::Owner)
            .expect("Not initialized");

        if caller != &owner {
            panic!("Not owner");
        }
    }
}
