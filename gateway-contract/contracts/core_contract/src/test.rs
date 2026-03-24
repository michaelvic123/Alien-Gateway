#![cfg(test)]

use crate::{Contract, ContractClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, BytesN, Env, String};

// ============================================
// Test Setup Helpers
// ============================================

fn setup_test(env: &Env) -> (ContractClient<'_>, BytesN<32>, Address) {
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(env, &contract_id);
    let commitment = BytesN::from_array(env, &[7u8; 32]);
    let wallet = Address::generate(env);

    (client, commitment, wallet)
}

fn setup_with_owner(env: &Env) -> (ContractClient<'_>, Address, BytesN<32>) {
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(env, &contract_id);
    let owner = Address::generate(env);
    let commitment = BytesN::from_array(env, &[42u8; 32]);

    // Initialize address manager with owner
    client.init_address_manager(&owner);

    (client, owner, commitment)
}

// ============================================
// Resolver Tests (existing)
// ============================================

#[test]
fn test_resolve_returns_none_when_no_memo() {
    let env = Env::default();
    let (client, commitment, wallet) = setup_test(&env);

    client.register_resolver(&commitment, &wallet, &None);

    let (resolved_wallet, memo) = client.resolve(&commitment);
    assert_eq!(resolved_wallet, wallet);
    assert_eq!(memo, None);
}

#[test]
fn test_set_memo_and_resolve_flow() {
    let env = Env::default();
    let (client, commitment, wallet) = setup_test(&env);

    client.register_resolver(&commitment, &wallet, &None);
    client.set_memo(&commitment, &4242u64);

    let (resolved_wallet, memo) = client.resolve(&commitment);
    assert_eq!(resolved_wallet, wallet);
    assert_eq!(memo, Some(4242u64));
}

// ============================================
// Registration Tests
// ============================================

#[test]
fn test_register_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let caller = Address::generate(&env);
    let commitment = BytesN::from_array(&env, &[1u8; 32]);

    // Register should succeed
    client.register(&caller, &commitment);

    // Verify owner is set
    let owner = client.get_owner(&commitment);
    assert_eq!(owner, Some(caller));
}

#[test]
#[should_panic(expected = "Commitment already registered")]
fn test_register_duplicate_rejection() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let caller = Address::generate(&env);
    let commitment = BytesN::from_array(&env, &[2u8; 32]);

    // First registration succeeds
    client.register(&caller, &commitment);

    // Second registration with same commitment should panic
    client.register(&caller, &commitment);
}

#[test]
fn test_get_owner_returns_owner_after_registration() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let caller = Address::generate(&env);
    let commitment = BytesN::from_array(&env, &[3u8; 32]);

    client.register(&caller, &commitment);

    let owner = client.get_owner(&commitment);
    assert_eq!(owner, Some(caller));
}

#[test]
fn test_get_owner_returns_none_for_unknown() {
    let env = Env::default();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let unknown_commitment = BytesN::from_array(&env, &[99u8; 32]);

    let owner = client.get_owner(&unknown_commitment);
    assert_eq!(owner, None);
}

// ============================================
// Address Manager Tests
// ============================================

#[test]
fn test_set_master_stellar_address_success() {
    let env = Env::default();
    let (client, owner, commitment) = setup_with_owner(&env);
    let stellar_address = String::from_str(&env, "GXXXXXXXXXXXXXXX");

    // Register address first (required for set_master)
    client.register_address(&owner, &owner);

    // Set master address
    client.set_master_stellar_address(&owner, &commitment, &stellar_address);

    // Verify master is set
    let master = client.get_master();
    assert_eq!(master, Some(stellar_address));
}

#[test]
#[should_panic(expected = "Address not registered")]
fn test_set_master_stellar_address_not_registered() {
    let env = Env::default();
    let (client, owner, commitment) = setup_with_owner(&env);
    let stellar_address = String::from_str(&env, "GXXXXXXXXXXXXXXX");

    // Don't register address - should fail
    client.set_master_stellar_address(&owner, &commitment, &stellar_address);
}

