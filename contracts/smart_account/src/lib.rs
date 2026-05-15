#![no_std]

mod errors;
mod storage;
mod types;

use errors::SmartAccountError;
use soroban_sdk::{
    auth::{Context, ContractContext, CustomAccountInterface, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractimpl, crypto::Hash, symbol_short, Address, Bytes, BytesN, Env, IntoVal,
    Symbol, TryFromVal, Val, Vec,
};
use spf_condition_verifier::ConditionVerifierContractClient;
use spf_policy_engine::{PolicyEngineContractClient, types::InteractiveActionKind as PolicyActionKind};
use spf_split_adapter::SplitAdapterContractClient;
use spf_swap_adapter::SwapAdapterContractClient;
use spf_shared_events::{event_version_v1, topic_account, topic_status};
use spf_shared_types::{
    AdapterConfig, AssetConfig, AttestationProof, AutomationCapability, CapabilityId, ChildExecutionId,
    InteractiveAction, SessionScope, SignerKind, SignerRecord, SignerStatus, YieldOperation,
    ADAPTER_TYPE_PAYMENT, ADAPTER_TYPE_SPLIT, ADAPTER_TYPE_SWAP, ADAPTER_TYPE_YIELD,
    YIELD_OP_DEPOSIT, YIELD_OP_WITHDRAW,
    SIGNER_ROLE_ADAPTER, SIGNER_ROLE_GOVERNANCE, SIGNER_ROLE_MANAGEMENT, SIGNER_ROLE_PAYMENT,
    SIGNER_ROLE_RECOVERY, SIGNER_ROLE_SESSION_DEFAULT,
};
use spf_transfer_adapter::TransferAdapterContractClient;
use spf_yield_adapter::YieldAdapterContractClient;
use storage::{AccountConfig, DataKey};
use types::{
    AccountSignature, AccountStatusSnapshot, AutomationCapabilityState, PendingRecoveryPlan,
    StoredSession, StoredSigner,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum AdminAuthClass {
    Management,
    Governance,
    Recovery,
}

#[contract]
pub struct SmartAccountContract;

#[contractimpl]
impl SmartAccountContract {
    pub fn contract_name() -> Symbol {
        symbol_short!("smartacc")
    }

    pub fn initialize(
        env: Env,
        bootstrap_admin: Address,
        policy_engine: Address,
        intent_registry: Address,
        condition_verifier: Address,
        recovery_manager: Address,
    ) -> Result<(), SmartAccountError> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(SmartAccountError::AlreadyInitialized);
        }

        bootstrap_admin.require_auth();

        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage()
            .persistent()
            .set(&DataKey::BootstrapAdmin, &bootstrap_admin);
        env.storage()
            .persistent()
            .set(&DataKey::PrimarySignerCount, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::PrimarySignerWeightTotal, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::ManagementSignerWeightTotal, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::GovernanceSignerWeightTotal, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::RecoveryPrimaryWeightTotal, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::GuardianSignerCount, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::GuardianWeightTotal, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::ManagementThreshold, &1_u32);
        env.storage()
            .persistent()
            .set(&DataKey::GovernanceThreshold, &1_u32);
        env.storage()
            .persistent()
            .set(&DataKey::RecoveryThreshold, &1_u32);
        env.storage().persistent().set(&DataKey::Paused, &false);
        env.storage().persistent().set(&DataKey::Frozen, &false);
        env.storage()
            .persistent()
            .set(&DataKey::SignerIds, &Vec::<BytesN<32>>::new(&env));
        env.storage().persistent().set(&DataKey::PolicyVersion, &1_u32);
        env.storage()
            .persistent()
            .set(&DataKey::PolicyEngine, &policy_engine);
        env.storage()
            .persistent()
            .set(&DataKey::IntentRegistry, &intent_registry);
        env.storage()
            .persistent()
            .set(&DataKey::ConditionVerifier, &condition_verifier);
        env.storage()
            .persistent()
            .set(&DataKey::RecoveryManager, &recovery_manager);

        let config = AccountConfig {
            bootstrap_admin,
            management_threshold: 1,
            governance_threshold: 1,
            recovery_threshold: 1,
            policy_engine,
            intent_registry,
            condition_verifier,
            recovery_manager,
            policy_version: 1,
            paused: false,
            frozen: false,
        };

        env.events()
            .publish((event_version_v1(), topic_account()), config);

        Ok(())
    }

    pub fn status(env: Env) -> Result<AccountStatusSnapshot, SmartAccountError> {
        ensure_initialized(&env)?;

        Ok(AccountStatusSnapshot {
            initialized: true,
            paused: env
                .storage()
                .persistent()
                .get(&DataKey::Paused)
                .unwrap_or(false),
            frozen: env
                .storage()
                .persistent()
                .get(&DataKey::Frozen)
                .unwrap_or(false),
            policy_version: env
                .storage()
                .persistent()
                .get(&DataKey::PolicyVersion)
                .unwrap_or(1_u32),
        })
    }

    pub fn get_bootstrap_admin(env: Env) -> Result<Address, SmartAccountError> {
        ensure_initialized(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::BootstrapAdmin)
            .ok_or(SmartAccountError::NotInitialized)
    }

    pub fn get_management_threshold(env: Env) -> Result<u32, SmartAccountError> {
        ensure_initialized(&env)?;
        Ok(management_threshold(&env))
    }

    pub fn get_governance_threshold(env: Env) -> Result<u32, SmartAccountError> {
        ensure_initialized(&env)?;
        Ok(governance_threshold(&env))
    }

    pub fn get_recovery_threshold(env: Env) -> Result<u32, SmartAccountError> {
        ensure_initialized(&env)?;
        Ok(recovery_threshold(&env))
    }

    pub fn get_policy_engine(env: Env) -> Result<Address, SmartAccountError> {
        ensure_initialized(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::PolicyEngine)
            .ok_or(SmartAccountError::NotInitialized)
    }

    pub fn get_condition_verifier(env: Env) -> Result<Address, SmartAccountError> {
        ensure_initialized(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::ConditionVerifier)
            .ok_or(SmartAccountError::NotInitialized)
    }

    pub fn get_pending_recovery(env: Env) -> Result<PendingRecoveryPlan, SmartAccountError> {
        ensure_initialized(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::PendingRecovery)
            .ok_or(SmartAccountError::RecoveryNotPending)
    }

    pub fn claim_cv_owner(env: Env) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_claim_governance_auth(&env)?;

        let verifier_address = load_condition_verifier_address(&env)?;
        preauthorize_verifier_invocation(&env, &verifier_address, "apply_transfer_ownership", Vec::new(&env));
        ConditionVerifierContractClient::new(&env, &verifier_address).apply_transfer_ownership();
        Ok(())
    }

    pub fn cv_sched_add_attestor(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<u32, SmartAccountError> {
        ensure_initialized(&env)?;
        require_governance_auth(&env);
        let verifier_address = load_condition_verifier_address(&env)?;
        ensure_condition_verifier_owned_by_current_contract(&env, &verifier_address)?;
        preauthorize_verifier_invocation(
            &env,
            &verifier_address,
            "schedule_add_attestor",
            Vec::from_array(&env, [attestor.clone().into_val(&env)]),
        );
        Ok(ConditionVerifierContractClient::new(&env, &verifier_address).schedule_add_attestor(&attestor))
    }

    pub fn cv_apply_add_attestor(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_governance_auth(&env);
        let verifier_address = load_condition_verifier_address(&env)?;
        ensure_condition_verifier_owned_by_current_contract(&env, &verifier_address)?;
        preauthorize_verifier_invocation(
            &env,
            &verifier_address,
            "apply_add_attestor",
            Vec::from_array(&env, [attestor.into_val(&env)]),
        );
        ConditionVerifierContractClient::new(&env, &verifier_address).apply_add_attestor(&attestor);
        Ok(())
    }

    pub fn cv_sched_set_threshold(
        env: Env,
        threshold: u32,
    ) -> Result<u32, SmartAccountError> {
        ensure_initialized(&env)?;
        require_governance_auth(&env);
        let verifier_address = load_condition_verifier_address(&env)?;
        ensure_condition_verifier_owned_by_current_contract(&env, &verifier_address)?;
        preauthorize_verifier_invocation(
            &env,
            &verifier_address,
            "schedule_set_threshold",
            Vec::from_array(&env, [threshold.into_val(&env)]),
        );
        Ok(ConditionVerifierContractClient::new(&env, &verifier_address).schedule_set_threshold(&threshold))
    }

    pub fn cv_apply_set_threshold(env: Env) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_governance_auth(&env);
        let verifier_address = load_condition_verifier_address(&env)?;
        ensure_condition_verifier_owned_by_current_contract(&env, &verifier_address)?;
        preauthorize_verifier_invocation(&env, &verifier_address, "apply_set_threshold", Vec::new(&env));
        ConditionVerifierContractClient::new(&env, &verifier_address).apply_set_threshold();
        Ok(())
    }

    pub fn cv_sched_transfer_owner(
        env: Env,
        next_owner: Address,
    ) -> Result<u32, SmartAccountError> {
        ensure_initialized(&env)?;
        require_governance_auth(&env);
        let verifier_address = load_condition_verifier_address(&env)?;
        ensure_condition_verifier_owned_by_current_contract(&env, &verifier_address)?;
        preauthorize_verifier_invocation(
            &env,
            &verifier_address,
            "schedule_transfer_ownership",
            Vec::from_array(&env, [next_owner.clone().into_val(&env)]),
        );
        Ok(
            ConditionVerifierContractClient::new(&env, &verifier_address)
                .schedule_transfer_ownership(&next_owner),
        )
    }

    pub fn set_bootstrap_admin(
        env: Env,
        next_bootstrap_admin: Address,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;
        next_bootstrap_admin.require_auth();

        env.storage()
            .persistent()
            .set(&DataKey::BootstrapAdmin, &next_bootstrap_admin);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("bootadm"), next_bootstrap_admin),
        );

        Ok(())
    }

    pub fn set_management_threshold(
        env: Env,
        threshold: u32,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;
        validate_management_threshold(&env, threshold)?;
        env.storage()
            .persistent()
            .set(&DataKey::ManagementThreshold, &threshold);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("mgmtt"), threshold),
        );
        Ok(())
    }

    pub fn set_governance_threshold(
        env: Env,
        threshold: u32,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_governance_auth(&env);
        validate_governance_threshold(&env, threshold)?;
        env.storage()
            .persistent()
            .set(&DataKey::GovernanceThreshold, &threshold);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("govthr"), threshold),
        );
        Ok(())
    }

    pub fn set_recovery_threshold(
        env: Env,
        threshold: u32,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_governance_auth(&env);
        validate_recovery_threshold(&env, threshold)?;
        env.storage()
            .persistent()
            .set(&DataKey::RecoveryThreshold, &threshold);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("recthr"), threshold),
        );
        Ok(())
    }

    pub fn add_signer(env: Env, record: SignerRecord) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;
        validate_new_signer(&env, &record)?;

        env.storage()
            .persistent()
            .set(
                &DataKey::Signer(record.signer_id.clone()),
                &StoredSigner {
                    record: record.clone(),
                },
            );
        push_signer_id(&env, &record.signer_id)?;
        increment_signer_stats(&env, &record);
        env.events()
            .publish((event_version_v1(), topic_account()), record);

        Ok(())
    }

    pub fn set_asset_config(
        env: Env,
        asset: Address,
        config: AssetConfig,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;
        env.storage()
            .persistent()
            .set(&DataKey::AssetConfig(asset.clone()), &config);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("assetcfg"), asset, config),
        );
        Ok(())
    }

    pub fn get_asset_config(env: Env, asset: Address) -> Result<AssetConfig, SmartAccountError> {
        ensure_initialized(&env)?;
        load_asset_config(&env, &asset)
    }

    pub fn set_adapter_config(
        env: Env,
        adapter_id: BytesN<32>,
        config: AdapterConfig,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;
        env.storage()
            .persistent()
            .set(&DataKey::AdapterConfig(adapter_id.clone()), &config);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("adpcfg"), adapter_id, config),
        );
        Ok(())
    }

    pub fn get_adapter_config(
        env: Env,
        adapter_id: BytesN<32>,
    ) -> Result<AdapterConfig, SmartAccountError> {
        ensure_initialized(&env)?;
        load_adapter_config(&env, &adapter_id)
    }

    pub fn set_destination_allowed(
        env: Env,
        destination: Address,
        allowed: bool,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;
        env.storage()
            .persistent()
            .set(&DataKey::DestinationAllowlist(destination.clone()), &allowed);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("destcfg"), destination, allowed),
        );
        Ok(())
    }

    pub fn is_destination_allowed(
        env: Env,
        destination: Address,
    ) -> Result<bool, SmartAccountError> {
        ensure_initialized(&env)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::DestinationAllowlist(destination))
            .unwrap_or(false))
    }

    pub fn get_signer(env: Env, signer_id: BytesN<32>) -> Result<SignerRecord, SmartAccountError> {
        ensure_initialized(&env)?;
        load_signer(&env, &signer_id)
    }

    pub fn remove_signer(env: Env, signer_id: BytesN<32>) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;
        let record = load_signer(&env, &signer_id)?;
        if is_primary_signer_kind(&record.signer_kind) {
            if primary_signer_count(&env) <= 1 {
                return Err(SmartAccountError::LastPrimarySigner);
            }
        }
        decrement_signer_stats(&env, &record)?;

        env.storage().persistent().remove(&DataKey::Signer(signer_id.clone()));
        env.storage().persistent().remove(&DataKey::Session(signer_id.clone()));
        remove_signer_id(&env, &signer_id);
        env.events()
            .publish((event_version_v1(), topic_status()), signer_id);

        Ok(())
    }

    pub fn create_session_key(
        env: Env,
        record: SignerRecord,
        scope: SessionScope,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;

        if record.signer_kind != SignerKind::SessionKey {
            return Err(SmartAccountError::UnsupportedSignerKind);
        }
        validate_session_signer(&env, &record)?;
        validate_session_scope(&scope, env.ledger().sequence())?;
        if let Some(expires_ledger) = record.expires_ledger {
            if expires_ledger < scope.expiry_ledger {
                return Err(SmartAccountError::SessionScopeInvalid);
            }
        }

        env.storage().persistent().set(
            &DataKey::Signer(record.signer_id.clone()),
            &StoredSigner {
                record: record.clone(),
            },
        );
        push_signer_id(&env, &record.signer_id)?;
        env.storage().persistent().set(
            &DataKey::Session(record.signer_id.clone()),
            &StoredSession {
                scope: scope.clone(),
            },
        );
        env.events()
            .publish((event_version_v1(), topic_account()), scope);

        Ok(())
    }

    pub fn get_session_scope(
        env: Env,
        signer_id: BytesN<32>,
    ) -> Result<SessionScope, SmartAccountError> {
        ensure_initialized(&env)?;
        load_session_scope(&env, &signer_id)
    }

    pub fn revoke_session_key(env: Env, signer_id: BytesN<32>) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;

        let mut record = load_signer(&env, &signer_id)?;
        if record.signer_kind != SignerKind::SessionKey {
            return Err(SmartAccountError::UnsupportedSignerKind);
        }

        record.status = SignerStatus::Revoked;
        env.storage()
            .persistent()
            .set(&DataKey::Signer(signer_id.clone()), &StoredSigner { record });
        env.storage()
            .persistent()
            .remove(&DataKey::Session(signer_id.clone()));
        env.events()
            .publish((event_version_v1(), topic_status()), signer_id);

        Ok(())
    }

    pub fn grant_automation_capability(
        env: Env,
        capability: AutomationCapability,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;
        validate_capability(&env, &capability)?;
        if env
            .storage()
            .persistent()
            .has(&DataKey::Capability(capability.capability_id.0.clone()))
        {
            return Err(SmartAccountError::DuplicateCapability);
        }

        env.storage().persistent().set(
            &DataKey::Capability(capability.capability_id.0.clone()),
            &AutomationCapabilityState {
                capability: capability.clone(),
                revoked: false,
                execution_count: 0,
            },
        );
        env.events()
            .publish((event_version_v1(), topic_account()), capability);

        Ok(())
    }

    pub fn get_automation_capability(
        env: Env,
        capability_id: CapabilityId,
    ) -> Result<AutomationCapabilityState, SmartAccountError> {
        ensure_initialized(&env)?;
        load_capability(&env, &capability_id.0)
    }

    pub fn revoke_automation_capability(
        env: Env,
        capability_id: CapabilityId,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_management_auth(&env)?;

        let mut state = load_capability(&env, &capability_id.0)?;
        state.revoked = true;
        env.storage()
            .persistent()
            .set(&DataKey::Capability(capability_id.0), &state);
        Ok(())
    }

    pub fn set_policy_engine(env: Env, policy_engine: Address) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_governance_auth(&env);
        replace_policy_engine_and_bump_version(&env, policy_engine, symbol_short!("polgov"))?;
        Ok(())
    }

    pub fn recovery_set_policy_engine(
        env: Env,
        policy_engine: Address,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_mode_auth(&env)?;
        replace_policy_engine_and_bump_version(&env, policy_engine, symbol_short!("polrec"))?;
        Ok(())
    }

    pub fn recovery_disable_adapter(
        env: Env,
        adapter_id: BytesN<32>,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_mode_auth(&env)?;
        let mut config = load_adapter_config(&env, &adapter_id)?;
        config.enabled = false;
        env.storage()
            .persistent()
            .set(&DataKey::AdapterConfig(adapter_id.clone()), &config);
        bump_policy_version(&env);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("adprec"), adapter_id),
        );
        Ok(())
    }

    pub fn recovery_disable_asset(env: Env, asset: Address) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_mode_auth(&env)?;
        let mut config = load_asset_config(&env, &asset)?;
        config.enabled = false;
        env.storage()
            .persistent()
            .set(&DataKey::AssetConfig(asset.clone()), &config);
        bump_policy_version(&env);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("astrec"), asset),
        );
        Ok(())
    }

    pub fn recovery_block_destination(
        env: Env,
        destination: Address,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_mode_auth(&env)?;
        env.storage()
            .persistent()
            .set(&DataKey::DestinationAllowlist(destination.clone()), &false);
        bump_policy_version(&env);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("dstrec"), destination),
        );
        Ok(())
    }

    pub fn recovery_revoke_capability(
        env: Env,
        capability_id: CapabilityId,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_mode_auth(&env)?;
        let mut state = load_capability(&env, &capability_id.0)?;
        state.revoked = true;
        env.storage()
            .persistent()
            .set(&DataKey::Capability(capability_id.0.clone()), &state);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("caprec"), capability_id.0),
        );
        Ok(())
    }

    pub fn execute_interactive(
        env: Env,
        action: InteractiveAction,
        expected_policy_version: u32,
        signer_id: BytesN<32>,
    ) -> Result<(), SmartAccountError> {
        ensure_active(&env)?;
        require_current_policy_version(&env, expected_policy_version)?;
        let auth_args = Vec::from_array(
            &env,
            [
                action.clone().into_val(&env),
                expected_policy_version.into_val(&env),
                signer_id.clone().into_val(&env),
            ],
        );
        env.current_contract_address()
            .require_auth_for_args(auth_args);

        validate_action_against_account_policy(&env, &action)?;
        dispatch_interactive_action(&env, &action)?;

        let signer = load_signer(&env, &signer_id)?;
        if signer.signer_kind == SignerKind::SessionKey {
            apply_session_consumption(&env, &signer_id, &action)?;
        }

        env.events().publish(
            (event_version_v1(), topic_status()),
            (
                symbol_short!("interact"),
                action,
                expected_policy_version,
                signer_id,
            ),
        );
        Ok(())
    }

    pub fn execute_automation(
        env: Env,
        capability_id: CapabilityId,
        child_execution_id: ChildExecutionId,
        attestation_proof: Option<AttestationProof>,
    ) -> Result<(), SmartAccountError> {
        ensure_active(&env)?;

        let child_key = DataKey::ConsumedChildExecution(child_execution_id.0.clone());
        if env.storage().persistent().has(&child_key) {
            return Err(SmartAccountError::ExecutionReplay);
        }

        let mut state = load_capability(&env, &capability_id.0)?;
        if state.revoked {
            return Err(SmartAccountError::AutomationCapabilityRevoked);
        }

        let ledger = env.ledger().sequence();
        if ledger < state.capability.executable_from_ledger
            || ledger > state.capability.executable_until_ledger
        {
            return Err(SmartAccountError::InvalidExecutionWindow);
        }

        require_current_policy_version(&env, state.capability.policy_version)?;

        if state.execution_count >= state.capability.max_executions {
            return Err(SmartAccountError::InvalidState);
        }

        consume_required_attestation(&env, &state.capability, attestation_proof)?;

        validate_action_against_account_policy(&env, &state.capability.action)?;
        dispatch_interactive_action(&env, &state.capability.action)?;

        state.execution_count += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Capability(capability_id.0), &state);
        env.storage().persistent().set(&child_key, &true);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("automate"), child_execution_id),
        );
        Ok(())
    }

    pub fn pause(env: Env) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_auth(&env);
        env.storage().persistent().set(&DataKey::Paused, &true);
        env.events().publish(
            (event_version_v1(), topic_status()),
            symbol_short!("paused"),
        );
        Ok(())
    }

    pub fn unpause(env: Env) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_auth(&env);
        env.storage().persistent().set(&DataKey::Paused, &false);
        env.events().publish(
            (event_version_v1(), topic_status()),
            symbol_short!("unpausd"),
        );
        Ok(())
    }

    pub fn freeze(env: Env) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_auth(&env);
        env.storage().persistent().set(&DataKey::Frozen, &true);
        env.events().publish(
            (event_version_v1(), topic_status()),
            symbol_short!("frozen"),
        );
        Ok(())
    }

    pub fn initiate_recovery(
        env: Env,
        primary_signers: Vec<SignerRecord>,
        guardian_signers: Vec<SignerRecord>,
        management_threshold: u32,
        governance_threshold: u32,
        recovery_threshold: u32,
        activate_at_ledger: u32,
    ) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_auth(&env);
        if env.storage().persistent().has(&DataKey::PendingRecovery) {
            return Err(SmartAccountError::RecoveryAlreadyPending);
        }
        let plan = PendingRecoveryPlan {
            activate_at_ledger,
            primary_signers,
            guardian_signers,
            management_threshold,
            governance_threshold,
            recovery_threshold,
        };
        validate_recovery_plan(&env, &plan)?;
        env.storage().persistent().set(&DataKey::PendingRecovery, &plan);
        env.storage().persistent().set(&DataKey::Frozen, &true);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("recinit"), plan.activate_at_ledger),
        );
        Ok(())
    }

    pub fn cancel_recovery(env: Env) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_auth(&env);
        if !env.storage().persistent().has(&DataKey::PendingRecovery) {
            return Err(SmartAccountError::RecoveryNotPending);
        }
        env.storage().persistent().remove(&DataKey::PendingRecovery);
        env.events().publish(
            (event_version_v1(), topic_status()),
            symbol_short!("reccncl"),
        );
        Ok(())
    }

    pub fn finalize_recovery(env: Env) -> Result<(), SmartAccountError> {
        ensure_initialized(&env)?;
        require_recovery_auth(&env);
        let plan = load_pending_recovery(&env)?;
        if env.ledger().sequence() < plan.activate_at_ledger {
            return Err(SmartAccountError::RecoveryDelayNotElapsed);
        }

        clear_all_signers_and_sessions(&env);
        env.storage()
            .persistent()
            .set(&DataKey::PrimarySignerCount, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::PrimarySignerWeightTotal, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::ManagementSignerWeightTotal, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::GovernanceSignerWeightTotal, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::RecoveryPrimaryWeightTotal, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::GuardianSignerCount, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::GuardianWeightTotal, &0_u32);

        for record in plan.primary_signers.iter() {
            env.storage().persistent().set(
                &DataKey::Signer(record.signer_id.clone()),
                &StoredSigner {
                    record: record.clone(),
                },
            );
            push_signer_id(&env, &record.signer_id)?;
            increment_signer_stats(&env, &record);
        }
        for record in plan.guardian_signers.iter() {
            env.storage().persistent().set(
                &DataKey::Signer(record.signer_id.clone()),
                &StoredSigner {
                    record: record.clone(),
                },
            );
            push_signer_id(&env, &record.signer_id)?;
            increment_signer_stats(&env, &record);
        }

        env.storage()
            .persistent()
            .set(&DataKey::ManagementThreshold, &plan.management_threshold);
        env.storage()
            .persistent()
            .set(&DataKey::GovernanceThreshold, &plan.governance_threshold);
        env.storage()
            .persistent()
            .set(&DataKey::RecoveryThreshold, &plan.recovery_threshold);
        let next_policy_version = env
            .storage()
            .persistent()
            .get::<_, u32>(&DataKey::PolicyVersion)
            .unwrap_or(1)
            .saturating_add(1);
        env.storage()
            .persistent()
            .set(&DataKey::PolicyVersion, &next_policy_version);
        env.storage().persistent().set(&DataKey::Paused, &false);
        env.storage().persistent().set(&DataKey::Frozen, &false);
        env.storage().persistent().remove(&DataKey::PendingRecovery);
        env.events().publish(
            (event_version_v1(), topic_status()),
            (symbol_short!("recfin"), next_policy_version),
        );
        Ok(())
    }
}

