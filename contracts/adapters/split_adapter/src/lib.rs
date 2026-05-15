#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol, Vec};
use spf_shared_types::SplitRecipient;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    MaxRecipients,
    ExecutionCount,
    LastExecution,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitExecution {
    pub smart_account: Address,
    pub asset: Address,
    pub recipients: Vec<SplitRecipient>,
}

#[contract]
pub struct SplitAdapterContract;

#[contractimpl]
impl SplitAdapterContract {
    pub fn contract_name() -> Symbol {
        symbol_short!("splitadp")
    }

    pub fn initialize(env: Env, max_recipients: u32) {
        if !env.storage().persistent().has(&DataKey::MaxRecipients) {
            if max_recipients == 0 {
                panic!("max recipients must be positive");
            }
            env.storage()
                .persistent()
                .set(&DataKey::MaxRecipients, &max_recipients);
        }
    }

    pub fn max_recipients(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::MaxRecipients)
            .unwrap_or(0_u32)
    }

    pub fn execute(
        env: Env,
        smart_account: Address,
        asset: Address,
        recipients: Vec<SplitRecipient>,
    ) {
        smart_account.require_auth();
        let max_recipients = Self::max_recipients(env.clone());
        if max_recipients == 0 || recipients.is_empty() || recipients.len() > max_recipients {
            panic!("invalid split recipient count");
        }
        let seen = Vec::new(&env);
        let mut unique = seen;
        for recipient in recipients.iter() {
            if recipient.amount <= 0 {
                panic!("invalid recipient amount");
            }
            for existing in unique.iter() {
                if existing == recipient.destination {
                    panic!("duplicate recipient");
                }
            }
            unique.push_back(recipient.destination.clone());
        }
        let client = token::Client::new(&env, &asset);
        for recipient in recipients.iter() {
            client.transfer(&smart_account, &recipient.destination, &recipient.amount);
        }
        let next_count = Self::execution_count(env.clone()) + 1;
        env.storage()
            .persistent()
            .set(&DataKey::ExecutionCount, &next_count);
        env.storage().persistent().set(
            &DataKey::LastExecution,
            &SplitExecution {
                smart_account,
                asset,
                recipients,
            },
        );
    }

    pub fn execution_count(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::ExecutionCount)
            .unwrap_or(0_u32)
    }

    pub fn last_execution(env: Env) -> Option<SplitExecution> {
        env.storage().persistent().get(&DataKey::LastExecution)
    }
}
