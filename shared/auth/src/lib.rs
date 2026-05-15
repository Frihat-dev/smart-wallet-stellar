#![no_std]

use soroban_sdk::{contracttype, symbol_short, Symbol};

pub fn auth_domain_v1() -> Symbol {
    symbol_short!("spfauth")
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthPath {
    Interactive = 0,
    Automation = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionScope {
    Config = 0,
    SignerManagement = 1,
    InteractiveExecution = 2,
    AutomationExecution = 3,
    Recovery = 4,
    Maintenance = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthContextSummary {
    pub path: AuthPath,
    pub scope: ActionScope,
    pub nonce: u64,
    pub expires_ledger: u32,
    pub policy_version: u32,
}
