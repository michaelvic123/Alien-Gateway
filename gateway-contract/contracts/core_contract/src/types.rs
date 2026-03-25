use soroban_sdk::{contracttype, Address, Symbol};

#[contracttype]
#[derive(Clone)]
pub struct AddressMetadata {
    pub label: Symbol,
}

#[contracttype]
#[derive(Clone)]
pub struct ResolveData {
    pub wallet: Address,
    pub memo: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChainType {
    Evm,
    Bitcoin,
    Solana,
    Cosmos,
}