fn ensure_initialized(env: &Env) -> Result<(), SmartAccountError> {
    if env.storage().persistent().has(&DataKey::Initialized) {
        Ok(())
    } else {
        Err(SmartAccountError::NotInitialized)
    }
}

#[contractimpl]
impl CustomAccountInterface for SmartAccountContract {
    type Signature = Vec<AccountSignature>;
    type Error = SmartAccountError;

    #[allow(non_snake_case)]
    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signatures: Self::Signature,
        auth_context: Vec<Context>,
    ) -> Result<(), Self::Error> {
        ensure_initialized(&env)?;

        if signatures.is_empty() {
            return Err(SmartAccountError::Unauthorized);
        }

        let interactive_context = has_execute_interactive_context(&env, &auth_context)?;
        let interactive_action = if interactive_context {
            Some(extract_interactive_action_from_auth_context(&env, &auth_context)?)
        } else {
            None
        };
        let mut saw_session_signer = false;
        let mut last_signer_id: Option<BytesN<32>> = None;
        let mut total_weight = 0_u32;

        for account_signature in signatures.iter() {
            if let Some(previous) = &last_signer_id {
                if previous >= &account_signature.signer_id {
                    return Err(SmartAccountError::BadSignatureOrder);
                }
            }

            let record = load_signer(&env, &account_signature.signer_id)?;
            ensure_signer_is_usable(&env, &record)?;

            match record.signer_kind {
                SignerKind::Ed25519 | SignerKind::SessionKey | SignerKind::Guardian => {
                    let payload_hash: BytesN<32> = signature_payload.clone().into();
                    let payload_bytes = Bytes::from_array(&env, &payload_hash.to_array());
                    env.crypto().ed25519_verify(
                        &account_signature.signer_id,
                        &payload_bytes,
                        &account_signature.signature,
                    );
                }
                _ => return Err(SmartAccountError::UnsupportedSignerKind),
            }

            if interactive_context {
                if record.signer_kind == SignerKind::SessionKey {
                    saw_session_signer = true;
                    let scope = load_session_scope(&env, &account_signature.signer_id)?;
                    validate_session_contexts(
                        &env,
                        &auth_context,
                        &account_signature.signer_id,
                        Some(&scope),
                    )?;
                } else {
                    validate_session_contexts(
                        &env,
                        &auth_context,
                        &account_signature.signer_id,
                        None,
                    )?;
                }
            } else if record.signer_kind == SignerKind::SessionKey {
                return Err(SmartAccountError::UnexpectedContext);
            }

            if interactive_context {
                if record.signer_kind != SignerKind::SessionKey
                    && record.role_bitmap
                        & required_spend_role_for_action(
                            interactive_action
                                .as_ref()
                                .ok_or(SmartAccountError::InvalidAuthPayload)?,
                        )
                        == 0
                {
                    return Err(SmartAccountError::MissingRequiredRole);
                }
            } else {
                match classify_admin_contexts(&env, &auth_context)? {
                    AdminAuthClass::Management
                        if record.role_bitmap & SIGNER_ROLE_MANAGEMENT == 0 =>
                    {
                        return Err(SmartAccountError::MissingRequiredRole);
                    }
                    AdminAuthClass::Governance
                        if record.role_bitmap & SIGNER_ROLE_GOVERNANCE == 0 =>
                    {
                        return Err(SmartAccountError::MissingRequiredRole);
                    }
                    AdminAuthClass::Recovery
                        if record.role_bitmap & SIGNER_ROLE_RECOVERY == 0 =>
                    {
                        return Err(SmartAccountError::MissingRequiredRole);
                    }
                    AdminAuthClass::Recovery
                        if guardian_signer_count(&env) > 0
                            && record.signer_kind != SignerKind::Guardian =>
                    {
                        return Err(SmartAccountError::MissingRequiredRole);
                    }
                    _ => {}
                }
            }

            total_weight = total_weight.saturating_add(record.weight);
            last_signer_id = Some(account_signature.signer_id.clone());
        }

        if interactive_context {
            if signatures.len() != 1 {
                return Err(SmartAccountError::Unauthorized);
            }
            if saw_session_signer && signatures.len() > 1 {
                return Err(SmartAccountError::UnexpectedContext);
            }
        } else {
            match classify_admin_contexts(&env, &auth_context)? {
                AdminAuthClass::Management => {
                    if total_weight < management_threshold(&env) {
                        return Err(SmartAccountError::InsufficientManagementWeight);
                    }
                }
                AdminAuthClass::Governance => {
                    if total_weight < governance_threshold(&env) {
                        return Err(SmartAccountError::InsufficientGovernanceWeight);
                    }
                }
                AdminAuthClass::Recovery => {
                    if total_weight < recovery_threshold(&env) {
                        return Err(SmartAccountError::InsufficientRecoveryWeight);
                    }
                }
            }
        }

        Ok(())
    }
}

const MAX_SCOPE_ITEMS: u32 = 16;
const SESSION_ACTION_PAYMENT: u32 = 1;
const SESSION_ACTION_ADAPTER: u32 = 1 << 1;

fn require_bootstrap_admin(env: &Env) -> Result<(), SmartAccountError> {
    let bootstrap_admin: Address = env
        .storage()
        .persistent()
        .get(&DataKey::BootstrapAdmin)
        .ok_or(SmartAccountError::NotInitialized)?;
    bootstrap_admin.require_auth();
    Ok(())
}

fn require_governance_auth(env: &Env) {
    env.current_contract_address().require_auth();
}

fn require_claim_governance_auth(env: &Env) -> Result<(), SmartAccountError> {
    if primary_signer_count(env) == 0 {
        require_bootstrap_admin(env)
    } else {
        require_governance_auth(env);
        Ok(())
    }
}

fn require_management_auth(env: &Env) -> Result<(), SmartAccountError> {
    if primary_signer_count(env) == 0 {
        require_bootstrap_admin(env)
    } else {
        require_governance_auth(env);
        Ok(())
    }
}

fn require_recovery_auth(env: &Env) {
    env.current_contract_address().require_auth();
}

fn require_recovery_mode_auth(env: &Env) -> Result<(), SmartAccountError> {
    require_recovery_auth(env);
    let frozen: bool = env.storage().persistent().get(&DataKey::Frozen).unwrap_or(false);
    let pending = env.storage().persistent().has(&DataKey::PendingRecovery);
    if !frozen && !pending {
        return Err(SmartAccountError::InvalidState);
    }
    Ok(())
}

fn require_current_policy_version(
    env: &Env,
    expected_policy_version: u32,
) -> Result<(), SmartAccountError> {
    let current_policy_version: u32 = env
        .storage()
        .persistent()
        .get(&DataKey::PolicyVersion)
        .ok_or(SmartAccountError::NotInitialized)?;
    if current_policy_version != expected_policy_version {
        return Err(SmartAccountError::PolicyVersionMismatch);
    }
    Ok(())
}

fn bump_policy_version(env: &Env) -> u32 {
    let next = env
        .storage()
        .persistent()
        .get::<_, u32>(&DataKey::PolicyVersion)
        .unwrap_or(1_u32)
        .saturating_add(1);
    env.storage()
        .persistent()
        .set(&DataKey::PolicyVersion, &next);
    next
}

fn replace_policy_engine_and_bump_version(
    env: &Env,
    policy_engine: Address,
    event_tag: Symbol,
) -> Result<(), SmartAccountError> {
    let current_policy_engine = load_policy_engine_address(env)?;
    if current_policy_engine == policy_engine {
        return Err(SmartAccountError::InvalidState);
    }
    env.storage()
        .persistent()
        .set(&DataKey::PolicyEngine, &policy_engine);
    let next_policy_version = bump_policy_version(env);
    env.events().publish(
        (event_version_v1(), topic_status()),
        (event_tag, policy_engine, next_policy_version),
    );
    Ok(())
}

fn ensure_active(env: &Env) -> Result<(), SmartAccountError> {
    ensure_initialized(env)?;

    let paused: bool = env.storage().persistent().get(&DataKey::Paused).unwrap_or(false);
    if paused {
        return Err(SmartAccountError::Paused);
    }

    let frozen: bool = env.storage().persistent().get(&DataKey::Frozen).unwrap_or(false);
    if frozen {
        return Err(SmartAccountError::Frozen);
    }

    Ok(())
}

fn validate_new_signer(env: &Env, record: &SignerRecord) -> Result<(), SmartAccountError> {
    if env
        .storage()
        .persistent()
        .has(&DataKey::Signer(record.signer_id.clone()))
    {
        return Err(SmartAccountError::DuplicateSigner);
    }
    if record.weight == 0 {
        return Err(SmartAccountError::InvalidState);
    }
    if record.signer_kind == SignerKind::SessionKey {
        return Err(SmartAccountError::SessionScopeInvalid);
    }
    if record.signer_kind == SignerKind::Guardian {
        if record.role_bitmap != SIGNER_ROLE_RECOVERY {
            return Err(SmartAccountError::MissingRequiredRole);
        }
    }
    if record.role_bitmap == 0 {
        return Err(SmartAccountError::MissingRequiredRole);
    }
    if record.status != SignerStatus::Active {
        return Err(SmartAccountError::SignerNotActive);
    }
    Ok(())
}

fn validate_session_signer(env: &Env, record: &SignerRecord) -> Result<(), SmartAccountError> {
    if env
        .storage()
        .persistent()
        .has(&DataKey::Signer(record.signer_id.clone()))
    {
        return Err(SmartAccountError::DuplicateSigner);
    }
    if record.weight == 0 {
        return Err(SmartAccountError::InvalidState);
    }
    if record.role_bitmap != SIGNER_ROLE_SESSION_DEFAULT {
        return Err(SmartAccountError::MissingRequiredRole);
    }
    if record.status != SignerStatus::Active {
        return Err(SmartAccountError::SignerNotActive);
    }
    Ok(())
}

fn increment_signer_stats(env: &Env, record: &SignerRecord) {
    if is_primary_signer_kind(&record.signer_kind) {
        let next = primary_signer_count(env) + 1;
        env.storage()
            .persistent()
            .set(&DataKey::PrimarySignerCount, &next);
        let next_weight_total = primary_signer_weight_total(env).saturating_add(record.weight);
        env.storage()
            .persistent()
            .set(&DataKey::PrimarySignerWeightTotal, &next_weight_total);
    }
    if record.role_bitmap & SIGNER_ROLE_MANAGEMENT != 0 {
        let next = management_signer_weight_total(env).saturating_add(record.weight);
        env.storage()
            .persistent()
            .set(&DataKey::ManagementSignerWeightTotal, &next);
    }
    if record.role_bitmap & SIGNER_ROLE_GOVERNANCE != 0 {
        let next = governance_signer_weight_total(env).saturating_add(record.weight);
        env.storage()
            .persistent()
            .set(&DataKey::GovernanceSignerWeightTotal, &next);
    }
    if record.signer_kind == SignerKind::Guardian {
        let next_count = guardian_signer_count(env) + 1;
        env.storage()
            .persistent()
            .set(&DataKey::GuardianSignerCount, &next_count);
        let next_weight_total = guardian_weight_total(env).saturating_add(record.weight);
        env.storage()
            .persistent()
            .set(&DataKey::GuardianWeightTotal, &next_weight_total);
    } else if record.role_bitmap & SIGNER_ROLE_RECOVERY != 0 {
        let next = recovery_primary_weight_total(env).saturating_add(record.weight);
        env.storage()
            .persistent()
            .set(&DataKey::RecoveryPrimaryWeightTotal, &next);
    }
}

