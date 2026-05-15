#![no_std]

use soroban_sdk::{symbol_short, Symbol};

pub const EVENT_SCHEMA_VERSION: u32 = 1;

pub fn event_version_v1() -> Symbol {
    symbol_short!("v1")
}

pub fn topic_account() -> Symbol {
    symbol_short!("account")
}

pub fn topic_intent() -> Symbol {
    symbol_short!("intent")
}

pub fn topic_policy() -> Symbol {
    symbol_short!("policy")
}

pub fn topic_status() -> Symbol {
    symbol_short!("status")
}
