#![no_std]

use soroban_sdk::{BytesN, Env};
use spf_shared_types::{ChildExecutionId, ParentIntentId};

pub fn zero_parent_intent_id(env: &Env) -> ParentIntentId {
    ParentIntentId(BytesN::from_array(env, &[0; 32]))
}

pub fn zero_child_execution_id(env: &Env) -> ChildExecutionId {
    ChildExecutionId(BytesN::from_array(env, &[0; 32]))
}
