use soroban_sdk::{symbol_short, BytesN, Env};

pub struct EscrowEvents;

impl EscrowEvents {
    pub fn emit_sched_pay(
        env: &Env,
        payment_id: u32,
        from: BytesN<32>,
        to: BytesN<32>,
        amount: i128,
        release_at: u64,
    ) {
        let topics = (symbol_short!("SCHED_PAY"), payment_id);
        env.events().publish(topics, (from, to, amount, release_at));
    }
}
