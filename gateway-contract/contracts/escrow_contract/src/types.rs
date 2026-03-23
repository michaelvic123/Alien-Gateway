use soroban_sdk::{contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Vault(BytesN<32>),
    ScheduledPayment(u32),
    PaymentCounter,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultState {
    pub owner: Address,
    pub token: Address,
    pub balance: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledPayment {
    pub from: BytesN<32>,
    pub to: BytesN<32>,
    pub token: Address,
    pub amount: i128,
    pub release_at: u64,
    pub executed: bool,
}
