#![no_std]

mod errors;
mod storage;
pub mod types;

use errors::PolicyEngineError;
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};
use spf_shared_events::{event_version_v1, topic_policy};
use storage::{DataKey, PolicyConfig};
use types::{ExecutionPolicy, InteractiveActionKind, PolicyStatusSnapshot};

#[contract]
pub struct PolicyEngineContract;

#[contractimpl]
impl PolicyEngineContract {
    pub fn contract_name() -> Symbol {
        topic_policy()
    }

    pub fn initialize(
        env: Env,
        owner: Address,
        current_policy_version: u32,
    ) -> Result<(), PolicyEngineError> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(PolicyEngineError::AlreadyInitialized);
        }

        owner.require_auth();

        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage().persistent().set(&DataKey::Owner, &owner);
        env.storage()
            .persistent()
            .set(&DataKey::CurrentPolicyVersion, &current_policy_version);
        env.storage()
            .persistent()
            .set(&DataKey::AllowPayments, &true);
        env.storage()
            .persistent()
            .set(&DataKey::AllowAdapters, &true);
        env.storage()
            .persistent()
            .set(&DataKey::MaxAssetRiskTier, &u32::MAX);
        env.events().publish(
            (event_version_v1(), topic_policy()),
            PolicyConfig {
                owner,
                current_policy_version,
            },
        );

        Ok(())
    }

    pub fn status(env: Env) -> Result<PolicyStatusSnapshot, PolicyEngineError> {
        let current_policy_version: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::CurrentPolicyVersion)
            .ok_or(PolicyEngineError::NotInitialized)?;

        Ok(PolicyStatusSnapshot {
            initialized: true,
            current_policy_version,
        })
    }

    pub fn validate_policy_version(
        env: Env,
        expected_policy_version: u32,
    ) -> Result<bool, PolicyEngineError> {
        let current_policy_version: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::CurrentPolicyVersion)
            .ok_or(PolicyEngineError::NotInitialized)?;

        if current_policy_version != expected_policy_version {
            return Err(PolicyEngineError::PolicyVersionMismatch);
        }

        Ok(true)
    }

    pub fn execution_policy(env: Env) -> Result<ExecutionPolicy, PolicyEngineError> {
        ensure_initialized(&env)?;
        Ok(ExecutionPolicy {
            allow_payments: env
                .storage()
                .persistent()
                .get(&DataKey::AllowPayments)
                .unwrap_or(true),
            allow_adapters: env
                .storage()
                .persistent()
                .get(&DataKey::AllowAdapters)
                .unwrap_or(true),
            max_asset_risk_tier: env
                .storage()
                .persistent()
                .get(&DataKey::MaxAssetRiskTier)
                .unwrap_or(u32::MAX),
        })
    }

    pub fn set_execution_policy(
        env: Env,
        allow_payments: bool,
        allow_adapters: bool,
        max_asset_risk_tier: u32,
    ) -> Result<(), PolicyEngineError> {
        ensure_initialized(&env)?;
        let owner: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Owner)
            .ok_or(PolicyEngineError::NotInitialized)?;
        owner.require_auth();

        env.storage()
            .persistent()
            .set(&DataKey::AllowPayments, &allow_payments);
        env.storage()
            .persistent()
            .set(&DataKey::AllowAdapters, &allow_adapters);
        env.storage()
            .persistent()
            .set(&DataKey::MaxAssetRiskTier, &max_asset_risk_tier);
        Ok(())
    }

    pub fn validate_interactive_action(
        env: Env,
        action_kind: InteractiveActionKind,
        asset_risk_tier: u32,
    ) -> Result<bool, PolicyEngineError> {
        let policy = Self::execution_policy(env)?;
        match action_kind {
            InteractiveActionKind::Payment if !policy.allow_payments => {
                return Err(PolicyEngineError::ActionNotAllowed)
            }
            InteractiveActionKind::Adapter if !policy.allow_adapters => {
                return Err(PolicyEngineError::ActionNotAllowed)
            }
            _ => {}
        }
        if asset_risk_tier > policy.max_asset_risk_tier {
            return Err(PolicyEngineError::RiskTierTooHigh);
        }
        Ok(true)
    }

    pub fn update_policy_version(_env: Env, _next_policy_version: u32) -> Result<(), PolicyEngineError> {
        Err(PolicyEngineError::NotImplemented)
    }
}

fn ensure_initialized(env: &Env) -> Result<(), PolicyEngineError> {
    if env.storage().persistent().has(&DataKey::Initialized) {
        Ok(())
    } else {
        Err(PolicyEngineError::NotInitialized)
    }
}
