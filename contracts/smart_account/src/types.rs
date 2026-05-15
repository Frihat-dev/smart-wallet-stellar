use soroban_sdk::{contracttype, Vec, BytesN};
use spf_shared_types::{AutomationCapability, SessionScope, SignerRecord};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountStatusSnapshot {
    pub initialized: bool,
    pub paused: bool,
    pub frozen: bool,
    pub policy_version: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountSignature {
    pub signer_id: BytesN<32>,
    pub signature: BytesN<64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutomationCapabilityState {
    pub capability: AutomationCapability,
    pub revoked: bool,
    pub execution_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredSigner {
    pub record: SignerRecord,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredSession {
    pub scope: SessionScope,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingRecoveryPlan {
    pub activate_at_ledger: u32,
    pub primary_signers: Vec<SignerRecord>,
    pub guardian_signers: Vec<SignerRecord>,
    pub management_threshold: u32,
    pub governance_threshold: u32,
    pub recovery_threshold: u32,
}
