#![no_std]

use soroban_sdk::{contracttype, Address, BytesN, Vec};

pub const SIGNER_ROLE_PAYMENT: u32 = 1;
pub const SIGNER_ROLE_ADAPTER: u32 = 1 << 1;
pub const SIGNER_ROLE_SPEND: u32 = SIGNER_ROLE_PAYMENT | SIGNER_ROLE_ADAPTER;
pub const SIGNER_ROLE_MANAGEMENT: u32 = 1 << 2;
pub const SIGNER_ROLE_GOVERNANCE: u32 = 1 << 3;
pub const SIGNER_ROLE_RECOVERY: u32 = 1 << 4;
pub const SIGNER_ROLE_FULL_PRIMARY: u32 =
    SIGNER_ROLE_SPEND | SIGNER_ROLE_MANAGEMENT | SIGNER_ROLE_GOVERNANCE | SIGNER_ROLE_RECOVERY;
pub const SIGNER_ROLE_SESSION_DEFAULT: u32 = SIGNER_ROLE_SPEND;
pub const ADAPTER_TYPE_PAYMENT: u32 = 1;
pub const ADAPTER_TYPE_SWAP: u32 = 2;
pub const ADAPTER_TYPE_YIELD: u32 = 3;
pub const ADAPTER_TYPE_SPLIT: u32 = 4;
pub const YIELD_OP_DEPOSIT: u32 = 1;
pub const YIELD_OP_WITHDRAW: u32 = 1 << 1;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParentIntentId(pub BytesN<32>);

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildExecutionId(pub BytesN<32>);

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityId(pub BytesN<32>);

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRef {
    pub address: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SignerKind {
    Ed25519 = 0,
    PasskeyP256 = 1,
    PolicySigner = 2,
    SessionKey = 3,
    Guardian = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SignerStatus {
    Active = 0,
    Revoked = 1,
    Expired = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntentKind {
    ScheduledPayment = 0,
    ConditionalPayment = 1,
    RevenueSplit = 2,
    TreasuryRebalance = 3,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParentIntentStatus {
    Draft = 0,
    Active = 1,
    Queued = 2,
    Executable = 3,
    ExecutedTerminal = 4,
    Cancelled = 5,
    Expired = 6,
    FailedTerminal = 7,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChildExecutionStatus {
    Pending = 0,
    ConsumedInProgress = 1,
    Executed = 2,
    Skipped = 3,
    Cancelled = 4,
    FailedTerminal = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerRecord {
    pub signer_id: BytesN<32>,
    pub signer_kind: SignerKind,
    pub role_bitmap: u32,
    pub status: SignerStatus,
    pub weight: u32,
    pub created_ledger: u32,
    pub expires_ledger: Option<u32>,
    pub metadata_hash: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionScope {
    pub allowed_action_bitmap: u32,
    pub allowed_assets: Vec<Address>,
    pub allowed_destinations: Vec<Address>,
    pub allowed_adapters: Vec<BytesN<32>>,
    pub per_execution_cap: i128,
    pub cumulative_cap: i128,
    pub consumed_amount: i128,
    pub expiry_ledger: u32,
    pub single_use: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterConfig {
    pub adapter_address: Address,
    pub enabled: bool,
    pub adapter_type: u32,
    pub max_single_execution_amount: i128,
    pub allowed_assets: Vec<Address>,
    pub max_slippage_bps: u32,
    pub allowed_yield_operations: u32,
    pub max_split_recipients: u32,
    pub max_exposure_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetConfig {
    pub enabled: bool,
    pub risk_tier: u32,
    pub max_single_transfer: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentAction {
    pub asset: Address,
    pub destination: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapAction {
    pub adapter_id: BytesN<32>,
    pub asset_in: Address,
    pub asset_out: Address,
    pub amount_in: i128,
    pub quoted_amount_out: i128,
    pub min_amount_out: i128,
    pub route_hash: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum YieldOperation {
    Deposit = 0,
    Withdraw = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YieldAction {
    pub adapter_id: BytesN<32>,
    pub vault: Address,
    pub asset: Address,
    pub amount: i128,
    pub operation: YieldOperation,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitRecipient {
    pub destination: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitAction {
    pub adapter_id: BytesN<32>,
    pub asset: Address,
    pub recipients: Vec<SplitRecipient>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InteractiveAction {
    Payment(PaymentAction),
    Swap(SwapAction),
    Yield(YieldAction),
    Split(SplitAction),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutomationCapability {
    pub capability_id: CapabilityId,
    pub parent_intent_id: ParentIntentId,
    pub action: InteractiveAction,
    pub required_attestation_id: Option<BytesN<32>>,
    pub policy_version: u32,
    pub executable_from_ledger: u32,
    pub executable_until_ledger: u32,
    pub max_executions: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationProof {
    pub smart_account: Address,
    pub attestation_id: BytesN<32>,
    pub capability_id: CapabilityId,
    pub expires_ledger: u32,
    pub signatures: Vec<AttestationSignature>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationSignature {
    pub attestor: BytesN<32>,
    pub signature: BytesN<64>,
}
