#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    ExecutionCount,
    LastExecution,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferExecution {
    pub smart_account: Address,
    pub asset: Address,
    pub destination: Address,
    pub amount: i128,
}

#[contract]
pub struct TransferAdapterContract;

#[contractimpl]
impl TransferAdapterContract {
    pub fn contract_name() -> Symbol {
        symbol_short!("xfersac")
    }

    pub fn execute(
        env: Env,
        smart_account: Address,
        asset: Address,
        destination: Address,
        amount: i128,
    ) {
        smart_account.require_auth();
        token::Client::new(&env, &asset).transfer(&smart_account, &destination, &amount);

        let next_count = Self::execution_count(env.clone()) + 1;
        env.storage()
            .persistent()
            .set(&DataKey::ExecutionCount, &next_count);
        env.storage().persistent().set(
            &DataKey::LastExecution,
            &TransferExecution {
                smart_account,
                asset,
                destination,
                amount,
            },
        );
    }

    pub fn execution_count(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::ExecutionCount)
            .unwrap_or(0_u32)
    }

    pub fn last_execution(env: Env) -> Option<TransferExecution> {
        env.storage().persistent().get(&DataKey::LastExecution)
    }
}
