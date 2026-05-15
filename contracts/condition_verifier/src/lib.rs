#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, xdr::ToXdr, Address, Bytes,
    BytesN, Env, Symbol,
};
use spf_shared_types::{AttestationProof, CapabilityId};

const DEFAULT_GOVERNANCE_DELAY_LEDGERS: u32 = 5;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Initialized,
    Owner,
    Threshold,
    GovernanceDelay,
    PendingOwner,
    PendingGovernanceDelay,
    ApprovedAttestorCount,
    ApprovedAttestor(BytesN<32>),
    PendingAttestorAdd(BytesN<32>),
    PendingAttestorRemove(BytesN<32>),
    PendingThreshold,
    ConsumedAttestation(BytesN<32>),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingLedgerChange {
    pub value: u32,
    pub activate_at_ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAttestorChange {
    pub activate_at_ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingOwnerChange {
    pub next_owner: Address,
    pub activate_at_ledger: u32,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ConditionVerifierError {
    AlreadyInitialized = 2300,
    NotInitialized = 2301,
    Unauthorized = 2302,
    AttestationExpired = 2304,
    AttestationConsumed = 2305,
    CapabilityMismatch = 2306,
    AttestorNotApproved = 2307,
    InvalidSignature = 2308,
    DuplicateAttestor = 2309,
    InsufficientAttestors = 2310,
    AttestorAlreadyApproved = 2311,
    PendingChangeNotFound = 2312,
    PendingChangeNotReady = 2313,
    AttestorAlreadyQueued = 2314,
    ThresholdTooHigh = 2315,
    InvalidGovernanceDelay = 2316,
    OwnerAcceptanceRequired = 2317,
}

#[contract]
pub struct ConditionVerifierContract;

#[contractimpl]
impl ConditionVerifierContract {
    pub fn contract_name() -> Symbol {
        symbol_short!("condverf")
    }

    pub fn initialize(env: Env, owner: Address) -> Result<(), ConditionVerifierError> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(ConditionVerifierError::AlreadyInitialized);
        }
        owner.require_auth();
        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage().persistent().set(&DataKey::Owner, &owner);
        env.storage().persistent().set(&DataKey::Threshold, &1_u32);
        env.storage()
            .persistent()
            .set(&DataKey::GovernanceDelay, &DEFAULT_GOVERNANCE_DELAY_LEDGERS);
        env.storage()
            .persistent()
            .set(&DataKey::ApprovedAttestorCount, &0_u32);
        Ok(())
    }

    pub fn get_governance_delay(env: Env) -> Result<u32, ConditionVerifierError> {
        ensure_initialized(&env)?;
        Ok(governance_delay(&env))
    }

    pub fn get_owner(env: Env) -> Result<Address, ConditionVerifierError> {
        ensure_initialized(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::Owner)
            .ok_or(ConditionVerifierError::NotInitialized)
    }

    pub fn get_threshold(env: Env) -> Result<u32, ConditionVerifierError> {
        ensure_initialized(&env)?;
        Ok(current_threshold(&env))
    }

    pub fn schedule_transfer_ownership(
        env: Env,
        next_owner: Address,
    ) -> Result<u32, ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        if current_owner(&env)? == next_owner {
            return Err(ConditionVerifierError::Unauthorized);
        }
        let activate_at = scheduled_activation_ledger(&env);
        env.storage().persistent().set(
            &DataKey::PendingOwner,
            &PendingOwnerChange {
                next_owner,
                activate_at_ledger: activate_at,
            },
        );
        Ok(activate_at)
    }

    pub fn apply_transfer_ownership(env: Env) -> Result<(), ConditionVerifierError> {
        ensure_initialized(&env)?;
        let pending = load_pending_owner(&env)?;
        ensure_change_ready(&env, pending.activate_at_ledger)?;
        pending.next_owner.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::Owner, &pending.next_owner);
        env.storage().persistent().remove(&DataKey::PendingOwner);
        Ok(())
    }

    pub fn cancel_transfer_ownership(env: Env) -> Result<(), ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        if !env.storage().persistent().has(&DataKey::PendingOwner) {
            return Err(ConditionVerifierError::PendingChangeNotFound);
        }
        env.storage().persistent().remove(&DataKey::PendingOwner);
        Ok(())
    }

    pub fn get_pending_owner(env: Env) -> Result<Option<PendingOwnerChange>, ConditionVerifierError> {
        ensure_initialized(&env)?;
        Ok(env.storage().persistent().get(&DataKey::PendingOwner))
    }

    pub fn schedule_set_governance_delay(
        env: Env,
        governance_delay: u32,
    ) -> Result<u32, ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        validate_governance_delay(governance_delay)?;
        let activate_at = scheduled_activation_ledger(&env);
        env.storage().persistent().set(
            &DataKey::PendingGovernanceDelay,
            &PendingLedgerChange {
                value: governance_delay,
                activate_at_ledger: activate_at,
            },
        );
        Ok(activate_at)
    }

    pub fn apply_set_governance_delay(env: Env) -> Result<(), ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        let pending = load_pending_governance_delay(&env)?;
        ensure_change_ready(&env, pending.activate_at_ledger)?;
        validate_governance_delay(pending.value)?;
        env.storage()
            .persistent()
            .set(&DataKey::GovernanceDelay, &pending.value);
        env.storage()
            .persistent()
            .remove(&DataKey::PendingGovernanceDelay);
        Ok(())
    }

    pub fn cancel_set_governance_delay(env: Env) -> Result<(), ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        if !env
            .storage()
            .persistent()
            .has(&DataKey::PendingGovernanceDelay)
        {
            return Err(ConditionVerifierError::PendingChangeNotFound);
        }
        env.storage()
            .persistent()
            .remove(&DataKey::PendingGovernanceDelay);
        Ok(())
    }

    pub fn get_pending_governance_delay(
        env: Env,
    ) -> Result<Option<PendingLedgerChange>, ConditionVerifierError> {
        ensure_initialized(&env)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::PendingGovernanceDelay))
    }

    pub fn schedule_add_attestor(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<u32, ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        if is_attestor_approved_internal(&env, &attestor) {
            return Err(ConditionVerifierError::AttestorAlreadyApproved);
        }
        if has_pending_attestor_change(&env, &attestor) {
            return Err(ConditionVerifierError::AttestorAlreadyQueued);
        }

        let activate_at = scheduled_activation_ledger(&env);
        env.storage().persistent().set(
            &DataKey::PendingAttestorAdd(attestor),
            &PendingAttestorChange {
                activate_at_ledger: activate_at,
            },
        );
        Ok(activate_at)
    }

    pub fn apply_add_attestor(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<(), ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        let pending = load_pending_attestor_add(&env, &attestor)?;
        ensure_change_ready(&env, pending.activate_at_ledger)?;

        env.storage()
            .persistent()
            .remove(&DataKey::PendingAttestorAdd(attestor.clone()));
        env.storage()
            .persistent()
            .set(&DataKey::ApprovedAttestor(attestor), &true);
        set_approved_attestor_count(&env, approved_attestor_count(&env) + 1);
        Ok(())
    }

    pub fn cancel_add_attestor(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<(), ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        if !env
            .storage()
            .persistent()
            .has(&DataKey::PendingAttestorAdd(attestor.clone()))
        {
            return Err(ConditionVerifierError::PendingChangeNotFound);
        }
        env.storage()
            .persistent()
            .remove(&DataKey::PendingAttestorAdd(attestor));
        Ok(())
    }

    pub fn schedule_remove_attestor(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<u32, ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        if !is_attestor_approved_internal(&env, &attestor) {
            return Err(ConditionVerifierError::AttestorNotApproved);
        }
        if has_pending_attestor_change(&env, &attestor) {
            return Err(ConditionVerifierError::AttestorAlreadyQueued);
        }
        let remaining = approved_attestor_count(&env).saturating_sub(1);
        if remaining < current_threshold(&env) {
            return Err(ConditionVerifierError::ThresholdTooHigh);
        }

        let activate_at = scheduled_activation_ledger(&env);
        env.storage().persistent().set(
            &DataKey::PendingAttestorRemove(attestor),
            &PendingAttestorChange {
                activate_at_ledger: activate_at,
            },
        );
        Ok(activate_at)
    }

    pub fn apply_remove_attestor(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<(), ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        let pending = load_pending_attestor_remove(&env, &attestor)?;
        ensure_change_ready(&env, pending.activate_at_ledger)?;

        let remaining = approved_attestor_count(&env).saturating_sub(1);
        if remaining < current_threshold(&env) {
            return Err(ConditionVerifierError::ThresholdTooHigh);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::PendingAttestorRemove(attestor.clone()));
        env.storage()
            .persistent()
            .remove(&DataKey::ApprovedAttestor(attestor));
        set_approved_attestor_count(&env, remaining);
        Ok(())
    }

    pub fn cancel_remove_attestor(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<(), ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        if !env
            .storage()
            .persistent()
            .has(&DataKey::PendingAttestorRemove(attestor.clone()))
        {
            return Err(ConditionVerifierError::PendingChangeNotFound);
        }
        env.storage()
            .persistent()
            .remove(&DataKey::PendingAttestorRemove(attestor));
        Ok(())
    }

    pub fn schedule_set_threshold(
        env: Env,
        threshold: u32,
    ) -> Result<u32, ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        validate_threshold(&env, threshold)?;

        let activate_at = scheduled_activation_ledger(&env);
        env.storage().persistent().set(
            &DataKey::PendingThreshold,
            &PendingLedgerChange {
                value: threshold,
                activate_at_ledger: activate_at,
            },
        );
        Ok(activate_at)
    }

    pub fn apply_set_threshold(env: Env) -> Result<(), ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        let pending = load_pending_threshold(&env)?;
        ensure_change_ready(&env, pending.activate_at_ledger)?;
        validate_threshold(&env, pending.value)?;
        env.storage()
            .persistent()
            .set(&DataKey::Threshold, &pending.value);
        env.storage().persistent().remove(&DataKey::PendingThreshold);
        Ok(())
    }

    pub fn cancel_set_threshold(env: Env) -> Result<(), ConditionVerifierError> {
        ensure_initialized(&env)?;
        require_owner(&env)?;
        if !env.storage().persistent().has(&DataKey::PendingThreshold) {
            return Err(ConditionVerifierError::PendingChangeNotFound);
        }
        env.storage().persistent().remove(&DataKey::PendingThreshold);
        Ok(())
    }

    pub fn get_pending_threshold(env: Env) -> Result<Option<PendingLedgerChange>, ConditionVerifierError> {
        ensure_initialized(&env)?;
        Ok(env.storage().persistent().get(&DataKey::PendingThreshold))
    }

    pub fn get_pending_add_attestor(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<Option<PendingAttestorChange>, ConditionVerifierError> {
        ensure_initialized(&env)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::PendingAttestorAdd(attestor)))
    }

    pub fn get_pending_remove_attestor(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<Option<PendingAttestorChange>, ConditionVerifierError> {
        ensure_initialized(&env)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::PendingAttestorRemove(attestor)))
    }

    pub fn consume_attestation(
        env: Env,
        proof: AttestationProof,
    ) -> Result<bool, ConditionVerifierError> {
        ensure_initialized(&env)?;
        if env
            .storage()
            .persistent()
            .has(&DataKey::ConsumedAttestation(proof.attestation_id.clone()))
        {
            return Err(ConditionVerifierError::AttestationConsumed);
        }
        if env.ledger().sequence() > proof.expires_ledger {
            return Err(ConditionVerifierError::AttestationExpired);
        }

        let threshold = current_threshold(&env);
        validate_threshold(&env, threshold)?;

        let payload = build_attestation_payload(
            &env,
            &proof.smart_account,
            &env.current_contract_address(),
            &proof.attestation_id,
            &proof.capability_id,
            proof.expires_ledger,
        );

        let mut verified_count = 0_u32;
        let mut seen = soroban_sdk::Vec::<BytesN<32>>::new(&env);

        for attestation_signature in proof.signatures.iter() {
            for prior in seen.iter() {
                if prior == attestation_signature.attestor {
                    return Err(ConditionVerifierError::DuplicateAttestor);
                }
            }
            if !is_attestor_approved_internal(&env, &attestation_signature.attestor) {
                return Err(ConditionVerifierError::AttestorNotApproved);
            }
            env.crypto().ed25519_verify(
                &attestation_signature.attestor,
                &payload,
                &attestation_signature.signature,
            );
            seen.push_back(attestation_signature.attestor.clone());
            verified_count += 1;
        }

        if verified_count < threshold {
            return Err(ConditionVerifierError::InsufficientAttestors);
        }

        env.storage()
            .persistent()
            .set(&DataKey::ConsumedAttestation(proof.attestation_id), &true);
        Ok(true)
    }

    pub fn is_attestor_approved(
        env: Env,
        attestor: BytesN<32>,
    ) -> Result<bool, ConditionVerifierError> {
        ensure_initialized(&env)?;
        Ok(is_attestor_approved_internal(&env, &attestor))
    }

    pub fn is_attestation_consumed(
        env: Env,
        attestation_id: BytesN<32>,
    ) -> Result<bool, ConditionVerifierError> {
        ensure_initialized(&env)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::ConsumedAttestation(attestation_id))
            .unwrap_or(false))
    }
}

