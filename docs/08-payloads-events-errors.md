# Payloads, Events, and Errors

## 1. Purpose

This document defines the canonical schema direction for:

- authorization payloads
- automation capability payloads
- events
- error categories

It is intended to keep the implementation consistent across contracts.

## 2. Interactive Authorization Payload

Recommended fields:

- `network_id`
- `smart_account`
- `signer_id`
- `signer_type`
- `action_type`
- `adapter_id`
- `asset`
- `destination`
- `amount`
- `intent_id`, optional
- `execution_nonce`
- `expiry_ledger`
- `policy_version`
- `payload_hash`

## 3. Stored Automation Capability Payload

Recommended fields:

- `capability_id`
- `parent_intent_id`
- `policy_version`
- `action_type`
- `adapter_id`
- `allowed_assets`
- `allowed_destinations`
- `per_execution_cap`
- `cumulative_cap`
- `trigger_mode`
- `schedule_hash`
- `start_ledger`
- `end_ledger`
- `remaining_execution_count`
- `revoked`

## 4. Child Execution Payload

Recommended fields:

- `child_execution_id`
- `parent_intent_id`
- `window_index`
- `requested_amount`
- `execution_context_hash`

## 5. Event Schema Direction

Every execution event should include:

- `account`
- `policy_version`
- `action_type`
- `adapter_id`
- `intent_id`, if applicable
- `child_execution_id`, if applicable
- `asset`, if applicable
- `amount`, if applicable
- `status`

Recommended additional fields:

- `event_version`
- `ledger_sequence`, when available in implementation context

## 6. Error Model

Implementation should group error codes by domain:

- `1000-1999` auth and signer errors
- `2000-2999` policy and capability errors
- `3000-3999` replay and execution-state errors
- `4000-4999` asset and adapter errors
- `5000-5999` recovery and guardian errors
- `6000-6999` maintenance and TTL errors

Recommended per-contract reservation:

- `SmartAccount`: `1000-1499`
- `IntentRegistry`: `1500-1999`
- `PolicyEngine`: `2000-2299`
- `ConditionVerifier`: `2300-2599`
- `RecoveryManager`: `2600-2999`
- adapters: `3000-3999`

## 7. V1 Notes

- automation capability and interactive payloads must remain distinct
- payload serialization must be canonical and versioned
- event fields should remain stable once implementation begins