fn decrement_signer_stats(env: &Env, record: &SignerRecord) -> Result<(), SmartAccountError> {
    if is_primary_signer_kind(&record.signer_kind) {
        let current = primary_signer_count(env);
        if current == 0 {
            return Err(SmartAccountError::InvalidState);
        }
        let current_weight_total = primary_signer_weight_total(env);
        if current_weight_total < record.weight {
            return Err(SmartAccountError::InvalidState);
        }
        env.storage()
            .persistent()
            .set(&DataKey::PrimarySignerCount, &(current - 1));
        env.storage()
            .persistent()
            .set(&DataKey::PrimarySignerWeightTotal, &(current_weight_total - record.weight));
    }
    if record.role_bitmap & SIGNER_ROLE_MANAGEMENT != 0 {
        let current = management_signer_weight_total(env);
        if current < record.weight || current - record.weight < management_threshold(env) {
            return Err(SmartAccountError::InsufficientManagementWeight);
        }
    }
    if record.role_bitmap & SIGNER_ROLE_GOVERNANCE != 0 {
        let current = governance_signer_weight_total(env);
        if current < record.weight || current - record.weight < governance_threshold(env) {
            return Err(SmartAccountError::InsufficientGovernanceWeight);
        }
    }
    if record.signer_kind == SignerKind::Guardian {
        let current_count = guardian_signer_count(env);
        let current_weight = guardian_weight_total(env);
        if current_count == 0 || current_weight < record.weight {
            return Err(SmartAccountError::InvalidState);
        }
        let next_effective_recovery_weight = if current_count > 1 {
            current_weight - record.weight
        } else {
            recovery_primary_weight_total(env)
        };
        if next_effective_recovery_weight < recovery_threshold(env) {
            return Err(SmartAccountError::InsufficientRecoveryWeight);
        }
    } else if record.role_bitmap & SIGNER_ROLE_RECOVERY != 0 {
        let current = recovery_primary_weight_total(env);
        if guardian_signer_count(env) == 0
            && (current < record.weight || current - record.weight < recovery_threshold(env))
        {
            return Err(SmartAccountError::InsufficientRecoveryWeight);
        }
    }

    if record.role_bitmap & SIGNER_ROLE_MANAGEMENT != 0 {
        env.storage().persistent().set(
            &DataKey::ManagementSignerWeightTotal,
            &(management_signer_weight_total(env) - record.weight),
        );
    }
    if record.role_bitmap & SIGNER_ROLE_GOVERNANCE != 0 {
        env.storage().persistent().set(
            &DataKey::GovernanceSignerWeightTotal,
            &(governance_signer_weight_total(env) - record.weight),
        );
    }
    if record.signer_kind == SignerKind::Guardian {
        env.storage()
            .persistent()
            .set(&DataKey::GuardianSignerCount, &(guardian_signer_count(env) - 1));
        env.storage()
            .persistent()
            .set(&DataKey::GuardianWeightTotal, &(guardian_weight_total(env) - record.weight));
    } else if record.role_bitmap & SIGNER_ROLE_RECOVERY != 0 {
        env.storage().persistent().set(
            &DataKey::RecoveryPrimaryWeightTotal,
            &(recovery_primary_weight_total(env) - record.weight),
        );
    }
    Ok(())
}

fn is_primary_signer_kind(kind: &SignerKind) -> bool {
    *kind != SignerKind::SessionKey && *kind != SignerKind::Guardian
}

fn primary_signer_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::PrimarySignerCount)
        .unwrap_or(0_u32)
}

fn primary_signer_weight_total(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::PrimarySignerWeightTotal)
        .unwrap_or(0_u32)
}

fn management_signer_weight_total(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::ManagementSignerWeightTotal)
        .unwrap_or(0_u32)
}

fn governance_signer_weight_total(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::GovernanceSignerWeightTotal)
        .unwrap_or(0_u32)
}

fn recovery_primary_weight_total(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::RecoveryPrimaryWeightTotal)
        .unwrap_or(0_u32)
}

fn guardian_signer_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::GuardianSignerCount)
        .unwrap_or(0_u32)
}

fn guardian_weight_total(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::GuardianWeightTotal)
        .unwrap_or(0_u32)
}

fn effective_recovery_weight_total(env: &Env) -> u32 {
    if guardian_signer_count(env) > 0 {
        guardian_weight_total(env)
    } else {
        recovery_primary_weight_total(env)
    }
}

fn management_threshold(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::ManagementThreshold)
        .unwrap_or(1_u32)
}

fn governance_threshold(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::GovernanceThreshold)
        .unwrap_or(1_u32)
}

fn recovery_threshold(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::RecoveryThreshold)
        .unwrap_or(1_u32)
}

fn validate_management_threshold(env: &Env, threshold: u32) -> Result<(), SmartAccountError> {
    if threshold == 0 || threshold > management_signer_weight_total(env) {
        return Err(SmartAccountError::InsufficientManagementWeight);
    }
    Ok(())
}

fn validate_governance_threshold(env: &Env, threshold: u32) -> Result<(), SmartAccountError> {
    if threshold == 0 || threshold > governance_signer_weight_total(env) {
        return Err(SmartAccountError::InsufficientGovernanceWeight);
    }
    Ok(())
}

fn validate_recovery_threshold(env: &Env, threshold: u32) -> Result<(), SmartAccountError> {
    if threshold == 0 || threshold > effective_recovery_weight_total(env) {
        return Err(SmartAccountError::InsufficientRecoveryWeight);
    }
    Ok(())
}

fn validate_session_scope(scope: &SessionScope, current_ledger: u32) -> Result<(), SmartAccountError> {
    if scope.allowed_action_bitmap == 0 {
        return Err(SmartAccountError::SessionScopeInvalid);
    }
    if scope.allowed_assets.len() > MAX_SCOPE_ITEMS
        || scope.allowed_destinations.len() > MAX_SCOPE_ITEMS
        || scope.allowed_adapters.len() > MAX_SCOPE_ITEMS
    {
        return Err(SmartAccountError::SessionScopeInvalid);
    }
    if scope.expiry_ledger <= current_ledger
        || scope.per_execution_cap < 0
        || scope.cumulative_cap < 0
        || scope.consumed_amount < 0
        || scope.consumed_amount > scope.cumulative_cap
    {
        return Err(SmartAccountError::SessionScopeInvalid);
    }
    Ok(())
}

fn validate_capability(env: &Env, capability: &AutomationCapability) -> Result<(), SmartAccountError> {
    let current_ledger = env.ledger().sequence();
    if capability.policy_version == 0
        || capability.max_executions == 0
        || capability.executable_from_ledger < current_ledger
        || capability.executable_until_ledger < capability.executable_from_ledger
    {
        return Err(SmartAccountError::InvalidExecutionWindow);
    }
    require_current_policy_version(env, capability.policy_version)?;
    Ok(())
}

fn validate_recovery_plan(env: &Env, plan: &PendingRecoveryPlan) -> Result<(), SmartAccountError> {
    if plan.activate_at_ledger <= env.ledger().sequence() {
        return Err(SmartAccountError::InvalidRecoveryPlan);
    }
    if plan.primary_signers.is_empty() {
        return Err(SmartAccountError::InvalidRecoveryPlan);
    }

    let mut seen = Vec::<BytesN<32>>::new(env);
    let mut management_weight = 0_u32;
    let mut governance_weight = 0_u32;
    let mut recovery_primary_weight = 0_u32;
    let mut guardian_weight = 0_u32;

    for record in plan.primary_signers.iter() {
        validate_recovery_primary_signer(env, &record)?;
        if contains_signer_id(&seen, &record.signer_id) {
            return Err(SmartAccountError::InvalidRecoveryPlan);
        }
        seen.push_back(record.signer_id.clone());
        if record.role_bitmap & SIGNER_ROLE_MANAGEMENT != 0 {
            management_weight = management_weight.saturating_add(record.weight);
        }
        if record.role_bitmap & SIGNER_ROLE_GOVERNANCE != 0 {
            governance_weight = governance_weight.saturating_add(record.weight);
        }
        if record.role_bitmap & SIGNER_ROLE_RECOVERY != 0 {
            recovery_primary_weight = recovery_primary_weight.saturating_add(record.weight);
        }
    }
    for record in plan.guardian_signers.iter() {
        validate_recovery_guardian_signer(env, &record)?;
        if contains_signer_id(&seen, &record.signer_id) {
            return Err(SmartAccountError::InvalidRecoveryPlan);
        }
        seen.push_back(record.signer_id.clone());
        guardian_weight = guardian_weight.saturating_add(record.weight);
    }

    if plan.management_threshold == 0
        || plan.management_threshold > management_weight
        || plan.governance_threshold == 0
        || plan.governance_threshold > governance_weight
    {
        return Err(SmartAccountError::InvalidRecoveryPlan);
    }
    let effective_recovery_weight = if guardian_weight > 0 {
        guardian_weight
    } else {
        recovery_primary_weight
    };
    if plan.recovery_threshold == 0 || plan.recovery_threshold > effective_recovery_weight {
        return Err(SmartAccountError::InvalidRecoveryPlan);
    }
    Ok(())
}

fn validate_recovery_primary_signer(
    env: &Env,
    record: &SignerRecord,
) -> Result<(), SmartAccountError> {
    if record.signer_kind == SignerKind::SessionKey || record.signer_kind == SignerKind::Guardian {
        return Err(SmartAccountError::InvalidRecoveryPlan);
    }
    if record.weight == 0 || record.role_bitmap == 0 || record.status != SignerStatus::Active {
        return Err(SmartAccountError::InvalidRecoveryPlan);
    }
    if let Some(expires_ledger) = record.expires_ledger {
        if expires_ledger <= env.ledger().sequence() {
            return Err(SmartAccountError::InvalidRecoveryPlan);
        }
    }
    Ok(())
}

fn validate_recovery_guardian_signer(
    env: &Env,
    record: &SignerRecord,
) -> Result<(), SmartAccountError> {
    if record.signer_kind != SignerKind::Guardian
        || record.role_bitmap != SIGNER_ROLE_RECOVERY
        || record.weight == 0
        || record.status != SignerStatus::Active
    {
        return Err(SmartAccountError::InvalidRecoveryPlan);
    }
    if let Some(expires_ledger) = record.expires_ledger {
        if expires_ledger <= env.ledger().sequence() {
            return Err(SmartAccountError::InvalidRecoveryPlan);
        }
    }
    Ok(())
}

fn load_signer(env: &Env, signer_id: &BytesN<32>) -> Result<SignerRecord, SmartAccountError> {
    env.storage()
        .persistent()
        .get::<_, StoredSigner>(&DataKey::Signer(signer_id.clone()))
        .map(|stored| stored.record)
        .ok_or(SmartAccountError::SignerNotFound)
}

fn ensure_signer_is_usable(env: &Env, record: &SignerRecord) -> Result<(), SmartAccountError> {
    if record.status != SignerStatus::Active {
        return Err(SmartAccountError::SignerNotActive);
    }
    if let Some(expires_ledger) = record.expires_ledger {
        if env.ledger().sequence() > expires_ledger {
            return Err(SmartAccountError::SignerExpired);
        }
    }
    Ok(())
}

fn load_session_scope(env: &Env, signer_id: &BytesN<32>) -> Result<SessionScope, SmartAccountError> {
    env.storage()
        .persistent()
        .get::<_, StoredSession>(&DataKey::Session(signer_id.clone()))
        .map(|stored| stored.scope)
        .ok_or(SmartAccountError::SessionScopeNotFound)
}

fn load_capability(
    env: &Env,
    capability_id: &BytesN<32>,
) -> Result<AutomationCapabilityState, SmartAccountError> {
    env.storage()
        .persistent()
        .get(&DataKey::Capability(capability_id.clone()))
        .ok_or(SmartAccountError::AutomationCapabilityNotFound)
}

fn load_asset_config(env: &Env, asset: &Address) -> Result<AssetConfig, SmartAccountError> {
    env.storage()
        .persistent()
        .get(&DataKey::AssetConfig(asset.clone()))
        .ok_or(SmartAccountError::AssetNotAllowed)
}

fn load_adapter_config(
    env: &Env,
    adapter_id: &BytesN<32>,
) -> Result<AdapterConfig, SmartAccountError> {
    env.storage()
        .persistent()
        .get(&DataKey::AdapterConfig(adapter_id.clone()))
        .ok_or(SmartAccountError::AdapterNotAllowed)
}

fn load_condition_verifier_address(env: &Env) -> Result<Address, SmartAccountError> {
    env.storage()
        .persistent()
        .get(&DataKey::ConditionVerifier)
        .ok_or(SmartAccountError::NotInitialized)
}

fn load_policy_engine_address(env: &Env) -> Result<Address, SmartAccountError> {
    env.storage()
        .persistent()
        .get(&DataKey::PolicyEngine)
        .ok_or(SmartAccountError::NotInitialized)
}

fn load_pending_recovery(env: &Env) -> Result<PendingRecoveryPlan, SmartAccountError> {
    env.storage()
        .persistent()
        .get(&DataKey::PendingRecovery)
        .ok_or(SmartAccountError::RecoveryNotPending)
}

fn ensure_condition_verifier_owned_by_current_contract(
    env: &Env,
    verifier_address: &Address,
) -> Result<(), SmartAccountError> {
    let owner = ConditionVerifierContractClient::new(env, verifier_address).get_owner();
    if owner != env.current_contract_address() {
        return Err(SmartAccountError::ConditionVerifierOwnershipRequired);
    }
    Ok(())
}

fn validate_session_contexts(
    env: &Env,
    auth_context: &Vec<Context>,
    signer_id: &BytesN<32>,
    scope: Option<&SessionScope>,
) -> Result<(), SmartAccountError> {
    if let Some(session_scope) = scope {
        if env.ledger().sequence() > session_scope.expiry_ledger {
            return Err(SmartAccountError::SignerExpired);
        }
    }
    if auth_context.is_empty() {
        return Err(SmartAccountError::UnexpectedContext);
    }

    let current_contract = env.current_contract_address();
    let execute_interactive_fn = Symbol::new(env, "execute_interactive");

    for context in auth_context.iter() {
        match context {
            Context::Contract(contract_context) => {
                if contract_context.contract != current_contract
                    || contract_context.fn_name != execute_interactive_fn
                {
                    return Err(SmartAccountError::UnexpectedContext);
                }
                validate_interactive_contract_context(
                    env,
                    &contract_context,
                    signer_id,
                    scope,
                )?;
            }
            Context::CreateContractHostFn(_) => {
                return Err(SmartAccountError::UnexpectedContext);
            }
            Context::CreateContractWithCtorHostFn(_) => {
                return Err(SmartAccountError::UnexpectedContext);
            }
        }
    }

    Ok(())
}

fn validate_interactive_contract_context(
    env: &Env,
    contract_context: &soroban_sdk::auth::ContractContext,
    signer_id: &BytesN<32>,
    scope: Option<&SessionScope>,
) -> Result<(), SmartAccountError> {
    if contract_context.args.len() != 3 {
        return Err(SmartAccountError::InvalidAuthPayload);
    }

    let action_val: Val = contract_context
        .args
        .get(0)
        .ok_or(SmartAccountError::InvalidAuthPayload)?;
    let expected_policy_version_val: Val = contract_context
        .args
        .get(1)
        .ok_or(SmartAccountError::InvalidAuthPayload)?;
    let bound_signer_id_val: Val = contract_context
        .args
        .get(2)
        .ok_or(SmartAccountError::InvalidAuthPayload)?;

    let action = InteractiveAction::try_from_val(env, &action_val)
        .map_err(|_| SmartAccountError::InvalidAuthPayload)?;
    let expected_policy_version = u32::try_from_val(env, &expected_policy_version_val)
        .map_err(|_| SmartAccountError::InvalidAuthPayload)?;
    let bound_signer_id = BytesN::<32>::try_from_val(env, &bound_signer_id_val)
        .map_err(|_| SmartAccountError::InvalidAuthPayload)?;

    require_current_policy_version(env, expected_policy_version)?;
    if &bound_signer_id != signer_id {
        return Err(SmartAccountError::SignerBindingMismatch);
    }
    if let Some(session_scope) = scope {
        validate_action_against_scope(session_scope, &action)?;
    }
    Ok(())
}

fn has_execute_interactive_context(
    env: &Env,
    auth_context: &Vec<Context>,
) -> Result<bool, SmartAccountError> {
    let current_contract = env.current_contract_address();
    let execute_interactive_fn = Symbol::new(env, "execute_interactive");

    for context in auth_context.iter() {
        match context {
            Context::Contract(contract_context) => {
                if contract_context.contract == current_contract
                    && contract_context.fn_name == execute_interactive_fn
                {
                    return Ok(true);
                }
            }
            Context::CreateContractHostFn(_) | Context::CreateContractWithCtorHostFn(_) => {}
        }
    }

    Ok(false)
}

fn extract_interactive_action_from_auth_context(
    env: &Env,
    auth_context: &Vec<Context>,
) -> Result<InteractiveAction, SmartAccountError> {
    let current_contract = env.current_contract_address();
    let execute_interactive_fn = Symbol::new(env, "execute_interactive");

    for context in auth_context.iter() {
        if let Context::Contract(contract_context) = context {
            if contract_context.contract == current_contract
                && contract_context.fn_name == execute_interactive_fn
            {
                let action_val: Val = contract_context
                    .args
                    .get(0)
                    .ok_or(SmartAccountError::InvalidAuthPayload)?;
                return InteractiveAction::try_from_val(env, &action_val)
                    .map_err(|_| SmartAccountError::InvalidAuthPayload);
            }
        }
    }

    Err(SmartAccountError::InvalidAuthPayload)
}

fn classify_admin_contexts(
    env: &Env,
    auth_context: &Vec<Context>,
) -> Result<AdminAuthClass, SmartAccountError> {
    if auth_context.is_empty() {
        return Err(SmartAccountError::UnexpectedContext);
    }
    let current_contract = env.current_contract_address();
    let execute_interactive_fn = Symbol::new(env, "execute_interactive");
    let mut class: Option<AdminAuthClass> = None;

    for context in auth_context.iter() {
        match context {
            Context::Contract(contract_context) => {
                if contract_context.contract != current_contract
                    || contract_context.fn_name == execute_interactive_fn
                {
                    return Err(SmartAccountError::UnexpectedContext);
                }
                let context_class = if is_governance_fn_name(env, &contract_context.fn_name) {
                    AdminAuthClass::Governance
                } else if is_recovery_fn_name(env, &contract_context.fn_name) {
                    AdminAuthClass::Recovery
                } else {
                    AdminAuthClass::Management
                };
                match class {
                    None => class = Some(context_class),
                    Some(previous) if previous == context_class => {}
                    Some(_) => return Err(SmartAccountError::UnexpectedContext),
                }
            }
            Context::CreateContractHostFn(_) | Context::CreateContractWithCtorHostFn(_) => {
                return Err(SmartAccountError::UnexpectedContext);
            }
        }
    }

    class.ok_or(SmartAccountError::UnexpectedContext)
}