fn ensure_initialized(env: &Env) -> Result<(), ConditionVerifierError> {
    if env.storage().persistent().has(&DataKey::Initialized) {
        Ok(())
    } else {
        Err(ConditionVerifierError::NotInitialized)
    }
}

fn require_owner(env: &Env) -> Result<(), ConditionVerifierError> {
    let owner = current_owner(env)?;
    owner.require_auth();
    Ok(())
}

fn current_owner(env: &Env) -> Result<Address, ConditionVerifierError> {
    env.storage()
        .persistent()
        .get(&DataKey::Owner)
        .ok_or(ConditionVerifierError::NotInitialized)
}

fn governance_delay(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::GovernanceDelay)
        .unwrap_or(DEFAULT_GOVERNANCE_DELAY_LEDGERS)
}

fn current_threshold(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::Threshold)
        .unwrap_or(1_u32)
}

fn approved_attestor_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::ApprovedAttestorCount)
        .unwrap_or(0_u32)
}

fn set_approved_attestor_count(env: &Env, count: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::ApprovedAttestorCount, &count);
}

fn validate_threshold(env: &Env, threshold: u32) -> Result<(), ConditionVerifierError> {
    if threshold == 0 {
        return Err(ConditionVerifierError::InsufficientAttestors);
    }
    if threshold > approved_attestor_count(env) {
        return Err(ConditionVerifierError::ThresholdTooHigh);
    }
    Ok(())
}

