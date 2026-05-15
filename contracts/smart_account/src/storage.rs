use soroban_sdk::{contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Initialized,
    BootstrapAdmin,
    PrimarySignerCount,
    PrimarySignerWeightTotal,
    ManagementSignerWeightTotal,
    GovernanceSignerWeightTotal,
    RecoveryPrimaryWeightTotal,
    GuardianSignerCount,
    GuardianWeightTotal,
    ManagementThreshold,
    GovernanceThreshold,
    RecoveryThreshold,
    Paused,
    Frozen,
    PolicyVersion,
    PolicyEngine,
    IntentRegistry,
    ConditionVerifier,
    RecoveryManager,
    SignerIds,
    AssetConfig(Address),
    AdapterConfig(BytesN<32>),
    DestinationAllowlist(Address),
    Signer(BytesN<32>),
    Session(BytesN<32>),
    Capability(BytesN<32>),
    ConsumedChildExecution(BytesN<32>),
    PendingRecovery,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountConfig {
    pub bootstrap_admin: Address,
    pub management_threshold: u32,
    pub governance_threshold: u32,
    pub recovery_threshold: u32,
    pub policy_engine: Address,
    pub intent_registry: Address,
    pub condition_verifier: Address,
    pub recovery_manager: Address,
    pub policy_version: u32,
    pub paused: bool,
    pub frozen: bool,
}
