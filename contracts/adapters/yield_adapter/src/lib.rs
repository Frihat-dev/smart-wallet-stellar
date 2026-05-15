#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Owner,
    ApprovedVault(Address),
    ExecutionCount,
    LastExecution,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YieldExecution {
    pub smart_account: Address,
    pub vault: Address,
    pub asset: Address,
    pub amount: i128,
    pub operation: u32,
}

#[contract]
pub struct YieldAdapterContract;

#[contractimpl]
impl YieldAdapterContract {
    pub fn contract_name() -> Symbol {
        symbol_short!("yieldadp")
    }

    pub fn initialize(env: Env, owner: Address) {
        if !env.storage().persistent().has(&DataKey::Owner) {
            owner.require_auth();
            env.storage().persistent().set(&DataKey::Owner, &owner);
        }
    }

    pub fn approve_vault(env: Env, vault: Address) {
        let owner: Address = env.storage().persistent().get(&DataKey::Owner).expect("owner required");
        owner.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::ApprovedVault(vault), &true);
    }

    pub fn is_vault_approved(env: Env, vault: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::ApprovedVault(vault))
            .unwrap_or(false)
    }

    pub fn execute(
        env: Env,
        smart_account: Address,
        vault: Address,
        asset: Address,
        amount: i128,
        operation: u32,
    ) {
        smart_account.require_auth();
        if !Self::is_vault_approved(env.clone(), vault.clone()) {
            panic!("vault not approved");
        }
        if amount <= 0 || operation > 1 {
            panic!("invalid yield params");
        }
        token::Client::new(&env, &asset).transfer(&smart_account, &smart_account, &amount);
        let next_count = Self::execution_count(env.clone()) + 1;
        env.storage()
            .persistent()
            .set(&DataKey::ExecutionCount, &next_count);
        env.storage().persistent().set(
            &DataKey::LastExecution,
            &YieldExecution {
                smart_account,
                vault,
                asset,
                amount,
                operation,
            },
        );
    }

    pub fn execution_count(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::ExecutionCount)
            .unwrap_or(0_u32)
    }

    pub fn last_execution(env: Env) -> Option<YieldExecution> {
        env.storage().persistent().get(&DataKey::LastExecution)
    }
}