fn validate_governance_delay(governance_delay: u32) -> Result<(), ConditionVerifierError> {
    if governance_delay == 0 {
        return Err(ConditionVerifierError::InvalidGovernanceDelay);
    }
    Ok(())
}

fn scheduled_activation_ledger(env: &Env) -> u32 {
    env.ledger().sequence().saturating_add(governance_delay(env))
}

fn ensure_change_ready(env: &Env, activate_at_ledger: u32) -> Result<(), ConditionVerifierError> {
    if env.ledger().sequence() < activate_at_ledger {
        return Err(ConditionVerifierError::PendingChangeNotReady);
    }
    Ok(())
}

fn has_pending_attestor_change(env: &Env, attestor: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::PendingAttestorAdd(attestor.clone()))
        || env
            .storage()
            .persistent()
            .has(&DataKey::PendingAttestorRemove(attestor.clone()))
}

fn is_attestor_approved_internal(env: &Env, attestor: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::ApprovedAttestor(attestor.clone()))
        .unwrap_or(false)
}

fn load_pending_attestor_add(
    env: &Env,
    attestor: &BytesN<32>,
) -> Result<PendingAttestorChange, ConditionVerifierError> {
    env.storage()
        .persistent()
        .get(&DataKey::PendingAttestorAdd(attestor.clone()))
        .ok_or(ConditionVerifierError::PendingChangeNotFound)
}

