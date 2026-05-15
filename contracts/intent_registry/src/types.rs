use soroban_sdk::contracttype;
use spf_shared_types::ParentIntentStatus;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryStatusSnapshot {
    pub initialized: bool,
    pub parent_status_template: ParentIntentStatus,
}
