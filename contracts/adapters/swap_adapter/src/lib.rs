#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, token, Address, BytesN, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Owner,
    ApprovedRoute(BytesN<32>),
    ExecutionCount,
    LastExecution,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapExecution {
    pub smart_account: Address,
    pub asset_in: Address,
    pub asset_out: Address,
    pub amount_in: i128,
    pub quoted_amount_out: i128,
    pub min_amount_out: i128,
    pub route_hash: BytesN<32>,
}

#[contract]
pub struct SwapAdapterContract;

#[contractimpl]
impl SwapAdapterContract {
    pub fn contract_name() -> Symbol {
        symbol_short!("swapadpt")
    }

    pub fn initialize(env: Env, owner: Address) {
        if !env.storage().persistent().has(&DataKey::Owner) {
            owner.require_auth();
            env.storage().persistent().set(&DataKey::Owner, &owner);
        }
    }

    pub fn approve_route(env: Env, route_hash: BytesN<32>) {
        let owner: Address = env.storage().persistent().get(&DataKey::Owner).expect("owner required");
        owner.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::ApprovedRoute(route_hash), &true);
    }

    pub fn is_route_approved(env: Env, route_hash: BytesN<32>) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::ApprovedRoute(route_hash))
            .unwrap_or(false)
    }

    pub fn execute(
        env: Env,
        smart_account: Address,
        asset_in: Address,
        asset_out: Address,
        amount_in: i128,
        quoted_amount_out: i128,
        min_amount_out: i128,
        route_hash: BytesN<32>,
    ) {
        smart_account.require_auth();
        if !Self::is_route_approved(env.clone(), route_hash.clone()) {
            panic!("route not approved");
        }
        if asset_in == asset_out || amount_in <= 0 || quoted_amount_out <= 0 || min_amount_out <= 0 {
            panic!("invalid swap params");
        }
        if min_amount_out > quoted_amount_out {
            panic!("invalid slippage bounds");
        }
        token::Client::new(&env, &asset_in).transfer(&smart_account, &smart_account, &amount_in);
        let next_count = Self::execution_count(env.clone()) + 1;
        env.storage()
            .persistent()
            .set(&DataKey::ExecutionCount, &next_count);
        env.storage().persistent().set(
            &DataKey::LastExecution,
            &SwapExecution {
                smart_account,
                asset_in,
                asset_out,
                amount_in,
                quoted_amount_out,
                min_amount_out,
                route_hash,
            },
        );
    }

    pub fn execution_count(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::ExecutionCount)
            .unwrap_or(0_u32)
    }

    pub fn last_execution(env: Env) -> Option<SwapExecution> {
        env.storage().persistent().get(&DataKey::LastExecution)
    }
}