fn load_pending_attestor_remove(
    env: &Env,
    attestor: &BytesN<32>,
) -> Result<PendingAttestorChange, ConditionVerifierError> {
    env.storage()
        .persistent()
        .get(&DataKey::PendingAttestorRemove(attestor.clone()))
        .ok_or(ConditionVerifierError::PendingChangeNotFound)
}

fn load_pending_threshold(env: &Env) -> Result<PendingLedgerChange, ConditionVerifierError> {
    env.storage()
        .persistent()
        .get(&DataKey::PendingThreshold)
        .ok_or(ConditionVerifierError::PendingChangeNotFound)
}

fn load_pending_governance_delay(env: &Env) -> Result<PendingLedgerChange, ConditionVerifierError> {
    env.storage()
        .persistent()
        .get(&DataKey::PendingGovernanceDelay)
        .ok_or(ConditionVerifierError::PendingChangeNotFound)
}

fn load_pending_owner(env: &Env) -> Result<PendingOwnerChange, ConditionVerifierError> {
    env.storage()
        .persistent()
        .get(&DataKey::PendingOwner)
        .ok_or(ConditionVerifierError::PendingChangeNotFound)
}

pub fn build_attestation_payload(
    env: &Env,
    smart_account: &Address,
    verifier_contract: &Address,
    attestation_id: &BytesN<32>,
    capability_id: &CapabilityId,
    expires_ledger: u32,
) -> Bytes {
    let mut payload = Bytes::from_slice(env, b"condverf-v1");
    payload.append(&smart_account.to_xdr(env));
    payload.append(&verifier_contract.to_xdr(env));
    payload.append(&Bytes::from(attestation_id.clone()));
    payload.append(&Bytes::from(capability_id.0.clone()));
    payload.extend_from_slice(&expires_ledger.to_be_bytes());
    payload
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::{rngs::StdRng, SeedableRng};
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn signing_key(seed: u8) -> SigningKey {
        let mut rng = StdRng::from_seed([seed; 32]);
        SigningKey::generate(&mut rng)
    }

    fn setup_contract(env: &Env) -> (Address, ConditionVerifierContractClient<'_>) {
        let contract_id = env.register(ConditionVerifierContract, ());
        let client = ConditionVerifierContractClient::new(env, &contract_id);
        let owner = Address::generate(env);
        client.initialize(&owner);
        (owner, client)
    }

    fn activate_attestor(
        env: &Env,
        client: &ConditionVerifierContractClient<'_>,
        attestor: BytesN<32>,
    ) {
        let ready_at = client.schedule_add_attestor(&attestor);
        env.ledger().with_mut(|li| {
            li.sequence_number = ready_at;
        });
        client.apply_add_attestor(&attestor);
    }

    fn sign_proof(
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
            signatures.push_back(spf_shared_types::AttestationSignature {
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

    #[test]
    fn scheduled_attestor_is_not_active_until_applied() {
        let env = Env::default();
        env.mock_all_auths();
        let (_owner, client) = setup_contract(&env);
        let attestor = BytesN::from_array(&env, &signing_key(1).verifying_key().to_bytes());

        let activate_at = client.schedule_add_attestor(&attestor);
        assert_eq!(activate_at, DEFAULT_GOVERNANCE_DELAY_LEDGERS);
        assert!(!client.is_attestor_approved(&attestor));

        let err = client.try_apply_add_attestor(&attestor);
        assert_eq!(err, Err(Ok(ConditionVerifierError::PendingChangeNotReady)));

        env.ledger().with_mut(|li| {
            li.sequence_number = activate_at;
        });
        client.apply_add_attestor(&attestor);
        assert!(client.is_attestor_approved(&attestor));
    }

    #[test]
    fn threshold_change_is_delayed_and_cancellable() {
        let env = Env::default();
        env.mock_all_auths();
        let (_owner, client) = setup_contract(&env);
        let attestor_1 = BytesN::from_array(&env, &signing_key(2).verifying_key().to_bytes());
        let attestor_2 = BytesN::from_array(&env, &signing_key(3).verifying_key().to_bytes());
        activate_attestor(&env, &client, attestor_1);
        activate_attestor(&env, &client, attestor_2);

        let _ready_at = client.schedule_set_threshold(&2_u32);
        assert_eq!(client.get_threshold(), 1);
        client.cancel_set_threshold();
        assert_eq!(client.get_pending_threshold(), None);

        let ready_at = client.schedule_set_threshold(&2_u32);
        env.ledger().with_mut(|li| {
            li.sequence_number = ready_at;
        });
        client.apply_set_threshold();
        assert_eq!(client.get_threshold(), 2);
    }

    #[test]
    fn ownership_transfer_is_delayed_and_cancellable() {
        let env = Env::default();
        env.mock_all_auths();
        let (owner, client) = setup_contract(&env);
        let next_owner = Address::generate(&env);

        assert_eq!(client.get_owner(), owner);
        let _ready_at = client.schedule_transfer_ownership(&next_owner);
        let pending = client.get_pending_owner().unwrap();
        assert_eq!(pending.next_owner, next_owner);

        let err = client.try_apply_transfer_ownership();
        assert_eq!(err, Err(Ok(ConditionVerifierError::PendingChangeNotReady)));

        client.cancel_transfer_ownership();
        assert_eq!(client.get_pending_owner(), None);

        let ready_at = client.schedule_transfer_ownership(&next_owner);
        env.ledger().with_mut(|li| {
            li.sequence_number = ready_at;
        });
        client.apply_transfer_ownership();
        assert_eq!(client.get_owner(), next_owner);
    }

    #[test]
    fn governance_delay_change_is_delayed_and_affects_future_schedules() {
        let env = Env::default();
        env.mock_all_auths();
        let (_owner, client) = setup_contract(&env);

        let err = client.try_schedule_set_governance_delay(&0_u32);
        assert_eq!(err, Err(Ok(ConditionVerifierError::InvalidGovernanceDelay)));

        let _ready_at = client.schedule_set_governance_delay(&9_u32);
        assert_eq!(client.get_governance_delay(), DEFAULT_GOVERNANCE_DELAY_LEDGERS);

        let err = client.try_apply_set_governance_delay();
        assert_eq!(err, Err(Ok(ConditionVerifierError::PendingChangeNotReady)));

        client.cancel_set_governance_delay();
        assert_eq!(client.get_pending_governance_delay(), None);

        let ready_at = client.schedule_set_governance_delay(&9_u32);
        env.ledger().with_mut(|li| {
            li.sequence_number = ready_at;
        });
        client.apply_set_governance_delay();
        assert_eq!(client.get_governance_delay(), 9);

        let attestor = BytesN::from_array(&env, &signing_key(41).verifying_key().to_bytes());
        let next_ready_at = client.schedule_add_attestor(&attestor);
        assert_eq!(next_ready_at, env.ledger().sequence() + 9);
    }

    #[test]
    fn consume_attestation_accepts_quorum_signed_proof() {
        let env = Env::default();
        env.mock_all_auths();
        let (_owner, client) = setup_contract(&env);
        let smart_account = Address::generate(&env);
        let attestation_id = BytesN::from_array(&env, &[1; 32]);
        let capability_id = CapabilityId(BytesN::from_array(&env, &[2; 32]));
        let attestor_1 = signing_key(11);
        let attestor_2 = signing_key(12);

        activate_attestor(
            &env,
            &client,
            BytesN::from_array(&env, &attestor_1.verifying_key().to_bytes()),
        );
        activate_attestor(
            &env,
            &client,
            BytesN::from_array(&env, &attestor_2.verifying_key().to_bytes()),
        );

        let ready_at = client.schedule_set_threshold(&2_u32);
        env.ledger().with_mut(|li| {
            li.sequence_number = ready_at;
        });
        client.apply_set_threshold();

        let proof = sign_proof(
            &env,
            &smart_account,
            &client.address,
            &[&attestor_1, &attestor_2],
            attestation_id.clone(),
            capability_id,
            120,
        );
        assert!(client.consume_attestation(&proof));
        assert!(client.is_attestation_consumed(&attestation_id));
    }

    #[test]
    fn removing_attestor_that_would_break_threshold_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_owner, client) = setup_contract(&env);
        let attestor_1 = BytesN::from_array(&env, &signing_key(21).verifying_key().to_bytes());
        let attestor_2 = BytesN::from_array(&env, &signing_key(22).verifying_key().to_bytes());
        activate_attestor(&env, &client, attestor_1.clone());
        activate_attestor(&env, &client, attestor_2.clone());
        let ready_at = client.schedule_set_threshold(&2_u32);
        env.ledger().with_mut(|li| {
            li.sequence_number = ready_at;
        });
        client.apply_set_threshold();

        let err = client.try_schedule_remove_attestor(&attestor_1);
        assert_eq!(err, Err(Ok(ConditionVerifierError::ThresholdTooHigh)));

        let lower_ready_at = client.schedule_set_threshold(&1_u32);
        env.ledger().with_mut(|li| {
            li.sequence_number = lower_ready_at;
        });
        client.apply_set_threshold();
        let remove_ready_at = client.schedule_remove_attestor(&attestor_1);
        env.ledger().with_mut(|li| {
            li.sequence_number = remove_ready_at;
        });
        client.apply_remove_attestor(&attestor_1);
        assert!(!client.is_attestor_approved(&attestor_1));
        assert!(client.is_attestor_approved(&attestor_2));
    }

    #[test]
    fn consume_attestation_rejects_duplicate_attestor_entries() {
        let env = Env::default();
        env.mock_all_auths();
        let (_owner, client) = setup_contract(&env);
        let smart_account = Address::generate(&env);
        let attestation_id = BytesN::from_array(&env, &[3; 32]);
        let capability_id = CapabilityId(BytesN::from_array(&env, &[4; 32]));
        let attestor = signing_key(31);

        activate_attestor(
            &env,
            &client,
            BytesN::from_array(&env, &attestor.verifying_key().to_bytes()),
        );

        let proof = sign_proof(
            &env,
            &smart_account,
            &client.address,
            &[&attestor, &attestor],
            attestation_id,
            capability_id,
            220,
        );
        let err = client.try_consume_attestation(&proof);
        assert_eq!(err, Err(Ok(ConditionVerifierError::DuplicateAttestor)));
    }
}
