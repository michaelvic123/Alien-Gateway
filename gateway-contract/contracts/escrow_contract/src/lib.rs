#![no_std]

mod errors;
mod events;
mod storage;
mod types;

use crate::errors::EscrowError;
use crate::events::EscrowEvents;
use crate::storage::{increment_payment_id, read_vault, write_scheduled_payment, write_vault};
use crate::types::ScheduledPayment;
use soroban_sdk::{contract, contractimpl, panic_with_error, BytesN, Env};

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    pub fn schedule_payment(
        env: Env,
        from: BytesN<32>,
        to: BytesN<32>,
        amount: i128,
        release_at: u64,
    ) -> u32 {
        // 1. Load VaultState
        let mut vault = match read_vault(&env, &from) {
            Some(v) => v,
            None => panic_with_error!(&env, EscrowError::VaultNotFound),
        };

        // 2. Authenticate caller as owner of from vault
        vault.owner.require_auth();

        // 3. amount must be > 0 and <= vault balance
        if amount <= 0 {
            panic_with_error!(&env, EscrowError::InvalidAmount);
        }
        if amount > vault.balance {
            panic_with_error!(&env, EscrowError::InsufficientBalance);
        }

        // 4. release_at must be > current env.ledger().timestamp()
        if release_at <= env.ledger().timestamp() {
            panic_with_error!(&env, EscrowError::PastReleaseTime);
        }

        // 5. Reserve amount by decrementing VaultState.balance
        vault.balance -= amount;
        write_vault(&env, &from, &vault);

        // 6. Generate a payment_id: u32
        let payment_id = increment_payment_id(&env);

        // 7. Store ScheduledPayment
        let payment = ScheduledPayment {
            from: from.clone(),
            to,
            token: vault.token.clone(),
            amount,
            release_at,
            executed: false,
        };
        write_scheduled_payment(&env, payment_id, &payment);

        // 8. Emit SCHED_PAY event
        EscrowEvents::emit_sched_pay(&env, payment_id, from, payment.to, amount, release_at);

        // 9. Return payment_id
        payment_id
    }
}
