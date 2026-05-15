use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PolicyEngineError {
    AlreadyInitialized = 2000,
    NotInitialized = 2001,
    Unauthorized = 2002,
    PolicyVersionMismatch = 2003,
    ActionNotAllowed = 2004,
    RiskTierTooHigh = 2005,
    NotImplemented = 2299,
}