#[test]
#[should_panic(expected = "Not owner")]
fn test_set_master_stellar_address_non_owner() {
    let env = Env::default();
    let (client, owner, commitment) = setup_with_owner(&env);
    let non_owner = Address::generate(&env);
    let stellar_address = String::from_str(&env, "GXXXXXXXXXXXXXXX");

    // Register address
    client.register_address(&owner, &owner);

    // Non-owner tries to set master - should fail
    client.set_master_stellar_address(&non_owner, &commitment, &stellar_address);
}

#[test]
fn test_add_stellar_address_success() {
    let env = Env::default();
    let (client, owner, commitment) = setup_with_owner(&env);
    let stellar_address = String::from_str(&env, "GYYYYYYYYYYYYYYYY");

    // Add stellar address
    client.add_stellar_address(&owner, &commitment, &stellar_address);

    // Verify address is stored
    let stored = client.get_stellar_address(&commitment);
    assert_eq!(stored, Some(stellar_address));
}

#[test]
#[should_panic(expected = "Address already exists")]
fn test_add_stellar_address_duplicate_rejection() {
    let env = Env::default();
    let (client, owner, commitment) = setup_with_owner(&env);
    let stellar_address = String::from_str(&env, "GZZZZZZZZZZZZZZZZ");

    // First add succeeds
    client.add_stellar_address(&owner, &commitment, &stellar_address);

    // Second add with same commitment should panic
    client.add_stellar_address(&owner, &commitment, &stellar_address);
}

#[test]
#[should_panic(expected = "Not owner")]
fn test_add_stellar_address_non_owner_rejection() {
    let env = Env::default();
    let (client, _owner, commitment) = setup_with_owner(&env);
    let non_owner = Address::generate(&env);
    let stellar_address = String::from_str(&env, "GAAAAAAAAAAAAAAAA");

    // Non-owner tries to add - should fail
    client.add_stellar_address(&non_owner, &commitment, &stellar_address);
}

#[test]
fn test_register_address_success() {
    let env = Env::default();
    let (client, owner, _commitment) = setup_with_owner(&env);
    let address_to_register = Address::generate(&env);

    // Register address
    client.register_address(&owner, &address_to_register);

    // Verify address is registered
    let is_registered = client.is_address_registered(&address_to_register);
    assert!(is_registered);
}

#[test]
#[should_panic(expected = "Not owner")]
fn test_register_address_non_owner_rejection() {
    let env = Env::default();
    let (client, _owner, _commitment) = setup_with_owner(&env);
    let non_owner = Address::generate(&env);
    let address_to_register = Address::generate(&env);

    // Non-owner tries to register - should fail
    client.register_address(&non_owner, &address_to_register);
}

#[test]
#[should_panic(expected = "Address already registered")]
fn test_register_address_duplicate_rejection() {
    let env = Env::default();
    let (client, owner, _commitment) = setup_with_owner(&env);
    let address_to_register = Address::generate(&env);

    // First registration succeeds
    client.register_address(&owner, &address_to_register);

    // Second registration should fail
    client.register_address(&owner, &address_to_register);
}

#[test]
fn test_get_master_returns_correct_address() {
    let env = Env::default();
    let (client, owner, commitment) = setup_with_owner(&env);
    let stellar_address = String::from_str(&env, "GBBBBBBBBBBBBBBB");

    // Register and set master
    client.register_address(&owner, &owner);
    client.set_master_stellar_address(&owner, &commitment, &stellar_address);

    // Verify get_master returns correct address
    let master = client.get_master();
    assert_eq!(master, Some(stellar_address));
}

#[test]
fn test_get_master_returns_none_when_not_set() {
    let env = Env::default();
    let (client, _owner, _commitment) = setup_with_owner(&env);

    // Don't set master
    let master = client.get_master();
    assert_eq!(master, None);
}

#[test]
fn test_is_address_registered_false_for_unknown() {
    let env = Env::default();
    let (client, _owner, _commitment) = setup_with_owner(&env);
    let unknown_address = Address::generate(&env);

    let is_registered = client.is_address_registered(&unknown_address);
    assert!(!is_registered);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_init_address_manager_twice_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    // First init succeeds
    client.init_address_manager(&owner);

    // Second init should fail
    client.init_address_manager(&owner);
}
