use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum IntentRegistryError {
    AlreadyInitialized = 1500,
    NotInitialized = 1501,
    Unauthorized = 1502,
    NotImplemented = 1999,
}
