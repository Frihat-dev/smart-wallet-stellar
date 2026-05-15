use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyStatusSnapshot {
    pub initialized: bool,
    pub current_policy_version: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InteractiveActionKind {
    Payment = 0,
    Adapter = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionPolicy {
    pub allow_payments: bool,
    pub allow_adapters: bool,
    pub max_asset_risk_tier: u32,
}
