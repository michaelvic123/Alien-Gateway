use crate::types::{DataKey, ScheduledPayment, VaultState};
use soroban_sdk::{BytesN, Env};

pub fn read_vault(env: &Env, from: &BytesN<32>) -> Option<VaultState> {
    env.storage().persistent().get(&DataKey::Vault(from.clone()))
}

pub fn write_vault(env: &Env, from: &BytesN<32>, vault: &VaultState) {
    env.storage().persistent().set(&DataKey::Vault(from.clone()), vault);
}

pub fn increment_payment_id(env: &Env) -> u32 {
    let id: u32 = env.storage().instance().get(&DataKey::PaymentCounter).unwrap_or(0);
    env.storage().instance().set(&DataKey::PaymentCounter, &(id + 1));
    id
}

pub fn write_scheduled_payment(env: &Env, id: u32, payment: &ScheduledPayment) {
    env.storage().persistent().set(&DataKey::ScheduledPayment(id), payment);
}
