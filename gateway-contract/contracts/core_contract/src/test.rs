#![cfg(test)]

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Bytes, BytesN, Env};

use crate::types::ChainType;
use crate::{Contract, ContractClient};

fn setup(env: &Env) -> (Address, ContractClient<'_>) {
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(env, &contract_id);
    (contract_id, client)
}

fn commitment(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

// ── resolver / memo tests (from main) ────────────────────────────────────────

#[test]
fn test_resolve_returns_none_when_no_memo() {
    let env = Env::default();
    let (_, client) = setup(&env);
    let wallet = Address::generate(&env);
    let hash = commitment(&env, 0);

    client.register_resolver(&hash, &wallet, &None);

    let (resolved_wallet, memo) = client.resolve(&hash);
    assert_eq!(resolved_wallet, wallet);
    assert_eq!(memo, None);
}

#[test]
fn test_set_memo_and_resolve_flow() {
    let env = Env::default();
    let (_, client) = setup(&env);
    let wallet = Address::generate(&env);
    let hash = commitment(&env, 0);

    client.register_resolver(&hash, &wallet, &None);
    client.set_memo(&hash, &4242u64);

    let (resolved_wallet, memo) = client.resolve(&hash);
    assert_eq!(resolved_wallet, wallet);
    assert_eq!(memo, Some(4242u64));
}

// ── chain address helpers ─────────────────────────────────────────────────────

fn evm_address(env: &Env) -> Bytes {
    let raw = b"0xaAbBcCdDeEfF00112233445566778899aAbBcCdD";
    Bytes::from_slice(env, raw)
}

fn bitcoin_address(env: &Env) -> Bytes {
    let raw = b"1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf Na";
    Bytes::from_slice(env, &raw[..34])
}

fn solana_address(env: &Env) -> Bytes {
    let raw = b"So11111111111111111111111111111111111111112";
    Bytes::from_slice(env, raw)
}

// ── success cases ─────────────────────────────────────────────────────────────

#[test]
fn test_add_evm_address_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 1);
    let addr = evm_address(&env);

    client.register(&owner, &hash);
    client.add_chain_address(&owner, &hash, &ChainType::Evm, &addr);

    let stored = client.get_chain_address(&hash, &ChainType::Evm);
    assert_eq!(stored, Some(addr));
}

#[test]
fn test_add_bitcoin_address_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 2);
    let addr = bitcoin_address(&env);

    client.register(&owner, &hash);
    client.add_chain_address(&owner, &hash, &ChainType::Bitcoin, &addr);

    let stored = client.get_chain_address(&hash, &ChainType::Bitcoin);
    assert_eq!(stored, Some(addr));
}

#[test]
fn test_add_solana_address_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 3);
    let addr = solana_address(&env);

    client.register(&owner, &hash);
    client.add_chain_address(&owner, &hash, &ChainType::Solana, &addr);

    let stored = client.get_chain_address(&hash, &ChainType::Solana);
    assert_eq!(stored, Some(addr));
}

#[test]
fn test_get_chain_address_returns_none_when_not_set() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let hash = commitment(&env, 4);
    let result = client.get_chain_address(&hash, &ChainType::Evm);
    assert_eq!(result, None);
}

// ── auth / ownership failures ─────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_add_chain_address_not_registered_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let caller = Address::generate(&env);
    let hash = commitment(&env, 5);
    let addr = evm_address(&env);

    client.add_chain_address(&caller, &hash, &ChainType::Evm, &addr);
}

#[test]
#[should_panic]
fn test_add_chain_address_wrong_owner_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let hash = commitment(&env, 6);
    let addr = evm_address(&env);

    client.register(&owner, &hash);
    client.add_chain_address(&attacker, &hash, &ChainType::Evm, &addr);
}

// ── address validation failures ───────────────────────────────────────────────

#[test]
#[should_panic]
fn test_invalid_evm_address_wrong_length_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 7);

    client.register(&owner, &hash);
    let bad_addr = Bytes::from_slice(&env, b"0x1234567");
    client.add_chain_address(&owner, &hash, &ChainType::Evm, &bad_addr);
}

#[test]
#[should_panic]
fn test_invalid_evm_address_no_prefix_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 8);

    client.register(&owner, &hash);
    let bad_addr = Bytes::from_slice(&env, b"aAbBcCdDeEfF00112233445566778899aAbBcCdDeE");
    client.add_chain_address(&owner, &hash, &ChainType::Evm, &bad_addr);
}

#[test]
#[should_panic]
fn test_invalid_solana_address_too_short_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 9);

    client.register(&owner, &hash);
    let bad_addr = Bytes::from_slice(&env, b"short1234");
    client.add_chain_address(&owner, &hash, &ChainType::Solana, &bad_addr);
}
