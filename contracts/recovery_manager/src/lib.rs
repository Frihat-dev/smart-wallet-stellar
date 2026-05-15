#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Symbol};

#[contract]
pub struct RecoveryManagerContract;

#[contractimpl]
impl RecoveryManagerContract {
    pub fn contract_name() -> Symbol {
        symbol_short!("recoverm")
    }
}
