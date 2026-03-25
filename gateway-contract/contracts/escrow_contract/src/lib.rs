//! The Escrow contract handles scheduled payments between vaults.
//! This implementation focuses on security, identity commitment, and host-level authentication.

#![no_std]

pub mod errors;
pub mod events;
pub mod storage;
pub mod types;

#[cfg(test)]
mod test;

use crate::errors::EscrowError;
use crate::events::Events;
use crate::storage::{
    increment_auto_pay_id, increment_payment_id, read_auto_pay, read_vault, write_auto_pay,
    write_scheduled_payment, write_vault,
};
use crate::types::{AutoPay, DataKey, ScheduledPayment};
use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, BytesN, Env};

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// Schedules a payment from one vault to another.
    ///
    /// Funds are reserved in the source vault immediately upon scheduling.
    /// The payment can be executed at or after the `release_at` timestamp.
    ///
    /// ### Arguments
    /// - `from`: The commitment ID of the source vault.
    /// - `to`: The commitment ID of the destination vault.
    /// - `amount`: The amount of tokens to schedule. Must be > 0.
    /// - `release_at`: The ledger timestamp (u64) for release. Must be > current time.
    ///
    /// ### Returns
    /// - `u32`: The unique payment ID assigned to this schedule.
    ///
    /// ### Errors
    /// - `VaultNotFound`: If the `from` vault does not exist.
    /// - `InvalidAmount`: If `amount <= 0`.
    /// - `InsufficientBalance`: If the vault has less than `amount`.
    /// - `PastReleaseTime`: If `release_at` is not in the future.
    /// - `PaymentCounterOverflow`: If the global ID counter overflows.
    pub fn schedule_payment(
        env: Env,
        from: BytesN<32>,
        to: BytesN<32>,
        amount: i128,
        release_at: u64,
    ) -> Result<u32, EscrowError> {
        // 1. Validate Input
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        if release_at <= env.ledger().timestamp() {
            return Err(EscrowError::PastReleaseTime);
        }

        // 2. Read Vault
        let mut vault = read_vault(&env, &from).ok_or(EscrowError::VaultNotFound)?;

        // 3. Authenticate caller as owner of from vault
        // Host-level authentication. Panics with host error if unauthorized.
        vault.owner.require_auth();

        // 4. Validate Balance
        if vault.balance < amount {
            return Err(EscrowError::InsufficientBalance);
        }

        // 5. Reserve Funds
        vault.balance -= amount;
        write_vault(&env, &from, &vault);

        // 6. Generate Payment ID
        let payment_id = increment_payment_id(&env)?;

        // 7. Store Scheduled Payment
        let payment = ScheduledPayment {
            from,
            to,
            token: vault.token.clone(),
            amount,
            release_at,
            executed: false,
        };
        write_scheduled_payment(&env, payment_id, &payment);

        // 8. Emit Event
        Events::schedule_pay(
            &env,
            payment_id,
            payment.from,
            payment.to,
            payment.amount,
            payment.release_at,
        );

        Ok(payment_id)
    }

    pub fn execute_scheduled(env: Env, payment_id: u32) {
        let key = DataKey::ScheduledPayment(payment_id);
        let mut payment: ScheduledPayment = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, EscrowError::PaymentNotFound));

        if payment.executed {
            panic_with_error!(&env, EscrowError::PaymentAlreadyExecuted);
        }

        if env.ledger().timestamp() < payment.release_at {
            panic_with_error!(&env, EscrowError::PaymentNotYetDue);
        }

        let recipient = resolve(&env, &payment.to);
        let token_client = token::Client::new(&env, &payment.token);
        token_client.transfer(&env.current_contract_address(), &recipient, &payment.amount);

        payment.executed = true;
        write_scheduled_payment(&env, payment_id, &payment);

        Events::pay_exec(&env, payment_id, payment.from, payment.to, payment.amount);
    }

    /// Registers a recurring payment rule.
    ///
    /// Once registered, calling `trigger_auto_pay` will send `amount` tokens
    /// every `interval` seconds from the sender's vault to the recipient's resolved address.
    ///
    /// ### Arguments
    /// - `from`: The commitment ID of the source vault.
    /// - `to`: The commitment ID of the destination vault.
    /// - `amount`: The amount of tokens to send each interval. Must be > 0.
    /// - `interval`: The interval in seconds between payments. Must be > 0.
    ///
    /// ### Returns
    /// - `u32`: The unique auto_pay_id assigned to this rule.
    ///
    /// ### Errors
    /// - `VaultNotFound`: If the `from` vault does not exist.
    /// - `InvalidAmount`: If `amount <= 0`.
    /// - `InvalidInterval`: If `interval <= 0`.
    /// - `AutoPayCounterOverflow`: If the global ID counter overflows.
    pub fn setup_auto_pay(
        env: Env,
        from: BytesN<32>,
        to: BytesN<32>,
        amount: i128,
        interval: u64,
    ) -> Result<u32, EscrowError> {
        // 1. Validate Input
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        if interval == 0 {
            return Err(EscrowError::InvalidInterval);
        }

        // 2. Read Vault to verify it exists and get the token
        let vault = read_vault(&env, &from).ok_or(EscrowError::VaultNotFound)?;

        // 3. Authenticate caller as owner of from vault
        // Host-level authentication. Panics with host error if unauthorized.
        vault.owner.require_auth();

        // 4. Generate AutoPay ID
        let auto_pay_id = increment_auto_pay_id(&env)?;

        // 5. Store AutoPay Rule
        let auto_pay = AutoPay {
            from: from.clone(),
            to: to.clone(),
            token: vault.token.clone(),
            amount,
            interval,
            last_paid: 0,
        };
        write_auto_pay(&env, auto_pay_id, &auto_pay);

        // 6. Emit Event
        Events::auto_set(&env, auto_pay_id, from, to, amount, interval);

        Ok(auto_pay_id)
    }

    /// Executes one cycle of a recurring auto-pay rule if enough time has passed.
    ///
    /// This function is trustless and can be called by anyone (bots, keeper scripts, SDK).
    /// It checks if the interval has elapsed since the last payment, validates the vault
    /// balance, transfers the tokens, and updates the state.
    ///
    /// ### Arguments
    /// - `auto_pay_id`: The unique identifier of the auto-pay rule to trigger.
    ///
    /// ### Errors
    /// - Panics with `AutoPayNotFound` if the auto-pay rule does not exist.
    /// - Panics with `IntervalNotElapsed` if called before the interval has elapsed.
    /// - Panics with `VaultNotFound` if the source vault does not exist.
    /// - Panics with `InsufficientBalance` if the vault balance is less than the payment amount.
    pub fn trigger_auto_pay(env: Env, auto_pay_id: u32) {
        // 1. Load AutoPay rule
        let mut auto_pay = read_auto_pay(&env, auto_pay_id)
            .unwrap_or_else(|| panic_with_error!(&env, EscrowError::AutoPayNotFound));

        // 2. Check if interval has elapsed
        let current_time = env.ledger().timestamp();
        let next_payment_time = auto_pay.last_paid + auto_pay.interval;

        if current_time < next_payment_time {
            panic_with_error!(&env, EscrowError::IntervalNotElapsed);
        }

        // 3. Load vault and check balance
        let mut vault = read_vault(&env, &auto_pay.from)
            .unwrap_or_else(|| panic_with_error!(&env, EscrowError::VaultNotFound));

        if vault.balance < auto_pay.amount {
            panic_with_error!(&env, EscrowError::InsufficientBalance);
        }

        // 4. Resolve recipient address
        let recipient = resolve(&env, &auto_pay.to);

        // 5. Transfer tokens from contract to recipient
        let token_client = token::Client::new(&env, &auto_pay.token);
        token_client.transfer(
            &env.current_contract_address(),
            &recipient,
            &auto_pay.amount,
        );

        // 6. Decrement vault balance
        vault.balance -= auto_pay.amount;
        write_vault(&env, &auto_pay.from, &vault);

        // 7. Update last_paid timestamp
        auto_pay.last_paid = current_time;
        write_auto_pay(&env, auto_pay_id, &auto_pay);

        // 8. Emit event
        Events::auto_pay(
            &env,
            auto_pay_id,
            auto_pay.from,
            auto_pay.to,
            auto_pay.amount,
            current_time,
        );
    }
}

fn resolve(env: &Env, commitment: &BytesN<32>) -> Address {
    let vault = read_vault(env, commitment)
        .unwrap_or_else(|| panic_with_error!(env, EscrowError::VaultNotFound));
    vault.owner
}