fn is_governance_fn_name(env: &Env, fn_name: &Symbol) -> bool {
    *fn_name == Symbol::new(env, "claim_cv_owner")
        || *fn_name == Symbol::new(env, "cv_sched_add_attestor")
        || *fn_name == Symbol::new(env, "cv_apply_add_attestor")
        || *fn_name == Symbol::new(env, "cv_sched_set_threshold")
        || *fn_name == Symbol::new(env, "cv_apply_set_threshold")
        || *fn_name == Symbol::new(env, "cv_sched_transfer_owner")
        || *fn_name == Symbol::new(env, "set_policy_engine")
        || *fn_name == Symbol::new(env, "set_governance_threshold")
}

fn is_recovery_fn_name(env: &Env, fn_name: &Symbol) -> bool {
    *fn_name == Symbol::new(env, "pause")
        || *fn_name == Symbol::new(env, "unpause")
        || *fn_name == Symbol::new(env, "freeze")
        || *fn_name == Symbol::new(env, "initiate_recovery")
        || *fn_name == Symbol::new(env, "cancel_recovery")
        || *fn_name == Symbol::new(env, "finalize_recovery")
        || *fn_name == Symbol::new(env, "recovery_set_policy_engine")
        || *fn_name == Symbol::new(env, "recovery_disable_adapter")
        || *fn_name == Symbol::new(env, "recovery_disable_asset")
        || *fn_name == Symbol::new(env, "recovery_block_destination")
        || *fn_name == Symbol::new(env, "recovery_revoke_capability")
        || *fn_name == Symbol::new(env, "set_recovery_threshold")
}

fn required_spend_role_for_action(action: &InteractiveAction) -> u32 {
    match action {
        InteractiveAction::Payment(_) => SIGNER_ROLE_PAYMENT,
        InteractiveAction::Swap(_) | InteractiveAction::Yield(_) | InteractiveAction::Split(_) => {
            SIGNER_ROLE_ADAPTER
        }
    }
}

fn validate_action_against_scope(
    scope: &SessionScope,
    action: &InteractiveAction,
) -> Result<(), SmartAccountError> {
    match action {
        InteractiveAction::Payment(payment) => {
            if scope.allowed_action_bitmap & SESSION_ACTION_PAYMENT == 0 {
                return Err(SmartAccountError::ActionOutOfScope);
            }
            ensure_amount_within_scope(scope, payment.amount)?;
            ensure_address_allowed(&scope.allowed_assets, &payment.asset)?;
            ensure_address_allowed(&scope.allowed_destinations, &payment.destination)?;
        }
        InteractiveAction::Swap(adapter) => {
            if scope.allowed_action_bitmap & SESSION_ACTION_ADAPTER == 0 {
                return Err(SmartAccountError::ActionOutOfScope);
            }
            ensure_amount_within_scope(scope, adapter.amount_in)?;
            ensure_address_allowed(&scope.allowed_assets, &adapter.asset_in)?;
            ensure_address_allowed(&scope.allowed_assets, &adapter.asset_out)?;
            ensure_id_allowed(&scope.allowed_adapters, &adapter.adapter_id)?;
        }
        InteractiveAction::Yield(adapter) => {
            if scope.allowed_action_bitmap & SESSION_ACTION_ADAPTER == 0 {
                return Err(SmartAccountError::ActionOutOfScope);
            }
            ensure_amount_within_scope(scope, adapter.amount)?;
            ensure_address_allowed(&scope.allowed_assets, &adapter.asset)?;
            ensure_id_allowed(&scope.allowed_adapters, &adapter.adapter_id)?;
        }
        InteractiveAction::Split(adapter) => {
            if scope.allowed_action_bitmap & SESSION_ACTION_ADAPTER == 0 {
                return Err(SmartAccountError::ActionOutOfScope);
            }
            let total_amount = split_total_amount(&adapter.recipients)?;
            ensure_amount_within_scope(scope, total_amount)?;
            ensure_address_allowed(&scope.allowed_assets, &adapter.asset)?;
            ensure_id_allowed(&scope.allowed_adapters, &adapter.adapter_id)?;
            for recipient in adapter.recipients.iter() {
                if recipient.amount <= 0 {
                    return Err(SmartAccountError::ActionOutOfScope);
                }
                ensure_address_allowed(&scope.allowed_destinations, &recipient.destination)?;
            }
        }
    }

    Ok(())
}

fn validate_action_against_account_policy(
    env: &Env,
    action: &InteractiveAction,
) -> Result<(), SmartAccountError> {
    match action {
        InteractiveAction::Payment(payment) => {
            let asset_config = load_asset_config(env, &payment.asset)?;
            if !asset_config.enabled {
                return Err(SmartAccountError::AssetNotAllowed);
            }
            if payment.amount > asset_config.max_single_transfer {
                return Err(SmartAccountError::AmountExceedsAssetLimit);
            }
            let destination_allowed: bool = env
                .storage()
                .persistent()
                .get(&DataKey::DestinationAllowlist(payment.destination.clone()))
                .unwrap_or(false);
            if !destination_allowed {
                return Err(SmartAccountError::DestinationNotAllowed);
            }
            let transfer_adapter = load_adapter_config(env, &payment_adapter_id(env))?;
            validate_payment_adapter_config(&transfer_adapter, &payment.asset, payment.amount)?;
            validate_action_against_policy_engine(env, PolicyActionKind::Payment, asset_config.risk_tier);
        }
        InteractiveAction::Swap(adapter) => {
            let asset_in_config = load_asset_config(env, &adapter.asset_in)?;
            let asset_out_config = load_asset_config(env, &adapter.asset_out)?;
            if !asset_in_config.enabled || !asset_out_config.enabled {
                return Err(SmartAccountError::AssetNotAllowed);
            }
            if adapter.amount_in > asset_in_config.max_single_transfer {
                return Err(SmartAccountError::AmountExceedsAssetLimit);
            }
            let adapter_config = load_adapter_config(env, &adapter.adapter_id)?;
            validate_swap_action(&adapter_config, adapter)?;
            validate_action_against_policy_engine(env, PolicyActionKind::Adapter, asset_in_config.risk_tier);
        }
        InteractiveAction::Yield(adapter) => {
            let asset_config = load_asset_config(env, &adapter.asset)?;
            if !asset_config.enabled {
                return Err(SmartAccountError::AssetNotAllowed);
            }
            if adapter.amount > asset_config.max_single_transfer {
                return Err(SmartAccountError::AmountExceedsAssetLimit);
            }
            let adapter_config = load_adapter_config(env, &adapter.adapter_id)?;
            validate_yield_action(&adapter_config, adapter)?;
            validate_action_against_policy_engine(env, PolicyActionKind::Adapter, asset_config.risk_tier);
        }
        InteractiveAction::Split(adapter) => {
            let asset_config = load_asset_config(env, &adapter.asset)?;
            if !asset_config.enabled {
                return Err(SmartAccountError::AssetNotAllowed);
            }
            let total_amount = split_total_amount(&adapter.recipients)?;
            if total_amount > asset_config.max_single_transfer {
                return Err(SmartAccountError::AmountExceedsAssetLimit);
            }
            let adapter_config = load_adapter_config(env, &adapter.adapter_id)?;
            validate_split_action(env, &adapter_config, adapter)?;
            validate_action_against_policy_engine(env, PolicyActionKind::Adapter, asset_config.risk_tier);
        }
    }

    Ok(())
}

fn dispatch_interactive_action(
    env: &Env,
    action: &InteractiveAction,
) -> Result<(), SmartAccountError> {
    match action {
        InteractiveAction::Payment(payment) => {
            let transfer_adapter = load_adapter_config(env, &payment_adapter_id(env))?;
            validate_payment_adapter_config(&transfer_adapter, &payment.asset, payment.amount)?;
            let smart_account = env.current_contract_address();
            preauthorize_token_transfer(
                env,
                &payment.asset,
                &smart_account,
                &payment.destination,
                payment.amount,
            );
            TransferAdapterContractClient::new(env, &transfer_adapter.adapter_address).execute(
                &smart_account,
                &payment.asset,
                &payment.destination,
                &payment.amount,
            );
        }
        InteractiveAction::Swap(adapter) => {
            let adapter_config = load_adapter_config(env, &adapter.adapter_id)?;
            validate_swap_action(&adapter_config, adapter)?;
            let smart_account = env.current_contract_address();
            preauthorize_token_transfer(
                env,
                &adapter.asset_in,
                &smart_account,
                &smart_account,
                adapter.amount_in,
            );
            SwapAdapterContractClient::new(env, &adapter_config.adapter_address).execute(
                &smart_account,
                &adapter.asset_in,
                &adapter.asset_out,
                &adapter.amount_in,
                &adapter.quoted_amount_out,
                &adapter.min_amount_out,
                &adapter.route_hash,
            );
        }
        InteractiveAction::Yield(adapter) => {
            let adapter_config = load_adapter_config(env, &adapter.adapter_id)?;
            validate_yield_action(&adapter_config, adapter)?;
            let smart_account = env.current_contract_address();
            preauthorize_token_transfer(env, &adapter.asset, &smart_account, &smart_account, adapter.amount);
            let operation = match adapter.operation {
                YieldOperation::Deposit => 0_u32,
                YieldOperation::Withdraw => 1_u32,
            };
            YieldAdapterContractClient::new(env, &adapter_config.adapter_address).execute(
                &smart_account,
                &adapter.vault,
                &adapter.asset,
                &adapter.amount,
                &operation,
            );
        }
        InteractiveAction::Split(adapter) => {
            let adapter_config = load_adapter_config(env, &adapter.adapter_id)?;
            validate_split_action(env, &adapter_config, adapter)?;
            let smart_account = env.current_contract_address();
            for recipient in adapter.recipients.iter() {
                preauthorize_token_transfer(
                    env,
                    &adapter.asset,
                    &smart_account,
                    &recipient.destination,
                    recipient.amount,
                );
            }
            SplitAdapterContractClient::new(env, &adapter_config.adapter_address).execute(
                &smart_account,
                &adapter.asset,
                &adapter.recipients,
            );
        }
    }

    Ok(())
}

fn payment_adapter_id(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[1; 32])
}

fn preauthorize_token_transfer(
    env: &Env,
    asset: &Address,
    from: &Address,
    to: &Address,
    amount: i128,
) {
    let args = Vec::from_array(
        env,
        [
            from.clone().into_val(env),
            to.clone().into_val(env),
            amount.into_val(env),
        ],
    );
    let auth_entry = InvokerContractAuthEntry::Contract(SubContractInvocation {
        context: ContractContext {
            contract: asset.clone(),
            fn_name: Symbol::new(env, "transfer"),
            args,
        },
        sub_invocations: Vec::new(env),
    });
    env.authorize_as_current_contract(Vec::from_array(env, [auth_entry]));
}

fn preauthorize_verifier_invocation(
    env: &Env,
    verifier_address: &Address,
    fn_name: &str,
    args: Vec<Val>,
) {
    let auth_entry = InvokerContractAuthEntry::Contract(SubContractInvocation {
        context: ContractContext {
            contract: verifier_address.clone(),
            fn_name: Symbol::new(env, fn_name),
            args,
        },
        sub_invocations: Vec::new(env),
    });
    env.authorize_as_current_contract(Vec::from_array(env, [auth_entry]));
}

fn consume_required_attestation(
    env: &Env,
    capability: &AutomationCapability,
    attestation_proof: Option<AttestationProof>,
) -> Result<(), SmartAccountError> {
    match (&capability.required_attestation_id, attestation_proof) {
        (None, _) => Ok(()),
        (Some(_), None) => Err(SmartAccountError::MissingRequiredAttestation),
        (Some(required), Some(proof)) => {
            if proof.smart_account != env.current_contract_address()
                || *required != proof.attestation_id
                || proof.capability_id != capability.capability_id
            {
                return Err(SmartAccountError::MissingRequiredAttestation);
            }
            let verifier = ConditionVerifierContractClient::new(env, &load_condition_verifier_address(env)?);
            verifier.consume_attestation(&proof);
            Ok(())
        }
    }
}

fn validate_action_against_policy_engine(
    env: &Env,
    action_kind: PolicyActionKind,
    asset_risk_tier: u32,
) {
    let policy_engine: Address = env
        .storage()
        .persistent()
        .get(&DataKey::PolicyEngine)
        .expect("policy engine must be configured");
    let expected_policy_version: u32 = env
        .storage()
        .persistent()
        .get(&DataKey::PolicyVersion)
        .expect("policy version must be configured");
    let client = PolicyEngineContractClient::new(env, &policy_engine);
    client.validate_policy_version(&expected_policy_version);
    client.validate_interactive_action(&action_kind, &asset_risk_tier);
}

fn ensure_amount_within_scope(scope: &SessionScope, amount: i128) -> Result<(), SmartAccountError> {
    if amount <= 0
        || amount > scope.per_execution_cap
        || scope.consumed_amount.saturating_add(amount) > scope.cumulative_cap
    {
        return Err(SmartAccountError::ActionOutOfScope);
    }
    Ok(())
}

fn ensure_address_allowed(
    allowed: &Vec<Address>,
    target: &Address,
) -> Result<(), SmartAccountError> {
    if allowed.is_empty() {
        return Err(SmartAccountError::ActionOutOfScope);
    }

    for item in allowed.iter() {
        if item == *target {
            return Ok(());
        }
    }

    Err(SmartAccountError::ActionOutOfScope)
}

fn ensure_id_allowed(
    allowed: &Vec<BytesN<32>>,
    target: &BytesN<32>,
) -> Result<(), SmartAccountError> {
    if allowed.is_empty() {
        return Err(SmartAccountError::ActionOutOfScope);
    }

    for item in allowed.iter() {
        if item == *target {
            return Ok(());
        }
    }

    Err(SmartAccountError::ActionOutOfScope)
}

