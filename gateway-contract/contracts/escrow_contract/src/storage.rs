use crate::errors::EscrowError;
use crate::types::{AutoPay, DataKey, ScheduledPayment, VaultState};
use soroban_sdk::{BytesN, Env};

/// Reads a vault's state from persistent storage.
pub fn read_vault(env: &Env, from: &BytesN<32>) -> Option<VaultState> {
    env.storage()
        .persistent()
        .get(&DataKey::Vault(from.clone()))
}

/// Writes a vault's state to persistent storage.
pub fn write_vault(env: &Env, from: &BytesN<32>, vault: &VaultState) {
    env.storage()
        .persistent()
        .set(&DataKey::Vault(from.clone()), vault);
}

/// Increments the global payment counter and returns the previous ID.
///
/// ### Errors
/// - Returns `EscrowError::PaymentCounterOverflow` if the counter reaches `u32::MAX`.
pub fn increment_payment_id(env: &Env) -> Result<u32, EscrowError> {
    let id: u32 = env
        .storage()
        .instance()
        .get(&DataKey::PaymentCounter)
        .unwrap_or(0);

    let next = id
        .checked_add(1)
        .ok_or(EscrowError::PaymentCounterOverflow)?;

    env.storage()
        .instance()
        .set(&DataKey::PaymentCounter, &next);

    Ok(id)
}

/// Records a new scheduled payment in persistent storage.
pub fn write_scheduled_payment(env: &Env, id: u32, payment: &ScheduledPayment) {
    env.storage()
        .persistent()
        .set(&DataKey::ScheduledPayment(id), payment);
}

/// Increments the global auto-pay counter and returns the previous ID.
///
/// ### Errors
/// - Returns `EscrowError::AutoPayCounterOverflow` if the counter reaches `u32::MAX`.
pub fn increment_auto_pay_id(env: &Env) -> Result<u32, EscrowError> {
    let id: u32 = env
        .storage()
        .instance()
        .get(&DataKey::AutoPayCounter)
        .unwrap_or(0);

    let next = id
        .checked_add(1)
        .ok_or(EscrowError::AutoPayCounterOverflow)?;

    env.storage()
        .instance()
        .set(&DataKey::AutoPayCounter, &next);

    Ok(id)
}

/// Records a new auto-pay rule in persistent storage.
pub fn write_auto_pay(env: &Env, id: u32, auto_pay: &AutoPay) {
    env.storage()
        .persistent()
        .set(&DataKey::AutoPay(id), auto_pay);
}

/// Reads an auto-pay rule from persistent storage.
pub fn read_auto_pay(env: &Env, id: u32) -> Option<AutoPay> {
    env.storage().persistent().get(&DataKey::AutoPay(id))
}
