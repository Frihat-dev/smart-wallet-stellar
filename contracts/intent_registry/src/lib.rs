#![no_std]

mod errors;
mod storage;
mod types;

use errors::IntentRegistryError;
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};
use spf_shared_events::{event_version_v1, topic_intent};
use spf_shared_types::ParentIntentStatus;
use storage::{DataKey, RegistryConfig};
use types::RegistryStatusSnapshot;

#[contract]
pub struct IntentRegistryContract;

#[contractimpl]
impl IntentRegistryContract {
    pub fn contract_name() -> Symbol {
        topic_intent()
    }

    pub fn initialize(env: Env, owner: Address) -> Result<(), IntentRegistryError> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(IntentRegistryError::AlreadyInitialized);
        }

        owner.require_auth();

        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage().persistent().set(&DataKey::Owner, &owner);
        env.events()
            .publish((event_version_v1(), topic_intent()), RegistryConfig { owner });

        Ok(())
    }

    pub fn status(env: Env) -> Result<RegistryStatusSnapshot, IntentRegistryError> {
        if !env.storage().persistent().has(&DataKey::Initialized) {
            return Err(IntentRegistryError::NotInitialized);
        }

        Ok(RegistryStatusSnapshot {
            initialized: true,
            parent_status_template: ParentIntentStatus::Draft,
        })
    }

    pub fn create_parent_intent(_env: Env) -> Result<(), IntentRegistryError> {
        Err(IntentRegistryError::NotImplemented)
    }

    pub fn cancel_parent_intent(_env: Env) -> Result<(), IntentRegistryError> {
        Err(IntentRegistryError::NotImplemented)
    }
}