fn ensure_adapter_asset_allowed(
    allowed_assets: &Vec<Address>,
    asset: &Address,
) -> Result<(), SmartAccountError> {
    if allowed_assets.is_empty() {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    for allowed in allowed_assets.iter() {
        if allowed == *asset {
            return Ok(());
        }
    }
    Err(SmartAccountError::AdapterConstraintViolation)
}

fn validate_payment_adapter_config(
    adapter_config: &AdapterConfig,
    asset: &Address,
    amount: i128,
) -> Result<(), SmartAccountError> {
    if !adapter_config.enabled {
        return Err(SmartAccountError::AdapterNotAllowed);
    }
    if adapter_config.adapter_type != ADAPTER_TYPE_PAYMENT {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    if amount <= 0 || amount > adapter_config.max_single_execution_amount {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    ensure_adapter_asset_allowed(&adapter_config.allowed_assets, asset)
}

fn validate_adapter_config(
    adapter_config: &AdapterConfig,
    expected_adapter_type: u32,
    asset: &Address,
    amount: i128,
) -> Result<(), SmartAccountError> {
    if !adapter_config.enabled {
        return Err(SmartAccountError::AdapterNotAllowed);
    }
    if adapter_config.adapter_type != expected_adapter_type {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    if amount <= 0 || amount > adapter_config.max_single_execution_amount {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    ensure_adapter_asset_allowed(&adapter_config.allowed_assets, asset)
}

fn validate_swap_action(
    adapter_config: &AdapterConfig,
    action: &spf_shared_types::SwapAction,
) -> Result<(), SmartAccountError> {
    validate_adapter_config(
        adapter_config,
        ADAPTER_TYPE_SWAP,
        &action.asset_in,
        action.amount_in,
    )?;
    if action.asset_in == action.asset_out
        || action.quoted_amount_out <= 0
        || action.min_amount_out <= 0
        || action.min_amount_out > action.quoted_amount_out
        || action.route_hash.to_array() == [0; 32]
    {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    let slippage_numerator = action
        .quoted_amount_out
        .checked_sub(action.min_amount_out)
        .ok_or(SmartAccountError::AdapterConstraintViolation)?;
    let implied_slippage_bps = slippage_numerator
        .checked_mul(10_000)
        .ok_or(SmartAccountError::AdapterConstraintViolation)?
        / action.quoted_amount_out;
    if implied_slippage_bps as u32 > adapter_config.max_slippage_bps {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    Ok(())
}

fn validate_yield_action(
    adapter_config: &AdapterConfig,
    action: &spf_shared_types::YieldAction,
) -> Result<(), SmartAccountError> {
    validate_adapter_config(
        adapter_config,
        ADAPTER_TYPE_YIELD,
        &action.asset,
        action.amount,
    )?;
    if action.vault == action.asset {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    let op_flag = match action.operation {
        YieldOperation::Deposit => YIELD_OP_DEPOSIT,
        YieldOperation::Withdraw => YIELD_OP_WITHDRAW,
    };
    if adapter_config.allowed_yield_operations & op_flag == 0 {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    Ok(())
}

fn validate_split_action(
    env: &Env,
    adapter_config: &AdapterConfig,
    action: &spf_shared_types::SplitAction,
) -> Result<(), SmartAccountError> {
    let total_amount = split_total_amount(&action.recipients)?;
    validate_adapter_config(
        adapter_config,
        ADAPTER_TYPE_SPLIT,
        &action.asset,
        total_amount,
    )?;
    if action.recipients.len() > adapter_config.max_split_recipients
        || action.recipients.is_empty()
    {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    let mut seen = Vec::<Address>::new(env);
    for recipient in action.recipients.iter() {
        let destination_allowed: bool = env
            .storage()
            .persistent()
            .get(&DataKey::DestinationAllowlist(recipient.destination.clone()))
            .unwrap_or(false);
        if !destination_allowed || recipient.amount <= 0 {
            return Err(SmartAccountError::DestinationNotAllowed);
        }
        for existing in seen.iter() {
            if existing == recipient.destination {
                return Err(SmartAccountError::AdapterConstraintViolation);
            }
        }
        seen.push_back(recipient.destination);
    }
    Ok(())
}

fn split_total_amount(
    recipients: &Vec<spf_shared_types::SplitRecipient>,
) -> Result<i128, SmartAccountError> {
    if recipients.is_empty() {
        return Err(SmartAccountError::AdapterConstraintViolation);
    }
    let mut total = 0_i128;
    for recipient in recipients.iter() {
        if recipient.amount <= 0 {
            return Err(SmartAccountError::AdapterConstraintViolation);
        }
        total = total
            .checked_add(recipient.amount)
            .ok_or(SmartAccountError::AdapterConstraintViolation)?;
    }
    Ok(total)
}

fn action_amount(action: &InteractiveAction) -> i128 {
    match action {
        InteractiveAction::Payment(payment) => payment.amount,
        InteractiveAction::Swap(adapter) => adapter.amount_in,
        InteractiveAction::Yield(adapter) => adapter.amount,
        InteractiveAction::Split(adapter) => split_total_amount(&adapter.recipients).unwrap_or(i128::MAX),
    }
}

fn apply_session_consumption(
    env: &Env,
    signer_id: &BytesN<32>,
    action: &InteractiveAction,
) -> Result<(), SmartAccountError> {
    let mut stored = env
        .storage()
        .persistent()
        .get::<_, StoredSession>(&DataKey::Session(signer_id.clone()))
        .ok_or(SmartAccountError::SessionScopeNotFound)?;
    let amount = action_amount(action);

    validate_action_against_scope(&stored.scope, action)?;
    stored.scope.consumed_amount = stored.scope.consumed_amount.saturating_add(amount);
    env.storage()
        .persistent()
        .set(&DataKey::Session(signer_id.clone()), &stored);

    if stored.scope.single_use || stored.scope.consumed_amount >= stored.scope.cumulative_cap {
        let mut record = load_signer(env, signer_id)?;
        record.status = SignerStatus::Revoked;
        env.storage()
            .persistent()
            .set(&DataKey::Signer(signer_id.clone()), &StoredSigner { record });
        env.storage()
            .persistent()
            .remove(&DataKey::Session(signer_id.clone()));
    }

    Ok(())
}

fn load_signer_ids(env: &Env) -> Vec<BytesN<32>> {
    env.storage()
        .persistent()
        .get(&DataKey::SignerIds)
        .unwrap_or(Vec::new(env))
}

fn contains_signer_id(ids: &Vec<BytesN<32>>, signer_id: &BytesN<32>) -> bool {
    for existing in ids.iter() {
        if existing == *signer_id {
            return true;
        }
    }
    false
}

fn push_signer_id(env: &Env, signer_id: &BytesN<32>) -> Result<(), SmartAccountError> {
    let mut ids = load_signer_ids(env);
    if contains_signer_id(&ids, signer_id) {
        return Err(SmartAccountError::DuplicateSigner);
    }
    ids.push_back(signer_id.clone());
    env.storage().persistent().set(&DataKey::SignerIds, &ids);
    Ok(())
}

fn remove_signer_id(env: &Env, signer_id: &BytesN<32>) {
    let ids = load_signer_ids(env);
    let filtered = Vec::new(env);
    let mut next = filtered;
    for existing in ids.iter() {
        if existing != *signer_id {
            next.push_back(existing);
        }
    }
    env.storage().persistent().set(&DataKey::SignerIds, &next);
}

fn clear_all_signers_and_sessions(env: &Env) {
    let ids = load_signer_ids(env);
    for signer_id in ids.iter() {
        env.storage().persistent().remove(&DataKey::Signer(signer_id.clone()));
        env.storage().persistent().remove(&DataKey::Session(signer_id));
    }
    env.storage()
        .persistent()
        .set(&DataKey::SignerIds, &Vec::<BytesN<32>>::new(env));
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::{rngs::StdRng, SeedableRng};
    use spf_condition_verifier::{build_attestation_payload, ConditionVerifierContract, ConditionVerifierContractClient};
    use spf_policy_engine::{PolicyEngineContract, PolicyEngineContractClient};
    use spf_shared_types::{
        AttestationProof, AttestationSignature, PaymentAction, SplitAction, SplitRecipient,
        SwapAction, YieldAction,
    };
    use spf_split_adapter::{SplitAdapterContract, SplitAdapterContractClient};
    use spf_swap_adapter::{SwapAdapterContract, SwapAdapterContractClient};
    use spf_transfer_adapter::TransferAdapterContract;
    use spf_yield_adapter::{YieldAdapterContract, YieldAdapterContractClient};
    use soroban_sdk::{
        auth::{Context, ContractContext},
        testutils::{Address as _, Ledger, StellarAssetContract},
        token, vec, IntoVal,
    };

    fn setup_contract(env: &Env) -> (Address, SmartAccountContractClient<'_>) {
        let contract_id = env.register(SmartAccountContract, ());
        let client = SmartAccountContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let policy_engine_id = env.register(PolicyEngineContract, ());
        let policy_engine_client = PolicyEngineContractClient::new(env, &policy_engine_id);
        policy_engine_client.initialize(&admin, &1_u32);
        let condition_verifier_id = env.register(ConditionVerifierContract, ());
        let condition_verifier_client =
            spf_condition_verifier::ConditionVerifierContractClient::new(env, &condition_verifier_id);
        condition_verifier_client.initialize(&admin);
        let peer = Address::generate(env);
        let recovery = Address::generate(env);

        client.initialize(&admin, &policy_engine_id, &peer, &condition_verifier_id, &recovery);
        (admin, client)
    }

    fn signing_key(seed: u8) -> SigningKey {
        let mut rng = StdRng::from_seed([seed; 32]);
        SigningKey::generate(&mut rng)
    }

    fn issue_test_asset(env: &Env) -> StellarAssetContract {
        env.register_stellar_asset_contract_v2(Address::generate(env))
    }

    fn ed25519_signer_record(
        env: &Env,
        signer_id: BytesN<32>,
        weight: u32,
        metadata_seed: u8,
    ) -> SignerRecord {
        SignerRecord {
            signer_id,
            signer_kind: SignerKind::Ed25519,
            role_bitmap: spf_shared_types::SIGNER_ROLE_FULL_PRIMARY,
            status: SignerStatus::Active,
            weight,
            created_ledger: env.ledger().sequence(),
            expires_ledger: Some(env.ledger().sequence() + 50),
            metadata_hash: BytesN::from_array(env, &[metadata_seed; 32]),
        }
    }

    fn session_signer_record(
        env: &Env,
        signer_id: BytesN<32>,
        metadata_seed: u8,
    ) -> SignerRecord {
        SignerRecord {
            signer_id,
            signer_kind: SignerKind::SessionKey,
            role_bitmap: SIGNER_ROLE_SESSION_DEFAULT,
            status: SignerStatus::Active,
            weight: 1,
            created_ledger: env.ledger().sequence(),
            expires_ledger: Some(env.ledger().sequence() + 50),
            metadata_hash: BytesN::from_array(env, &[metadata_seed; 32]),
        }
    }

    fn guardian_signer_record(
        env: &Env,
        signer_id: BytesN<32>,
        weight: u32,
        metadata_seed: u8,
    ) -> SignerRecord {
        SignerRecord {
            signer_id,
            signer_kind: SignerKind::Guardian,
            role_bitmap: SIGNER_ROLE_RECOVERY,
            status: SignerStatus::Active,
            weight,
            created_ledger: env.ledger().sequence(),
            expires_ledger: Some(env.ledger().sequence() + 50),
            metadata_hash: BytesN::from_array(env, &[metadata_seed; 32]),
        }
    }

    fn sign_attestation_proof(
        env: &Env,
        smart_account: &Address,
        verifier_contract: &Address,
        keys: &[&SigningKey],
        attestation_id: BytesN<32>,
        capability_id: CapabilityId,
        expires_ledger: u32,
    ) -> AttestationProof {
        let payload = build_attestation_payload(
            env,
            smart_account,
            verifier_contract,
            &attestation_id,
            &capability_id,
            expires_ledger,
        );
        let mut payload_vec = std::vec::Vec::new();
        for byte in payload.iter() {
            payload_vec.push(byte);
        }
        let mut signatures = soroban_sdk::Vec::new(env);
        for key in keys {
            let signature = key.sign(&payload_vec);
            signatures.push_back(AttestationSignature {
                attestor: BytesN::from_array(env, &key.verifying_key().to_bytes()),
                signature: BytesN::from_array(env, &signature.to_bytes()),
            });
        }
        AttestationProof {
            smart_account: smart_account.clone(),
            attestation_id,
            capability_id,
            expires_ledger,
            signatures,
        }
    }

    fn activate_verifier_attestor(
        env: &Env,
        verifier: &ConditionVerifierContractClient<'_>,
        attestor: BytesN<32>,
    ) {
        let ready_at = verifier.schedule_add_attestor(&attestor);
        env.ledger().with_mut(|li| {
            li.sequence_number = ready_at;
        });
        verifier.apply_add_attestor(&attestor);
    }

    fn management_auth_context(
        env: &Env,
        client: &SmartAccountContractClient<'_>,
    ) -> Vec<Context> {
        vec![
            env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(env, "set_destination_allowed"),
                args: Vec::from_array(
                    env,
                    [Address::generate(env).into_val(env), true.into_val(env)],
                ),
            }),
        ]
    }

    fn governance_auth_context(
        env: &Env,
        client: &SmartAccountContractClient<'_>,
    ) -> Vec<Context> {
        vec![
            env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(env, "cv_sched_set_threshold"),
                args: Vec::from_array(env, [1_u32.into_val(env)]),
            }),
        ]
    }

    fn governance_policy_engine_auth_context(
        env: &Env,
        client: &SmartAccountContractClient<'_>,
    ) -> Vec<Context> {
        vec![
            env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(env, "set_policy_engine"),
                args: Vec::from_array(env, [Address::generate(env).into_val(env)]),
            }),
        ]
    }

    fn recovery_auth_context(
        env: &Env,
        client: &SmartAccountContractClient<'_>,
    ) -> Vec<Context> {
        vec![
            env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(env, "pause"),
                args: Vec::new(env),
            }),
        ]
    }

    #[test]
    fn smart_account_can_claim_and_proxy_condition_verifier_governance() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let verifier_address = client.get_condition_verifier();
        let verifier = ConditionVerifierContractClient::new(&env, &verifier_address);
        let attestor_key = signing_key(91);
        let attestor = BytesN::from_array(&env, &attestor_key.verifying_key().to_bytes());

        let transfer_ready_at = verifier.schedule_transfer_ownership(&client.address);
        env.ledger().with_mut(|li| {
            li.sequence_number = transfer_ready_at;
        });
        client.claim_cv_owner();
        assert_eq!(verifier.get_owner(), client.address);

        let add_ready_at = client.cv_sched_add_attestor(&attestor);
        assert_eq!(
            verifier
                .get_pending_add_attestor(&attestor)
                .unwrap()
                .activate_at_ledger,
            add_ready_at
        );

        env.ledger().with_mut(|li| {
            li.sequence_number = add_ready_at;
        });
        client.cv_apply_add_attestor(&attestor);
        assert!(verifier.is_attestor_approved(&attestor));

        let threshold_ready_at = client.cv_sched_set_threshold(&1_u32);
        env.ledger().with_mut(|li| {
            li.sequence_number = threshold_ready_at;
        });
        client.cv_apply_set_threshold();
        assert_eq!(verifier.get_threshold(), 1);

        let next_owner = Address::generate(&env);
        let transfer_out_ready_at = client.cv_sched_transfer_owner(&next_owner);
        let pending_owner = verifier.get_pending_owner().unwrap();
        assert_eq!(pending_owner.next_owner, next_owner);
        assert_eq!(pending_owner.activate_at_ledger, transfer_out_ready_at);
    }

    #[test]
    fn governance_auth_accepts_primary_signer_context() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(92);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());

        client.add_signer(&ed25519_signer_record(&env, public.clone(), 1, 18));

        let payload = BytesN::from_array(&env, &[19; 32]);
        let signature = key.sign(&payload.to_array());
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: public,
                signature: BytesN::from_array(&env, &signature.to_bytes()),
            },
        ];
        let auth_context = vec![
            &env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(&env, "cv_sched_set_threshold"),
                args: Vec::from_array(&env, [1_u32.into_val(&env)]),
            }),
        ];

        let result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &auth_context,
        );
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn governance_auth_rejects_session_signer_context() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(93);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());

        client.create_session_key(
            &session_signer_record(&env, public.clone(), 20),
            &SessionScope {
                allowed_action_bitmap: SESSION_ACTION_PAYMENT,
                allowed_assets: vec![&env, Address::generate(&env)],
                allowed_destinations: vec![&env, Address::generate(&env)],
                allowed_adapters: vec![&env],
                per_execution_cap: 10,
                cumulative_cap: 10,
                consumed_amount: 0,
                expiry_ledger: env.ledger().sequence() + 10,
                single_use: false,
            },
        );

        let payload = BytesN::from_array(&env, &[21; 32]);
        let signature = key.sign(&payload.to_array());
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: public,
                signature: BytesN::from_array(&env, &signature.to_bytes()),
            },
        ];
        let auth_context = vec![
            &env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(&env, "cv_sched_set_threshold"),
                args: Vec::from_array(&env, [1_u32.into_val(&env)]),
            }),
        ];

        let result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &auth_context,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::UnexpectedContext)));
    }

    #[test]
    fn management_auth_accepts_primary_signer_context() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(94);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());

        client.add_signer(&ed25519_signer_record(&env, public.clone(), 1, 22));

        let payload = BytesN::from_array(&env, &[23; 32]);
        let signature = key.sign(&payload.to_array());
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: public,
                signature: BytesN::from_array(&env, &signature.to_bytes()),
            },
        ];
        let auth_context = management_auth_context(&env, &client);

        let result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &auth_context,
        );
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn management_auth_rejects_session_signer_context() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(95);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());

        client.create_session_key(
            &session_signer_record(&env, public.clone(), 24),
            &SessionScope {
                allowed_action_bitmap: SESSION_ACTION_PAYMENT,
                allowed_assets: vec![&env, Address::generate(&env)],
                allowed_destinations: vec![&env, Address::generate(&env)],
                allowed_adapters: vec![&env],
                per_execution_cap: 10,
                cumulative_cap: 10,
                consumed_amount: 0,
                expiry_ledger: env.ledger().sequence() + 10,
                single_use: false,
            },
        );

        let payload = BytesN::from_array(&env, &[25; 32]);
        let signature = key.sign(&payload.to_array());
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: public,
                signature: BytesN::from_array(&env, &signature.to_bytes()),
            },
        ];
        let auth_context = management_auth_context(&env, &client);

        let result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &auth_context,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::UnexpectedContext)));
    }

    fn configure_payment_policy(
        env: &Env,
        client: &SmartAccountContractClient<'_>,
        adapter_address: &Address,
        asset: &Address,
        destination: &Address,
        max_single_transfer: i128,
    ) {
        client.set_adapter_config(
            &payment_adapter_id(env),
            &AdapterConfig {
                adapter_address: adapter_address.clone(),
                enabled: true,
                adapter_type: ADAPTER_TYPE_PAYMENT,
                max_single_execution_amount: max_single_transfer,
                allowed_assets: vec![env, asset.clone()],
                max_slippage_bps: 0,
                allowed_yield_operations: 0,
                max_split_recipients: 0,
                max_exposure_bps: 10_000,
            },
        );
        client.set_asset_config(
            asset,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer,
            },
        );
        client.set_destination_allowed(destination, &true);
    }

    fn configure_adapter_policy(
        env: &Env,
        client: &SmartAccountContractClient<'_>,
        adapter_id: &BytesN<32>,
        adapter_address: &Address,
        adapter_type: u32,
        asset: &Address,
        max_single_transfer: i128,
        enabled: bool,
    ) {
        client.set_asset_config(
            asset,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer,
            },
        );
        client.set_adapter_config(
            adapter_id,
            &AdapterConfig {
                adapter_address: adapter_address.clone(),
                enabled,
                adapter_type,
                max_single_execution_amount: max_single_transfer,
                allowed_assets: vec![env, asset.clone()],
                max_slippage_bps: 500,
                allowed_yield_operations: YIELD_OP_DEPOSIT | YIELD_OP_WITHDRAW,
                max_split_recipients: 8,
                max_exposure_bps: 5_000,
            },
        );
        let _ = env;
    }

    fn initialize_swap_adapter(env: &Env, adapter_address: &Address, route_hash: &BytesN<32>) {
        let owner = Address::generate(env);
        let client = SwapAdapterContractClient::new(env, adapter_address);
        client.initialize(&owner);
        client.approve_route(&route_hash);
    }

    fn initialize_yield_adapter(env: &Env, adapter_address: &Address, vault: &Address) {
        let owner = Address::generate(env);
        let client = YieldAdapterContractClient::new(env, adapter_address);
        client.initialize(&owner);
        client.approve_vault(vault);
    }

    fn initialize_split_adapter(env: &Env, adapter_address: &Address, max_recipients: u32) {
        let client = SplitAdapterContractClient::new(env, adapter_address);
        client.initialize(&max_recipients);
    }

    #[test]
    fn bootstrap_admin_can_store_signer_and_session() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(7);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());

        let signer = session_signer_record(&env, public.clone(), 1);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let scope = SessionScope {
            allowed_action_bitmap: SESSION_ACTION_PAYMENT,
            allowed_assets: vec![&env, asset],
            allowed_destinations: vec![&env, destination],
            allowed_adapters: vec![&env],
            per_execution_cap: 10,
            cumulative_cap: 100,
            consumed_amount: 0,
            expiry_ledger: env.ledger().sequence() + 10,
            single_use: false,
        };

        client.create_session_key(&signer, &scope);

        let stored = client.get_session_scope(&public);
        assert_eq!(stored, scope);
    }

    #[test]
    fn execute_automation_consumes_capability_once_per_child_id() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        env.ledger().with_mut(|li| {
            li.sequence_number = 25;
        });
        let transfer_adapter = env.register(TransferAdapterContract, ());
        let asset = issue_test_asset(&env).address();
        let destination = Address::generate(&env);
        configure_payment_policy(&env, &client, &transfer_adapter, &asset, &destination, 20);
        token::StellarAssetClient::new(&env, &asset).mint(&client.address, &20);

        let capability_id = CapabilityId(BytesN::from_array(&env, &[9; 32]));
        let capability = AutomationCapability {
            capability_id: capability_id.clone(),
            parent_intent_id: spf_shared_types::ParentIntentId(BytesN::from_array(&env, &[8; 32])),
            action: InteractiveAction::Payment(PaymentAction {
                asset: asset.clone(),
                destination: destination.clone(),
                amount: 10,
            }),
            required_attestation_id: None,
            policy_version: 1,
            executable_from_ledger: 26,
            executable_until_ledger: 30,
            max_executions: 2,
        };
        client.grant_automation_capability(&capability);
        env.ledger().with_mut(|li| {
            li.sequence_number = 26;
        });

        let child_execution_id = ChildExecutionId(BytesN::from_array(&env, &[3; 32]));
        client.execute_automation(&capability_id, &child_execution_id, &None);
        assert_eq!(token::Client::new(&env, &asset).balance(&destination), 10);

        let err = client.try_execute_automation(&capability_id, &child_execution_id, &None);
        assert_eq!(err, Err(Ok(SmartAccountError::ExecutionReplay)));
    }

    #[test]
    fn check_auth_accepts_active_ed25519_signer() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(11);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());
        let signer = ed25519_signer_record(&env, public.clone(), 1, 2);
        client.add_signer(&signer);

        let payload = BytesN::from_array(&env, &[5; 32]);
        let signature = key.sign(&payload.to_array());
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: public.clone(),
                signature: BytesN::from_array(&env, &signature.to_bytes()),
            },
        ];
        let auth_context = management_auth_context(&env, &client);

        let result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &auth_context,
        );
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn management_threshold_requires_weighted_quorum() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key_1 = signing_key(96);
        let key_2 = signing_key(97);
        let public_1 = BytesN::from_array(&env, &key_1.verifying_key().to_bytes());
        let public_2 = BytesN::from_array(&env, &key_2.verifying_key().to_bytes());

        client.add_signer(&ed25519_signer_record(&env, public_1.clone(), 1, 26));
        client.add_signer(&ed25519_signer_record(&env, public_2.clone(), 1, 27));
        client.set_management_threshold(&2_u32);

        let payload = BytesN::from_array(&env, &[28; 32]);
        let auth_context = management_auth_context(&env, &client);

        let single_signature = vec![
            &env,
            AccountSignature {
                signer_id: public_1.clone(),
                signature: BytesN::from_array(&env, &key_1.sign(&payload.to_array()).to_bytes()),
            },
        ];
        let single_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            single_signature.into_val(&env),
            &auth_context,
        );
        assert_eq!(single_result, Err(Ok(SmartAccountError::InsufficientManagementWeight)));

        let sig_1 = AccountSignature {
            signer_id: public_1,
            signature: BytesN::from_array(&env, &key_1.sign(&payload.to_array()).to_bytes()),
        };
        let sig_2 = AccountSignature {
            signer_id: public_2,
            signature: BytesN::from_array(&env, &key_2.sign(&payload.to_array()).to_bytes()),
        };
        let quorum_signatures = if sig_1.signer_id < sig_2.signer_id {
            vec![&env, sig_1, sig_2]
        } else {
            vec![&env, sig_2, sig_1]
        };
        let quorum_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            quorum_signatures.into_val(&env),
            &auth_context,
        );
        assert_eq!(quorum_result, Ok(()));
    }

    #[test]
    fn governance_and_management_thresholds_can_diverge() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key_1 = signing_key(98);
        let key_2 = signing_key(99);
        let public_1 = BytesN::from_array(&env, &key_1.verifying_key().to_bytes());
        let public_2 = BytesN::from_array(&env, &key_2.verifying_key().to_bytes());

        client.add_signer(&ed25519_signer_record(&env, public_1.clone(), 1, 29));
        client.add_signer(&ed25519_signer_record(&env, public_2.clone(), 1, 30));
        client.set_management_threshold(&1_u32);
        client.set_governance_threshold(&2_u32);

        let payload = BytesN::from_array(&env, &[31; 32]);
        let management_context = management_auth_context(&env, &client);
        let governance_context = governance_auth_context(&env, &client);

        let single_signature = vec![
            &env,
            AccountSignature {
                signer_id: public_1.clone(),
                signature: BytesN::from_array(&env, &key_1.sign(&payload.to_array()).to_bytes()),
            },
        ];
        let management_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            single_signature.clone().into_val(&env),
            &management_context,
        );
        assert_eq!(management_result, Ok(()));

        let governance_single_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            single_signature.into_val(&env),
            &governance_context,
        );
        assert_eq!(
            governance_single_result,
            Err(Ok(SmartAccountError::InsufficientGovernanceWeight))
        );

        let sig_1 = AccountSignature {
            signer_id: public_1,
            signature: BytesN::from_array(&env, &key_1.sign(&payload.to_array()).to_bytes()),
        };
        let sig_2 = AccountSignature {
            signer_id: public_2,
            signature: BytesN::from_array(&env, &key_2.sign(&payload.to_array()).to_bytes()),
        };
        let quorum_signatures = if sig_1.signer_id < sig_2.signer_id {
            vec![&env, sig_1, sig_2]
        } else {
            vec![&env, sig_2, sig_1]
        };
        let governance_quorum_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            quorum_signatures.into_val(&env),
            &governance_context,
        );
        assert_eq!(governance_quorum_result, Ok(()));
    }

    #[test]
    fn recovery_threshold_can_diverge_from_other_planes() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key_1 = signing_key(100);
        let key_2 = signing_key(101);
        let public_1 = BytesN::from_array(&env, &key_1.verifying_key().to_bytes());
        let public_2 = BytesN::from_array(&env, &key_2.verifying_key().to_bytes());

        client.add_signer(&ed25519_signer_record(&env, public_1.clone(), 1, 32));
        client.add_signer(&ed25519_signer_record(&env, public_2.clone(), 1, 33));
        client.set_management_threshold(&1_u32);
        client.set_governance_threshold(&1_u32);
        client.set_recovery_threshold(&2_u32);

        let payload = BytesN::from_array(&env, &[34; 32]);
        let recovery_context = recovery_auth_context(&env, &client);
        let single_signature = vec![
            &env,
            AccountSignature {
                signer_id: public_1.clone(),
                signature: BytesN::from_array(&env, &key_1.sign(&payload.to_array()).to_bytes()),
            },
        ];
        let single_recovery_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            single_signature.into_val(&env),
            &recovery_context,
        );
        assert_eq!(
            single_recovery_result,
            Err(Ok(SmartAccountError::InsufficientRecoveryWeight))
        );

        let sig_1 = AccountSignature {
            signer_id: public_1,
            signature: BytesN::from_array(&env, &key_1.sign(&payload.to_array()).to_bytes()),
        };
        let sig_2 = AccountSignature {
            signer_id: public_2,
            signature: BytesN::from_array(&env, &key_2.sign(&payload.to_array()).to_bytes()),
        };
        let quorum_signatures = if sig_1.signer_id < sig_2.signer_id {
            vec![&env, sig_1, sig_2]
        } else {
            vec![&env, sig_2, sig_1]
        };
        let quorum_recovery_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            quorum_signatures.into_val(&env),
            &recovery_context,
        );
        assert_eq!(quorum_recovery_result, Ok(()));
    }

    #[test]
    fn guardian_signers_take_over_recovery_plane_when_configured() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let primary_key = signing_key(105);
        let guardian_key = signing_key(106);
        let primary_signer = BytesN::from_array(&env, &primary_key.verifying_key().to_bytes());
        let guardian_signer = BytesN::from_array(&env, &guardian_key.verifying_key().to_bytes());

        client.add_signer(&ed25519_signer_record(&env, primary_signer.clone(), 1, 42));
        client.add_signer(&guardian_signer_record(&env, guardian_signer.clone(), 1, 43));
        client.set_recovery_threshold(&1_u32);

        let payload = BytesN::from_array(&env, &[44; 32]);
        let recovery_context = recovery_auth_context(&env, &client);
        let primary_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            vec![
                &env,
                AccountSignature {
                    signer_id: primary_signer,
                    signature: BytesN::from_array(
                        &env,
                        &primary_key.sign(&payload.to_array()).to_bytes(),
                    ),
                },
            ]
            .into_val(&env),
            &recovery_context,
        );
        assert_eq!(primary_result, Err(Ok(SmartAccountError::MissingRequiredRole)));

        let guardian_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            vec![
                &env,
                AccountSignature {
                    signer_id: guardian_signer,
                    signature: BytesN::from_array(
                        &env,
                        &guardian_key.sign(&payload.to_array()).to_bytes(),
                    ),
                },
            ]
            .into_val(&env),
            &recovery_context,
        );
        assert_eq!(guardian_result, Ok(()));
    }

    #[test]
    fn governance_threshold_cannot_exceed_governance_capable_weight() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let governance_key = signing_key(107);
        let management_key = signing_key(108);
        let governance_signer =
            BytesN::from_array(&env, &governance_key.verifying_key().to_bytes());
        let management_signer =
            BytesN::from_array(&env, &management_key.verifying_key().to_bytes());

        client.add_signer(&SignerRecord {
            signer_id: governance_signer,
            signer_kind: SignerKind::Ed25519,
            role_bitmap: SIGNER_ROLE_GOVERNANCE,
            status: SignerStatus::Active,
            weight: 1,
            created_ledger: env.ledger().sequence(),
            expires_ledger: Some(env.ledger().sequence() + 50),
            metadata_hash: BytesN::from_array(&env, &[45; 32]),
        });
        client.add_signer(&SignerRecord {
            signer_id: management_signer,
            signer_kind: SignerKind::Ed25519,
            role_bitmap: SIGNER_ROLE_MANAGEMENT,
            status: SignerStatus::Active,
            weight: 5,
            created_ledger: env.ledger().sequence(),
            expires_ledger: Some(env.ledger().sequence() + 50),
            metadata_hash: BytesN::from_array(&env, &[46; 32]),
        });

        let result = client.try_set_governance_threshold(&2_u32);
        assert_eq!(result, Err(Ok(SmartAccountError::InsufficientGovernanceWeight)));
    }

    #[test]
    fn policy_engine_replacement_requires_governance_role() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(114);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());

        client.add_signer(&SignerRecord {
            signer_id: public,
            signer_kind: SignerKind::Ed25519,
            role_bitmap: SIGNER_ROLE_MANAGEMENT,
            status: SignerStatus::Active,
            weight: 1,
            created_ledger: env.ledger().sequence(),
            expires_ledger: Some(env.ledger().sequence() + 50),
            metadata_hash: BytesN::from_array(&env, &[58; 32]),
        });

        let payload = BytesN::from_array(&env, &[59; 32]);
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: BytesN::from_array(&env, &key.verifying_key().to_bytes()),
                signature: BytesN::from_array(&env, &key.sign(&payload.to_array()).to_bytes()),
            },
        ];
        let result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &governance_policy_engine_auth_context(&env, &client),
        );
        assert_eq!(result, Err(Ok(SmartAccountError::MissingRequiredRole)));
    }

    #[test]
    fn signer_roles_enforce_plane_separation() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(102);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());

        client.add_signer(&SignerRecord {
            signer_id: public.clone(),
            signer_kind: SignerKind::Ed25519,
            role_bitmap: SIGNER_ROLE_MANAGEMENT,
            status: SignerStatus::Active,
            weight: 1,
            created_ledger: env.ledger().sequence(),
            expires_ledger: Some(env.ledger().sequence() + 50),
            metadata_hash: BytesN::from_array(&env, &[35; 32]),
        });

        let payload = BytesN::from_array(&env, &[36; 32]);
        let signature = key.sign(&payload.to_array());
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: public,
                signature: BytesN::from_array(&env, &signature.to_bytes()),
            },
        ];

        let management_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.clone().into_val(&env),
            &management_auth_context(&env, &client),
        );
        assert_eq!(management_result, Ok(()));

        let governance_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &governance_auth_context(&env, &client),
        );
        assert_eq!(governance_result, Err(Ok(SmartAccountError::MissingRequiredRole)));
    }

    #[test]
    fn spend_roles_enforce_operation_separation() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let payment_key = signing_key(103);
        let adapter_key = signing_key(104);
        let payment_signer = BytesN::from_array(&env, &payment_key.verifying_key().to_bytes());
        let adapter_signer = BytesN::from_array(&env, &adapter_key.verifying_key().to_bytes());

        client.add_signer(&SignerRecord {
            signer_id: payment_signer.clone(),
            signer_kind: SignerKind::Ed25519,
            role_bitmap: SIGNER_ROLE_PAYMENT,
            status: SignerStatus::Active,
            weight: 1,
            created_ledger: env.ledger().sequence(),
            expires_ledger: Some(env.ledger().sequence() + 50),
            metadata_hash: BytesN::from_array(&env, &[37; 32]),
        });
        client.add_signer(&SignerRecord {
            signer_id: adapter_signer.clone(),
            signer_kind: SignerKind::Ed25519,
            role_bitmap: SIGNER_ROLE_ADAPTER,
            status: SignerStatus::Active,
            weight: 1,
            created_ledger: env.ledger().sequence(),
            expires_ledger: Some(env.ledger().sequence() + 50),
            metadata_hash: BytesN::from_array(&env, &[38; 32]),
        });

        let payment_action = InteractiveAction::Payment(PaymentAction {
            asset: Address::generate(&env),
            destination: Address::generate(&env),
            amount: 1,
        });
        let adapter_action = InteractiveAction::Swap(SwapAction {
            adapter_id: BytesN::from_array(&env, &[39; 32]),
            asset_in: Address::generate(&env),
            asset_out: Address::generate(&env),
            amount_in: 1,
            quoted_amount_out: 1,
            min_amount_out: 1,
            route_hash: BytesN::from_array(&env, &[40; 32]),
        });

        let payment_payload = BytesN::from_array(&env, &[40; 32]);
        let payment_sig = vec![
            &env,
            AccountSignature {
                signer_id: payment_signer.clone(),
                signature: BytesN::from_array(&env, &payment_key.sign(&payment_payload.to_array()).to_bytes()),
            },
        ];
        let payment_context = vec![
            &env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(&env, "execute_interactive"),
                args: Vec::from_array(
                    &env,
                    [
                        payment_action.clone().into_val(&env),
                        1_u32.into_val(&env),
                        payment_signer.clone().into_val(&env),
                    ],
                ),
            }),
        ];
        let payment_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payment_payload,
            payment_sig.clone().into_val(&env),
            &payment_context,
        );
        assert_eq!(payment_result, Ok(()));

        let payment_on_adapter_context = vec![
            &env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(&env, "execute_interactive"),
                args: Vec::from_array(
                    &env,
                    [
                        adapter_action.clone().into_val(&env),
                        1_u32.into_val(&env),
                        payment_signer.into_val(&env),
                    ],
                ),
            }),
        ];
        let payment_on_adapter_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payment_payload,
            payment_sig.into_val(&env),
            &payment_on_adapter_context,
        );
        assert_eq!(payment_on_adapter_result, Err(Ok(SmartAccountError::MissingRequiredRole)));

        let adapter_payload = BytesN::from_array(&env, &[41; 32]);
        let adapter_sig = vec![
            &env,
            AccountSignature {
                signer_id: adapter_signer.clone(),
                signature: BytesN::from_array(&env, &adapter_key.sign(&adapter_payload.to_array()).to_bytes()),
            },
        ];
        let adapter_context = vec![
            &env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(&env, "execute_interactive"),
                args: Vec::from_array(
                    &env,
                    [
                        adapter_action.into_val(&env),
                        1_u32.into_val(&env),
                        adapter_signer.clone().into_val(&env),
                    ],
                ),
            }),
        ];
        let adapter_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &adapter_payload,
            adapter_sig.clone().into_val(&env),
            &adapter_context,
        );
        assert_eq!(adapter_result, Ok(()));

        let adapter_on_payment_context = vec![
            &env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(&env, "execute_interactive"),
                args: Vec::from_array(
                    &env,
                    [
                        payment_action.into_val(&env),
                        1_u32.into_val(&env),
                        adapter_signer.into_val(&env),
                    ],
                ),
            }),
        ];
        let adapter_on_payment_result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &adapter_payload,
            adapter_sig.into_val(&env),
            &adapter_on_payment_context,
        );
        assert_eq!(adapter_on_payment_result, Err(Ok(SmartAccountError::MissingRequiredRole)));
    }

    #[test]
    fn initiate_recovery_freezes_account_and_blocks_execution() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(109);
        let signer = BytesN::from_array(&env, &key.verifying_key().to_bytes());
        client.add_signer(&ed25519_signer_record(&env, signer, 1, 47));

        let rotate_to = BytesN::from_array(&env, &[48; 32]);
        client.initiate_recovery(
            &vec![&env, ed25519_signer_record(&env, rotate_to, 1, 49)],
            &Vec::new(&env),
            &1_u32,
            &1_u32,
            &1_u32,
            &(env.ledger().sequence() + 5),
        );

        assert!(client.status().frozen);
        let result = client.try_execute_interactive(
            &InteractiveAction::Payment(PaymentAction {
                asset: Address::generate(&env),
                destination: Address::generate(&env),
                amount: 1,
            }),
            &1_u32,
            &BytesN::from_array(&env, &[50; 32]),
        );
        assert_eq!(result, Err(Ok(SmartAccountError::Frozen)));
    }

    #[test]
    fn finalize_recovery_requires_delay_and_rotates_signers() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let old_key = signing_key(110);
        let old_signer = BytesN::from_array(&env, &old_key.verifying_key().to_bytes());
        client.add_signer(&ed25519_signer_record(&env, old_signer.clone(), 1, 51));

        let new_key = signing_key(111);
        let new_signer = BytesN::from_array(&env, &new_key.verifying_key().to_bytes());
        let guardian_key = signing_key(112);
        let guardian_signer = BytesN::from_array(&env, &guardian_key.verifying_key().to_bytes());
        let activate_at = env.ledger().sequence() + 7;

        client.initiate_recovery(
            &vec![
                &env,
                SignerRecord {
                    role_bitmap: SIGNER_ROLE_MANAGEMENT | SIGNER_ROLE_GOVERNANCE,
                    ..ed25519_signer_record(&env, new_signer.clone(), 1, 52)
                },
            ],
            &vec![&env, guardian_signer_record(&env, guardian_signer.clone(), 1, 53)],
            &1_u32,
            &1_u32,
            &1_u32,
            &activate_at,
        );

        let early = client.try_finalize_recovery();
        assert_eq!(early, Err(Ok(SmartAccountError::RecoveryDelayNotElapsed)));

        env.ledger().with_mut(|li| {
            li.sequence_number = activate_at;
        });
        client.finalize_recovery();

        assert!(!client.status().frozen);
        assert_eq!(client.status().policy_version, 2);
        assert_eq!(
            client.try_get_signer(&old_signer),
            Err(Ok(SmartAccountError::SignerNotFound))
        );
        assert_eq!(client.get_signer(&new_signer).signer_id, new_signer);
        assert_eq!(client.get_signer(&guardian_signer).signer_kind, SignerKind::Guardian);
        assert_eq!(
            client.try_get_pending_recovery(),
            Err(Ok(SmartAccountError::RecoveryNotPending))
        );
    }

    #[test]
    fn recovery_can_be_cancelled_before_finalize() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(113);
        let signer = BytesN::from_array(&env, &key.verifying_key().to_bytes());
        client.add_signer(&ed25519_signer_record(&env, signer, 1, 54));

        client.initiate_recovery(
            &vec![
                &env,
                SignerRecord {
                    role_bitmap: spf_shared_types::SIGNER_ROLE_FULL_PRIMARY,
                    ..ed25519_signer_record(&env, BytesN::from_array(&env, &[55; 32]), 1, 55)
                },
            ],
            &Vec::new(&env),
            &1_u32,
            &1_u32,
            &1_u32,
            &(env.ledger().sequence() + 3),
        );
        assert!(client.get_pending_recovery().activate_at_ledger > env.ledger().sequence());

        client.cancel_recovery();
        assert_eq!(
            client.try_get_pending_recovery(),
            Err(Ok(SmartAccountError::RecoveryNotPending))
        );
        assert!(client.status().frozen);
    }

    #[test]
    fn recovery_mode_actions_require_frozen_or_pending_state() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let replacement_policy_engine = env.register(PolicyEngineContract, ());
        PolicyEngineContractClient::new(&env, &replacement_policy_engine)
            .initialize(&Address::generate(&env), &1_u32);

        let result = client.try_recovery_set_policy_engine(&replacement_policy_engine);
        assert_eq!(result, Err(Ok(SmartAccountError::InvalidState)));
    }

    #[test]
    fn recovery_mode_can_replace_policy_engine_and_lock_execution_surfaces() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let asset_admin = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(asset_admin);
        let asset = sac.address();
        let destination = Address::generate(&env);
        let adapter_address = env.register(TransferAdapterContract, ());
        let adapter_id = payment_adapter_id(&env);

        client.set_asset_config(
            &asset,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 100,
            },
        );
        client.set_adapter_config(
            &adapter_id,
            &AdapterConfig {
                adapter_address: adapter_address.clone(),
                enabled: true,
                adapter_type: ADAPTER_TYPE_PAYMENT,
                max_single_execution_amount: 100,
                allowed_assets: vec![&env, asset.clone()],
                max_slippage_bps: 0,
                allowed_yield_operations: 0,
                max_split_recipients: 0,
                max_exposure_bps: 10_000,
            },
        );
        client.set_destination_allowed(&destination, &true);

        let replacement_policy_engine = env.register(PolicyEngineContract, ());
        let replacement_policy_client =
            PolicyEngineContractClient::new(&env, &replacement_policy_engine);
        replacement_policy_client.initialize(&Address::generate(&env), &1_u32);

        let capability_id = CapabilityId(BytesN::from_array(&env, &[56; 32]));
        client.grant_automation_capability(&AutomationCapability {
            capability_id: capability_id.clone(),
            parent_intent_id: spf_shared_types::ParentIntentId(BytesN::from_array(&env, &[57; 32])),
            action: InteractiveAction::Payment(PaymentAction {
                asset: asset.clone(),
                destination: destination.clone(),
                amount: 5,
            }),
            required_attestation_id: None,
            policy_version: 1,
            executable_from_ledger: env.ledger().sequence() + 1,
            executable_until_ledger: env.ledger().sequence() + 10,
            max_executions: 1,
        });

        client.freeze();
        let policy_before = client.status().policy_version;
        client.recovery_set_policy_engine(&replacement_policy_engine);
        client.recovery_disable_adapter(&adapter_id);
        client.recovery_disable_asset(&asset);
        client.recovery_block_destination(&destination);
        client.recovery_revoke_capability(&capability_id);

        assert_eq!(client.get_policy_engine(), replacement_policy_engine);
        assert!(!client.get_adapter_config(&adapter_id).enabled);
        assert!(!client.get_asset_config(&asset).enabled);
        assert!(!client.is_destination_allowed(&destination));
        assert!(client.get_automation_capability(&capability_id).revoked);
        assert!(client.status().policy_version >= policy_before + 4);
    }

    #[test]
    fn payment_execution_rejects_non_payment_adapter_type() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let transfer_adapter = env.register(TransferAdapterContract, ());
        let asset = issue_test_asset(&env).address();
        let destination = Address::generate(&env);
        let signer_key = signing_key(115);
        let signer = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());

        client.add_signer(&SignerRecord {
            role_bitmap: SIGNER_ROLE_PAYMENT,
            ..ed25519_signer_record(&env, signer.clone(), 1, 60)
        });
        client.set_asset_config(
            &asset,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 100,
            },
        );
        client.set_destination_allowed(&destination, &true);
        client.set_adapter_config(
            &payment_adapter_id(&env),
            &AdapterConfig {
                adapter_address: transfer_adapter,
                enabled: true,
                adapter_type: ADAPTER_TYPE_SWAP,
                max_single_execution_amount: 100,
                allowed_assets: vec![&env, asset.clone()],
                max_slippage_bps: 500,
                allowed_yield_operations: 0,
                max_split_recipients: 0,
                max_exposure_bps: 10_000,
            },
        );

        let result = client.try_execute_interactive(
            &InteractiveAction::Payment(PaymentAction {
                asset,
                destination,
                amount: 10,
            }),
            &1_u32,
            &signer,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::AdapterConstraintViolation)));
    }

    #[test]
    fn adapter_execution_rejects_asset_outside_adapter_allowlist() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let transfer_adapter = env.register(SwapAdapterContract, ());
        let configured_asset = issue_test_asset(&env).address();
        let action_asset = issue_test_asset(&env).address();
        let asset_out = issue_test_asset(&env).address();
        let adapter_id = BytesN::from_array(&env, &[61; 32]);
        let signer_key = signing_key(116);
        let signer = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());

        client.add_signer(&SignerRecord {
            role_bitmap: SIGNER_ROLE_ADAPTER,
            ..ed25519_signer_record(&env, signer.clone(), 1, 62)
        });
        client.set_asset_config(
            &action_asset,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 100,
            },
        );
        client.set_asset_config(
            &asset_out,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 100,
            },
        );
        client.set_adapter_config(
            &adapter_id,
            &AdapterConfig {
                adapter_address: transfer_adapter,
                enabled: true,
                adapter_type: ADAPTER_TYPE_SWAP,
                max_single_execution_amount: 100,
                allowed_assets: vec![&env, configured_asset],
                max_slippage_bps: 500,
                allowed_yield_operations: 0,
                max_split_recipients: 0,
                max_exposure_bps: 10_000,
            },
        );

        let result = client.try_execute_interactive(
            &InteractiveAction::Swap(SwapAction {
                adapter_id,
                asset_in: action_asset,
                asset_out,
                amount_in: 10,
                quoted_amount_out: 10,
                min_amount_out: 1,
                route_hash: BytesN::from_array(&env, &[62; 32]),
            }),
            &1_u32,
            &signer,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::AdapterConstraintViolation)));
    }

    #[test]
    fn adapter_execution_rejects_amount_above_adapter_specific_cap() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let transfer_adapter = env.register(YieldAdapterContract, ());
        let asset = issue_test_asset(&env).address();
        let adapter_id = BytesN::from_array(&env, &[63; 32]);
        let signer_key = signing_key(117);
        let signer = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());

        client.add_signer(&SignerRecord {
            role_bitmap: SIGNER_ROLE_ADAPTER,
            ..ed25519_signer_record(&env, signer.clone(), 1, 64)
        });
        client.set_asset_config(
            &asset,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 100,
            },
        );
        client.set_adapter_config(
            &adapter_id,
            &AdapterConfig {
                adapter_address: transfer_adapter,
                enabled: true,
                adapter_type: ADAPTER_TYPE_YIELD,
                max_single_execution_amount: 5,
                allowed_assets: vec![&env, asset.clone()],
                max_slippage_bps: 0,
                allowed_yield_operations: YIELD_OP_DEPOSIT | YIELD_OP_WITHDRAW,
                max_split_recipients: 0,
                max_exposure_bps: 10_000,
            },
        );

        let result = client.try_execute_interactive(
            &InteractiveAction::Yield(YieldAction {
                adapter_id,
                vault: Address::generate(&env),
                asset,
                amount: 10,
                operation: YieldOperation::Deposit,
            }),
            &1_u32,
            &signer,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::AdapterConstraintViolation)));
    }

    #[test]
    fn swap_execution_rejects_slippage_above_adapter_limit() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let adapter_contract = env.register(SwapAdapterContract, ());
        let adapter_id = BytesN::from_array(&env, &[65; 32]);
        let asset_in = issue_test_asset(&env).address();
        let asset_out = issue_test_asset(&env).address();
        let signer_key = signing_key(118);
        let signer = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());

        client.add_signer(&SignerRecord {
            role_bitmap: SIGNER_ROLE_ADAPTER,
            ..ed25519_signer_record(&env, signer.clone(), 1, 67)
        });
        client.set_asset_config(
            &asset_in,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 100,
            },
        );
        client.set_asset_config(
            &asset_out,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 100,
            },
        );
        client.set_adapter_config(
            &adapter_id,
            &AdapterConfig {
                adapter_address: adapter_contract,
                enabled: true,
                adapter_type: ADAPTER_TYPE_SWAP,
                max_single_execution_amount: 100,
                allowed_assets: vec![&env, asset_in.clone()],
                max_slippage_bps: 100,
                allowed_yield_operations: 0,
                max_split_recipients: 0,
                max_exposure_bps: 10_000,
            },
        );

        let result = client.try_execute_interactive(
            &InteractiveAction::Swap(SwapAction {
                adapter_id,
                asset_in,
                asset_out,
                amount_in: 50,
                quoted_amount_out: 50,
                min_amount_out: 40,
                route_hash: BytesN::from_array(&env, &[72; 32]),
            }),
            &1_u32,
            &signer,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::AdapterConstraintViolation)));
    }

    #[test]
    fn yield_execution_rejects_operation_not_allowed_by_adapter() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let adapter_contract = env.register(YieldAdapterContract, ());
        let adapter_id = BytesN::from_array(&env, &[66; 32]);
        let asset = issue_test_asset(&env).address();
        let signer_key = signing_key(119);
        let signer = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());

        client.add_signer(&SignerRecord {
            role_bitmap: SIGNER_ROLE_ADAPTER,
            ..ed25519_signer_record(&env, signer.clone(), 1, 68)
        });
        client.set_asset_config(
            &asset,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 100,
            },
        );
        client.set_adapter_config(
            &adapter_id,
            &AdapterConfig {
                adapter_address: adapter_contract,
                enabled: true,
                adapter_type: ADAPTER_TYPE_YIELD,
                max_single_execution_amount: 100,
                allowed_assets: vec![&env, asset.clone()],
                max_slippage_bps: 0,
                allowed_yield_operations: YIELD_OP_DEPOSIT,
                max_split_recipients: 0,
                max_exposure_bps: 10_000,
            },
        );

        let result = client.try_execute_interactive(
            &InteractiveAction::Yield(YieldAction {
                adapter_id,
                vault: Address::generate(&env),
                asset,
                amount: 10,
                operation: YieldOperation::Withdraw,
            }),
            &1_u32,
            &signer,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::AdapterConstraintViolation)));
    }

    #[test]
    fn split_execution_rejects_too_many_or_duplicate_recipients() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let adapter_contract = env.register(SplitAdapterContract, ());
        let adapter_id = BytesN::from_array(&env, &[67; 32]);
        let asset = issue_test_asset(&env).address();
        let signer_key = signing_key(120);
        let signer = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());
        let recipient = Address::generate(&env);

        client.add_signer(&SignerRecord {
            role_bitmap: SIGNER_ROLE_ADAPTER,
            ..ed25519_signer_record(&env, signer.clone(), 1, 69)
        });
        client.set_asset_config(
            &asset,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 100,
            },
        );
        client.set_destination_allowed(&recipient, &true);
        client.set_adapter_config(
            &adapter_id,
            &AdapterConfig {
                adapter_address: adapter_contract,
                enabled: true,
                adapter_type: ADAPTER_TYPE_SPLIT,
                max_single_execution_amount: 100,
                allowed_assets: vec![&env, asset.clone()],
                max_slippage_bps: 0,
                allowed_yield_operations: 0,
                max_split_recipients: 1,
                max_exposure_bps: 10_000,
            },
        );

        let duplicate_recipients = vec![
            &env,
            SplitRecipient {
                destination: recipient.clone(),
                amount: 10,
            },
            SplitRecipient {
                destination: recipient,
                amount: 10,
            },
        ];
        let result = client.try_execute_interactive(
            &InteractiveAction::Split(SplitAction {
                adapter_id,
                asset,
                recipients: duplicate_recipients,
            }),
            &1_u32,
            &signer,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::AdapterConstraintViolation)));
    }

    #[test]
    fn pause_and_unpause_toggle_status() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        assert!(!client.status().paused);
        client.pause();
        assert!(client.status().paused);
        client.unpause();
        assert!(!client.status().paused);
    }

    #[test]
    fn duplicate_capability_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        env.ledger().with_mut(|li| {
            li.sequence_number = 40;
        });

        let capability_id = CapabilityId(BytesN::from_array(&env, &[4; 32]));
        let capability = AutomationCapability {
            capability_id: capability_id.clone(),
            parent_intent_id: spf_shared_types::ParentIntentId(BytesN::from_array(&env, &[5; 32])),
            action: InteractiveAction::Payment(PaymentAction {
                asset: Address::generate(&env),
                destination: Address::generate(&env),
                amount: 1,
            }),
            required_attestation_id: None,
            policy_version: 1,
            executable_from_ledger: 41,
            executable_until_ledger: 50,
            max_executions: 1,
        };

        client.grant_automation_capability(&capability);
        let err = client.try_grant_automation_capability(&capability);
        assert_eq!(err, Err(Ok(SmartAccountError::DuplicateCapability)));
    }

    #[test]
    fn session_signer_rejects_non_interactive_context() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(21);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());

        let signer = session_signer_record(&env, public.clone(), 7);
        let scope = SessionScope {
            allowed_action_bitmap: SESSION_ACTION_PAYMENT,
            allowed_assets: vec![&env],
            allowed_destinations: vec![&env],
            allowed_adapters: vec![&env],
            per_execution_cap: 10,
            cumulative_cap: 100,
            consumed_amount: 0,
            expiry_ledger: env.ledger().sequence() + 10,
            single_use: false,
        };
        client.create_session_key(&signer, &scope);

        let payload = BytesN::from_array(&env, &[6; 32]);
        let signature = key.sign(&payload.to_array());
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: public.clone(),
                signature: BytesN::from_array(&env, &signature.to_bytes()),
            },
        ];
        let auth_context = vec![
            &env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(&env, "pause"),
                args: vec![&env],
            }),
        ];

        let result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &auth_context,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::UnexpectedContext)));
    }

    #[test]
    fn bootstrap_admin_can_rotate() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup_contract(&env);
        let next_admin = Address::generate(&env);

        assert_eq!(client.get_bootstrap_admin(), admin);
        client.set_bootstrap_admin(&next_admin);
        assert_eq!(client.get_bootstrap_admin(), next_admin);
    }

    #[test]
    fn session_signer_accepts_in_scope_payment_action() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(31);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);

        let signer = session_signer_record(&env, public.clone(), 8);
        let scope = SessionScope {
            allowed_action_bitmap: SESSION_ACTION_PAYMENT,
            allowed_assets: vec![&env, asset.clone()],
            allowed_destinations: vec![&env, destination.clone()],
            allowed_adapters: vec![&env],
            per_execution_cap: 25,
            cumulative_cap: 100,
            consumed_amount: 10,
            expiry_ledger: env.ledger().sequence() + 10,
            single_use: false,
        };
        client.create_session_key(&signer, &scope);

        let action = InteractiveAction::Payment(PaymentAction {
            asset,
            destination,
            amount: 15,
        });
        let payload = BytesN::from_array(&env, &[7; 32]);
        let signature = key.sign(&payload.to_array());
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: public.clone(),
                signature: BytesN::from_array(&env, &signature.to_bytes()),
            },
        ];
        let auth_context = vec![
            &env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(&env, "execute_interactive"),
                args: Vec::from_array(
                    &env,
                    [
                        action.into_val(&env),
                        1_u32.into_val(&env),
                        public.clone().into_val(&env),
                    ],
                ),
            }),
        ];

        let result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &auth_context,
        );
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn session_signer_rejects_out_of_scope_payment_asset() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(41);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());
        let allowed_asset = Address::generate(&env);
        let disallowed_asset = Address::generate(&env);
        let destination = Address::generate(&env);

        let signer = session_signer_record(&env, public.clone(), 9);
        let scope = SessionScope {
            allowed_action_bitmap: SESSION_ACTION_PAYMENT,
            allowed_assets: vec![&env, allowed_asset],
            allowed_destinations: vec![&env, destination.clone()],
            allowed_adapters: vec![&env],
            per_execution_cap: 25,
            cumulative_cap: 100,
            consumed_amount: 10,
            expiry_ledger: env.ledger().sequence() + 10,
            single_use: false,
        };
        client.create_session_key(&signer, &scope);

        let action = InteractiveAction::Payment(PaymentAction {
            asset: disallowed_asset,
            destination,
            amount: 15,
        });
        let payload = BytesN::from_array(&env, &[10; 32]);
        let signature = key.sign(&payload.to_array());
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: public.clone(),
                signature: BytesN::from_array(&env, &signature.to_bytes()),
            },
        ];
        let auth_context = vec![
            &env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(&env, "execute_interactive"),
                args: Vec::from_array(
                    &env,
                    [
                        action.into_val(&env),
                        1_u32.into_val(&env),
                        public.clone().into_val(&env),
                    ],
                ),
            }),
        ];

        let result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &auth_context,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::ActionOutOfScope)));
    }

    #[test]
    fn execute_interactive_updates_session_consumption_and_revokes_at_cap() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(51);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());
        let transfer_adapter = env.register(TransferAdapterContract, ());
        let asset = issue_test_asset(&env).address();
        let destination = Address::generate(&env);

        let signer = session_signer_record(&env, public.clone(), 11);
        let scope = SessionScope {
            allowed_action_bitmap: SESSION_ACTION_PAYMENT,
            allowed_assets: vec![&env, asset.clone()],
            allowed_destinations: vec![&env, destination.clone()],
            allowed_adapters: vec![&env],
            per_execution_cap: 10,
            cumulative_cap: 10,
            consumed_amount: 0,
            expiry_ledger: env.ledger().sequence() + 10,
            single_use: false,
        };
        client.create_session_key(&signer, &scope);

        let action = InteractiveAction::Payment(PaymentAction {
            asset: asset.clone(),
            destination: destination.clone(),
            amount: 10,
        });
        configure_payment_policy(
            &env,
            &client,
            &transfer_adapter,
            &action_asset(&action),
            &action_destination(&action),
            10,
        );
        token::StellarAssetClient::new(&env, &asset).mint(&client.address, &10);
        client.execute_interactive(&action, &1_u32, &public);

        let err = client.try_get_session_scope(&public);
        assert_eq!(err, Err(Ok(SmartAccountError::SessionScopeNotFound)));

        let signer_after = client.get_signer(&public);
        assert_eq!(signer_after.status, SignerStatus::Revoked);
        assert_eq!(token::Client::new(&env, &asset).balance(&destination), 10);
    }

    #[test]
    fn execute_interactive_rejects_destination_not_in_account_allowlist() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let signer_key = signing_key(61);
        let public = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());
        let transfer_adapter = env.register(TransferAdapterContract, ());
        let asset = issue_test_asset(&env).address();
        let destination = Address::generate(&env);

        client.add_signer(&ed25519_signer_record(&env, public.clone(), 1, 12));
        client.set_adapter_config(
            &payment_adapter_id(&env),
            &AdapterConfig {
                adapter_address: transfer_adapter,
                enabled: true,
                adapter_type: ADAPTER_TYPE_PAYMENT,
                max_single_execution_amount: 100,
                allowed_assets: vec![&env, asset.clone()],
                max_slippage_bps: 0,
                allowed_yield_operations: 0,
                max_split_recipients: 0,
                max_exposure_bps: 10_000,
            },
        );
        client.set_asset_config(
            &asset,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 100,
            },
        );

        let action = InteractiveAction::Payment(PaymentAction {
            asset,
            destination,
            amount: 10,
        });
        let err = client.try_execute_interactive(&action, &1_u32, &public);
        assert_eq!(err, Err(Ok(SmartAccountError::DestinationNotAllowed)));
    }

    #[test]
    fn execute_interactive_rejects_when_policy_engine_disallows_payments() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup_contract(&env);
        let signer_key = signing_key(62);
        let public = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());
        let transfer_adapter = env.register(TransferAdapterContract, ());
        let asset = issue_test_asset(&env).address();
        let destination = Address::generate(&env);
        let policy_engine = client.status().policy_version;
        let _ = policy_engine;

        client.add_signer(&ed25519_signer_record(&env, public.clone(), 1, 16));
        configure_payment_policy(&env, &client, &transfer_adapter, &asset, &destination, 100);
        let policy_client =
            PolicyEngineContractClient::new(&env, &client.get_policy_engine());
        policy_client.set_execution_policy(&false, &true, &u32::MAX);

        let action = InteractiveAction::Payment(PaymentAction {
            asset,
            destination,
            amount: 10,
        });
        let _ = admin;
        let err = client.try_execute_interactive(&action, &1_u32, &public);
        assert!(err.is_err());
    }

    #[test]
    fn execute_automation_consumes_required_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        env.ledger().with_mut(|li| {
            li.sequence_number = 60;
        });
        let transfer_adapter = env.register(TransferAdapterContract, ());
        let asset = issue_test_asset(&env).address();
        let destination = Address::generate(&env);
        let attestation_id = BytesN::from_array(&env, &[31; 32]);
        let capability_id = CapabilityId(BytesN::from_array(&env, &[32; 32]));
        let attestor_key = signing_key(101);

        configure_payment_policy(&env, &client, &transfer_adapter, &asset, &destination, 20);
        token::StellarAssetClient::new(&env, &asset).mint(&client.address, &20);

        let verifier = ConditionVerifierContractClient::new(&env, &client.get_condition_verifier());
        let verifier_address = client.get_condition_verifier();
        activate_verifier_attestor(
            &env,
            &verifier,
            BytesN::from_array(&env, &attestor_key.verifying_key().to_bytes()),
        );
        let executable_from = env.ledger().sequence() + 1;
        let executable_until = executable_from + 9;

        let capability = AutomationCapability {
            capability_id: capability_id.clone(),
            parent_intent_id: spf_shared_types::ParentIntentId(BytesN::from_array(&env, &[33; 32])),
            action: InteractiveAction::Payment(PaymentAction {
                asset: asset.clone(),
                destination: destination.clone(),
                amount: 10,
            }),
            required_attestation_id: Some(attestation_id.clone()),
            policy_version: 1,
            executable_from_ledger: executable_from,
            executable_until_ledger: executable_until,
            max_executions: 1,
        };
        client.grant_automation_capability(&capability);
        env.ledger().with_mut(|li| {
            li.sequence_number = executable_from;
        });
        let proof = sign_attestation_proof(
            &env,
            &client.address,
            &verifier_address,
            &[&attestor_key],
            attestation_id.clone(),
            capability_id.clone(),
            executable_until,
        );

        client.execute_automation(
            &capability_id,
            &ChildExecutionId(BytesN::from_array(&env, &[34; 32])),
            &Some(proof),
        );
        assert_eq!(token::Client::new(&env, &asset).balance(&destination), 10);
        assert!(verifier.is_attestation_consumed(&attestation_id));
    }

    #[test]
    fn execute_automation_rejects_when_attestor_quorum_is_not_met() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        env.ledger().with_mut(|li| {
            li.sequence_number = 90;
        });
        let transfer_adapter = env.register(TransferAdapterContract, ());
        let asset = issue_test_asset(&env).address();
        let destination = Address::generate(&env);
        let attestation_id = BytesN::from_array(&env, &[51; 32]);
        let capability_id = CapabilityId(BytesN::from_array(&env, &[52; 32]));
        let attestor_key_1 = signing_key(111);
        let attestor_key_2 = signing_key(112);

        configure_payment_policy(&env, &client, &transfer_adapter, &asset, &destination, 20);
        token::StellarAssetClient::new(&env, &asset).mint(&client.address, &20);

        let verifier = ConditionVerifierContractClient::new(&env, &client.get_condition_verifier());
        let verifier_address = client.get_condition_verifier();
        activate_verifier_attestor(
            &env,
            &verifier,
            BytesN::from_array(&env, &attestor_key_1.verifying_key().to_bytes()),
        );
        activate_verifier_attestor(
            &env,
            &verifier,
            BytesN::from_array(&env, &attestor_key_2.verifying_key().to_bytes()),
        );
        let threshold_ready_at = verifier.schedule_set_threshold(&2_u32);
        env.ledger().with_mut(|li| {
            li.sequence_number = threshold_ready_at;
        });
        verifier.apply_set_threshold();
        let executable_from = env.ledger().sequence() + 1;
        let executable_until = executable_from + 9;

        let capability = AutomationCapability {
            capability_id: capability_id.clone(),
            parent_intent_id: spf_shared_types::ParentIntentId(BytesN::from_array(&env, &[53; 32])),
            action: InteractiveAction::Payment(PaymentAction {
                asset,
                destination,
                amount: 10,
            }),
            required_attestation_id: Some(attestation_id.clone()),
            policy_version: 1,
            executable_from_ledger: executable_from,
            executable_until_ledger: executable_until,
            max_executions: 1,
        };
        client.grant_automation_capability(&capability);
        env.ledger().with_mut(|li| {
            li.sequence_number = executable_from;
        });

        let proof = sign_attestation_proof(
            &env,
            &client.address,
            &verifier_address,
            &[&attestor_key_1],
            attestation_id,
            capability_id.clone(),
            executable_until,
        );

        let err = client.try_execute_automation(
            &capability_id,
            &ChildExecutionId(BytesN::from_array(&env, &[54; 32])),
            &Some(proof),
        );
        assert!(err.is_err());
    }

    #[test]
    fn execute_automation_rejects_missing_required_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        env.ledger().with_mut(|li| {
            li.sequence_number = 80;
        });
        let transfer_adapter = env.register(TransferAdapterContract, ());
        let asset = issue_test_asset(&env).address();
        let destination = Address::generate(&env);
        let capability_id = CapabilityId(BytesN::from_array(&env, &[41; 32]));

        configure_payment_policy(&env, &client, &transfer_adapter, &asset, &destination, 20);
        token::StellarAssetClient::new(&env, &asset).mint(&client.address, &20);

        let capability = AutomationCapability {
            capability_id: capability_id.clone(),
            parent_intent_id: spf_shared_types::ParentIntentId(BytesN::from_array(&env, &[42; 32])),
            action: InteractiveAction::Payment(PaymentAction {
                asset,
                destination,
                amount: 10,
            }),
            required_attestation_id: Some(BytesN::from_array(&env, &[43; 32])),
            policy_version: 1,
            executable_from_ledger: 81,
            executable_until_ledger: 90,
            max_executions: 1,
        };
        client.grant_automation_capability(&capability);
        env.ledger().with_mut(|li| {
            li.sequence_number = 81;
        });

        let err = client.try_execute_automation(
            &capability_id,
            &ChildExecutionId(BytesN::from_array(&env, &[44; 32])),
            &None,
        );
        assert_eq!(err, Err(Ok(SmartAccountError::MissingRequiredAttestation)));
    }

    #[test]
    fn interactive_auth_rejects_mismatched_bound_signer_id() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let key = signing_key(71);
        let public = BytesN::from_array(&env, &key.verifying_key().to_bytes());
        let wrong_signer = BytesN::from_array(&env, &[17; 32]);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);

        client.add_signer(&ed25519_signer_record(&env, public.clone(), 1, 13));

        let action = InteractiveAction::Payment(PaymentAction {
            asset,
            destination,
            amount: 10,
        });
        let payload = BytesN::from_array(&env, &[14; 32]);
        let signature = key.sign(&payload.to_array());
        let signatures = vec![
            &env,
            AccountSignature {
                signer_id: public,
                signature: BytesN::from_array(&env, &signature.to_bytes()),
            },
        ];
        let auth_context = vec![
            &env,
            Context::Contract(ContractContext {
                contract: client.address.clone(),
                fn_name: Symbol::new(&env, "execute_interactive"),
                args: Vec::from_array(
                    &env,
                    [
                        action.into_val(&env),
                        1_u32.into_val(&env),
                        wrong_signer.into_val(&env),
                    ],
                ),
            }),
        ];

        let result = env.try_invoke_contract_check_auth::<SmartAccountError>(
            &client.address,
            &payload,
            signatures.into_val(&env),
            &auth_context,
        );
        assert_eq!(result, Err(Ok(SmartAccountError::SignerBindingMismatch)));
    }

    #[test]
    fn execute_interactive_dispatches_to_enabled_adapter() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let signer_key = signing_key(81);
        let public = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());
        let adapter_contract = env.register(SwapAdapterContract, ());
        let adapter_client = SwapAdapterContractClient::new(&env, &adapter_contract);
        let adapter_id = BytesN::from_array(&env, &[21; 32]);
        let asset_in = issue_test_asset(&env).address();
        let asset_out = issue_test_asset(&env).address();

        client.add_signer(&ed25519_signer_record(&env, public.clone(), 1, 14));
        configure_adapter_policy(
            &env,
            &client,
            &adapter_id,
            &adapter_contract,
            ADAPTER_TYPE_SWAP,
            &asset_in,
            50,
            true,
        );
        let route_hash = BytesN::from_array(&env, &[70; 32]);
        initialize_swap_adapter(&env, &adapter_contract, &route_hash);

        let action = InteractiveAction::Swap(SwapAction {
            adapter_id: adapter_id.clone(),
            asset_in: asset_in.clone(),
            asset_out: asset_out.clone(),
            amount_in: 25,
            quoted_amount_out: 25,
            min_amount_out: 24,
            route_hash: route_hash.clone(),
        });
        token::StellarAssetClient::new(&env, &asset_in).mint(&client.address, &25);
        client.set_asset_config(
            &asset_out,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 50,
            },
        );
        client.execute_interactive(&action, &1_u32, &public);

        let last = adapter_client.last_execution().expect("adapter execution should exist");
        assert_eq!(last.smart_account, client.address.clone());
        assert_eq!(last.asset_in, asset_in);
        assert_eq!(last.asset_out, asset_out);
        assert_eq!(last.amount_in, 25);
        assert_eq!(last.min_amount_out, 24);
        assert_eq!(last.route_hash, route_hash);
        assert_eq!(adapter_client.execution_count(), 1);
    }

    #[test]
    fn execute_interactive_dispatches_to_yield_adapter() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let signer_key = signing_key(82);
        let public = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());
        let adapter_contract = env.register(YieldAdapterContract, ());
        let adapter_client = YieldAdapterContractClient::new(&env, &adapter_contract);
        let adapter_id = BytesN::from_array(&env, &[23; 32]);
        let asset = issue_test_asset(&env).address();

        client.add_signer(&ed25519_signer_record(&env, public.clone(), 1, 65));
        configure_adapter_policy(
            &env,
            &client,
            &adapter_id,
            &adapter_contract,
            ADAPTER_TYPE_YIELD,
            &asset,
            50,
            true,
        );
        let vault = Address::generate(&env);
        initialize_yield_adapter(&env, &adapter_contract, &vault);

        let action = InteractiveAction::Yield(YieldAction {
            adapter_id,
            vault: vault.clone(),
            asset: asset.clone(),
            amount: 25,
            operation: YieldOperation::Deposit,
        });
        token::StellarAssetClient::new(&env, &asset).mint(&client.address, &25);
        client.execute_interactive(&action, &1_u32, &public);

        let last = adapter_client.last_execution().expect("yield execution should exist");
        assert_eq!(last.smart_account, client.address.clone());
        assert_eq!(last.vault, vault);
        assert_eq!(last.asset, asset);
        assert_eq!(last.amount, 25);
        assert_eq!(last.operation, 0);
        assert_eq!(adapter_client.execution_count(), 1);
    }

    #[test]
    fn execute_interactive_dispatches_to_split_adapter() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let signer_key = signing_key(83);
        let public = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());
        let adapter_contract = env.register(SplitAdapterContract, ());
        let adapter_client = SplitAdapterContractClient::new(&env, &adapter_contract);
        let adapter_id = BytesN::from_array(&env, &[24; 32]);
        let asset = issue_test_asset(&env).address();
        let recipient_1 = Address::generate(&env);
        let recipient_2 = Address::generate(&env);

        client.add_signer(&ed25519_signer_record(&env, public.clone(), 1, 66));
        configure_adapter_policy(
            &env,
            &client,
            &adapter_id,
            &adapter_contract,
            ADAPTER_TYPE_SPLIT,
            &asset,
            50,
            true,
        );
        initialize_split_adapter(&env, &adapter_contract, 8);
        client.set_destination_allowed(&recipient_1, &true);
        client.set_destination_allowed(&recipient_2, &true);

        let recipients = vec![
            &env,
            SplitRecipient {
                destination: recipient_1.clone(),
                amount: 10,
            },
            SplitRecipient {
                destination: recipient_2.clone(),
                amount: 15,
            },
        ];
        let action = InteractiveAction::Split(SplitAction {
            adapter_id,
            asset: asset.clone(),
            recipients: recipients.clone(),
        });
        token::StellarAssetClient::new(&env, &asset).mint(&client.address, &25);
        client.execute_interactive(&action, &1_u32, &public);

        let last = adapter_client.last_execution().expect("split execution should exist");
        assert_eq!(last.smart_account, client.address.clone());
        assert_eq!(last.asset, asset);
        assert_eq!(last.recipients, recipients);
        assert_eq!(adapter_client.execution_count(), 1);
    }

    #[test]
    fn execute_interactive_rejects_disabled_adapter() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, client) = setup_contract(&env);
        let signer_key = signing_key(91);
        let public = BytesN::from_array(&env, &signer_key.verifying_key().to_bytes());
        let adapter_contract = env.register(SwapAdapterContract, ());
        let adapter_id = BytesN::from_array(&env, &[22; 32]);
        let asset = issue_test_asset(&env).address();
        let asset_out = issue_test_asset(&env).address();

        client.add_signer(&ed25519_signer_record(&env, public.clone(), 1, 15));
        configure_adapter_policy(
            &env,
            &client,
            &adapter_id,
            &adapter_contract,
            ADAPTER_TYPE_SWAP,
            &asset,
            50,
            false,
        );
        client.set_asset_config(
            &asset_out,
            &AssetConfig {
                enabled: true,
                risk_tier: 1,
                max_single_transfer: 50,
            },
        );

        let action = InteractiveAction::Swap(SwapAction {
            adapter_id,
            asset_in: asset,
            asset_out,
            amount_in: 25,
            quoted_amount_out: 25,
            min_amount_out: 10,
            route_hash: BytesN::from_array(&env, &[71; 32]),
        });
        let err = client.try_execute_interactive(&action, &1_u32, &public);
        assert_eq!(err, Err(Ok(SmartAccountError::AdapterNotAllowed)));
    }

    fn action_asset(action: &InteractiveAction) -> Address {
        match action {
            InteractiveAction::Payment(payment) => payment.asset.clone(),
            InteractiveAction::Swap(adapter) => adapter.asset_in.clone(),
            InteractiveAction::Yield(adapter) => adapter.asset.clone(),
            InteractiveAction::Split(adapter) => adapter.asset.clone(),
        }
    }

    fn action_destination(action: &InteractiveAction) -> Address {
        match action {
            InteractiveAction::Payment(payment) => payment.destination.clone(),
            InteractiveAction::Split(adapter) => adapter
                .recipients
                .get(0)
                .expect("split action recipient should exist")
                .destination
                .clone(),
            InteractiveAction::Swap(_) | InteractiveAction::Yield(_) => {
                panic!("action has no single destination")
            }
        }
    }
}
