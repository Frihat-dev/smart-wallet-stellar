use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Initialized,
    Owner,
    CurrentPolicyVersion,
    AllowPayments,
    AllowAdapters,
    MaxAssetRiskTier,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyConfig {
    pub owner: Address,
    pub current_policy_version: u32,
}
