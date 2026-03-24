#![cfg(test)]

use crate::errors::EscrowError;
use crate::types::{AutoPay, DataKey, ScheduledPayment, VaultState};
use crate::EscrowContract;
use crate::EscrowContractClient;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, BytesN, Env};

fn setup_test(
    env: &Env,
) -> (
    Address,
    EscrowContractClient<'_>,
    Address,
    BytesN<32>,
    BytesN<32>,
) {
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);

    let token = Address::generate(env);

    let from = BytesN::from_array(env, &[0u8; 32]);
    let to = BytesN::from_array(env, &[1u8; 32]);

    (contract_id, client, token, from, to)
}

fn create_vault(
    env: &Env,
    contract_id: &Address,
    id: &BytesN<32>,
    owner: &Address,
    token: &Address,
    balance: i128,
) {
    let vault = VaultState {
        owner: owner.clone(),
        token: token.clone(),
        balance,
    };
    env.as_contract(contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::Vault(id.clone()), &vault);
    });
}

fn read_vault(env: &Env, contract_id: &Address, id: &BytesN<32>) -> VaultState {
    env.as_contract(contract_id, || {
        env.storage()
            .persistent()
            .get(&DataKey::Vault(id.clone()))
            .unwrap()
    })
}

#[test]
fn test_schedule_payment_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, from, to) = setup_test(&env);

    let initial_balance = 1000i128;
    let amount = 400i128;
    let release_at = 2000u64;

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        initial_balance,
    );
    env.ledger().set_timestamp(1000);

    let payment_id = client.schedule_payment(&from, &to, &amount, &release_at);
    assert_eq!(payment_id, 0);

    // Verify balance decremented
    env.as_contract(&contract_id, || {
        let vault: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::Vault(from.clone()))
            .unwrap();
        assert_eq!(vault.balance, initial_balance - amount);

        // Verify ScheduledPayment stored correctly
        let payment: ScheduledPayment = env
            .storage()
            .persistent()
            .get(&DataKey::ScheduledPayment(payment_id))
            .unwrap();
        assert_eq!(payment.from, from);
        assert_eq!(payment.to, to);
        assert_eq!(payment.amount, amount);
        assert_eq!(payment.release_at, release_at);
        assert_eq!(payment.executed, false);
    });
}

#[test]
fn test_schedule_payment_past_release_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, _, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &Address::generate(&env),
        1000,
    );
    env.ledger().set_timestamp(2000);

    // release_at (1000) is in the past relative to current ledger (2000)
    let result = client.try_schedule_payment(&from, &to, &100, &1000);
    assert_eq!(result, Err(Ok(EscrowError::PastReleaseTime)));
}

#[test]
fn test_schedule_payment_insufficient_balance_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, _, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &Address::generate(&env),
        100,
    );
    env.ledger().set_timestamp(1000);

    // amount (200) > balance (100)
    let result = client.try_schedule_payment(&from, &to, &200, &2000);
    assert_eq!(result, Err(Ok(EscrowError::InsufficientBalance)));
}

#[test]
fn test_schedule_payment_returns_incrementing_ids() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        10000,
    );
    env.ledger().set_timestamp(1000);

    let id0 = client.schedule_payment(&from, &to, &100, &2000);
    let id1 = client.schedule_payment(&from, &to, &200, &3000);

    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
}

#[test]
fn test_auto_pay_setup_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1000,
    );

    client.setup_auto_pay(&from, &to, &100i128, &10u64);

    env.as_contract(&contract_id, || {
        let auto_pay: AutoPay = env
            .storage()
            .persistent()
            .get(&DataKey::AutoPay(from.clone()))
            .unwrap();
        assert_eq!(auto_pay.to, to);
        assert_eq!(auto_pay.amount, 100);
        assert_eq!(auto_pay.interval, 10);
        assert_eq!(auto_pay.last_paid, 0);
    });
}

#[test]
fn test_auto_pay_trigger_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1000,
    );
    create_vault(
        &env,
        &contract_id,
        &to,
        &Address::generate(&env),
        &token,
        50,
    );

    client.setup_auto_pay(&from, &to, &200i128, &10u64);
    env.ledger().set_timestamp(10);
    client.trigger_auto_pay(&from);

    let from_vault = read_vault(&env, &contract_id, &from);
    let to_vault = read_vault(&env, &contract_id, &to);
    assert_eq!(from_vault.balance, 800);
    assert_eq!(to_vault.balance, 250);

    env.as_contract(&contract_id, || {
        let auto_pay: AutoPay = env
            .storage()
            .persistent()
            .get(&DataKey::AutoPay(from.clone()))
            .unwrap();
        assert_eq!(auto_pay.last_paid, 10);
    });
}

#[test]
#[should_panic]
fn test_auto_pay_early_trigger_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1000,
    );
    create_vault(&env, &contract_id, &to, &Address::generate(&env), &token, 0);

    client.setup_auto_pay(&from, &to, &100i128, &10u64);
    env.ledger().set_timestamp(9);
    client.trigger_auto_pay(&from);
}

#[test]
fn test_auto_pay_second_cycle_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1000,
    );
    create_vault(&env, &contract_id, &to, &Address::generate(&env), &token, 0);

    client.setup_auto_pay(&from, &to, &150i128, &10u64);

    env.ledger().set_timestamp(10);
    client.trigger_auto_pay(&from);
    env.ledger().set_timestamp(20);
    client.trigger_auto_pay(&from);

    let from_vault = read_vault(&env, &contract_id, &from);
    let to_vault = read_vault(&env, &contract_id, &to);
    assert_eq!(from_vault.balance, 700);
    assert_eq!(to_vault.balance, 300);

    env.as_contract(&contract_id, || {
        let auto_pay: AutoPay = env
            .storage()
            .persistent()
            .get(&DataKey::AutoPay(from.clone()))
            .unwrap();
        assert_eq!(auto_pay.last_paid, 20);
    });
}

#[test]
#[should_panic]
fn test_auto_pay_insufficient_balance_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        90,
    );
    create_vault(&env, &contract_id, &to, &Address::generate(&env), &token, 0);

    client.setup_auto_pay(&from, &to, &100i128, &10u64);
    env.ledger().set_timestamp(10);
    client.trigger_auto_pay(&from);
}
