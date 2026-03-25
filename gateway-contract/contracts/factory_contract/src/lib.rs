#![no_std]

mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, BytesN, Env};

use crate::errors::FactoryError;
use crate::events::emit_username_deployed;
use crate::storage::{
    get_auction_contract, get_core_contract, get_username_record, set_auction_contract,
    set_core_contract, set_username_record,
};
use crate::types::UsernameRecord;

#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    pub fn configure(env: Env, auction_contract: Address, core_contract: Address) {
        set_auction_contract(&env, &auction_contract);
        set_core_contract(&env, &core_contract);
    }

    pub fn deploy_username(env: Env, username_hash: BytesN<32>, owner: Address) {
        let auction_contract = match get_auction_contract(&env) {
            Some(address) => address,
            None => panic_with_error!(&env, FactoryError::Unauthorized),
        };
        auction_contract.require_auth();

        if get_username_record(&env, &username_hash).is_some() {
            panic_with_error!(&env, FactoryError::AlreadyDeployed);
        }

        let core_contract = match get_core_contract(&env) {
            Some(address) => address,
            None => panic_with_error!(&env, FactoryError::CoreContractNotConfigured),
        };

        let record = UsernameRecord {
            username_hash: username_hash.clone(),
            owner,
            registered_at: env.ledger().timestamp(),
            core_contract,
        };

        set_username_record(&env, &record);
        emit_username_deployed(&env, &record);
    }

    pub fn get_username_record(env: Env, username_hash: BytesN<32>) -> Option<UsernameRecord> {
        get_username_record(&env, &username_hash)
    }

    pub fn get_auction_contract(env: Env) -> Option<Address> {
        get_auction_contract(&env)
    }

    pub fn get_core_contract(env: Env) -> Option<Address> {
        get_core_contract(&env)
    }
}
