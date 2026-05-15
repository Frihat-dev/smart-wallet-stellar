# Rust Module Layout and Test Matrix

## 1. Purpose

This document defines the recommended Soroban Rust project structure and the minimum test matrix for each contract area.

## 2. Workspace Direction

Recommended workspace layout:

```text
contracts/
  smart_account/
  intent_registry/
  policy_engine/
  condition_verifier/
  recovery_manager/
  adapters/
    transfer_adapter/
    split_adapter/
    swap_adapter/
    yield_adapter/
shared/
  types/
  events/
  errors/
  auth/
  testutils/
```

Implementation note:

- shared crates intended for contract use should remain compatible with Soroban `no_std` constraints
- any richer offchain-only helpers should live outside the onchain shared crate set

## 3. SmartAccount Module Layout

Recommended internal module split:

```text
smart_account/src/
  lib.rs
  contract.rs
  auth.rs
  types.rs
  storage.rs
  events.rs
  errors.rs
  execute_interactive.rs
  execute_automation.rs
  signer_management.rs
  session_keys.rs
  policy.rs
  ttl.rs
  recovery_hooks.rs
  tests/
```

## 4. IntentRegistry Module Layout

```text
intent_registry/src/
  lib.rs
  contract.rs
  types.rs
  storage.rs
  events.rs
  errors.rs
  parent_intents.rs
  child_executions.rs
  attestation_consumption.rs
  pruning.rs
  tests/
```

## 5. Shared Libraries

### 5.1 `shared/types`

- shared enums
- shared ids
- payload structs

### 5.2 `shared/events`

- event topic constants
- event serialization helpers

### 5.3 `shared/errors`

- global error code enum or grouped error definitions

### 5.4 `shared/auth`

- signing payload helpers
- domain separation helpers
- capability hash helpers

## 6. Error Code Matrix

This test matrix follows the per-contract reservation model defined in
[Payloads, Events, and Errors](./08-payloads-events-errors.md).

Recommended mapping:

- `SmartAccount`: `1000-1499`
- `IntentRegistry`: `1500-1999`
- `PolicyEngine`: `2000-2299`
- `ConditionVerifier`: `2300-2599`
- `RecoveryManager`: `2600-2999`
- adapters: `3000-3999`

## 7. Test Categories

Every contract should have tests in these categories:

- unit tests
- negative-path tests
- state transition tests
- replay tests
- TTL and pruning tests
- integration tests

## 8. SmartAccount Test Matrix

### 8.1 Initialization

- initialize once succeeds
- second initialization fails
- invalid subordinate contract references fail

### 8.2 Interactive auth

- valid interactive action succeeds
- invalid signer fails
- expired auth payload fails
- wrong network domain fails
- wrong policy version fails

### 8.3 Stored automation

- valid stored automation executes
- execution outside capability scope fails
- wrong adapter fails
- wrong destination fails
- amount above per-execution cap fails
- cumulative cap overflow fails

### 8.4 Session keys

- valid session-key action succeeds
- expired session fails
- revoked session fails
- single-use session cannot execute twice

### 8.5 Lifecycle

- paused account blocks execution
- frozen account blocks execution
- recovery-authorized path remains available while frozen

### 8.6 TTL

- successful auth extends critical TTL
- extend_ttl maintenance works
- expired critical state causes expected failure mode

## 9. IntentRegistry Test Matrix

### 9.1 Parent intents

- register parent intent succeeds
- duplicate parent id fails
- cancelling active parent succeeds
- terminal parent cannot spawn child execution

### 9.2 Child executions

- unique child execution can be consumed once
- duplicate consume fails
- consumed execution can settle once
- skipped child is terminal

### 9.3 Pruning

- mature terminal records can be pruned
- immature records cannot be pruned
- pruning preserves cumulative counters

## 10. PolicyEngine Test Matrix

- valid interactive policy passes
- invalid destination fails
- invalid asset fails
- single outflow above threshold fails
- cumulative usage above threshold fails
- pinned policy version mismatch fails

## 11. ConditionVerifier Test Matrix

- valid attestation verifies
- expired attestation fails
- wrong account binding fails
- wrong intent binding fails
- insufficient attestor quorum fails
- wrong attestor-set version fails

## 12. RecoveryManager Test Matrix

- guardian freeze path succeeds
- insufficient guardian approvals fail
- delayed recovery cannot finalize early
- recovery finalization updates auth state only
- weak bypass path does not exist in v1

## 13. Adapter Test Matrix

### 13.1 TransferAdapter

- approved SAC transfer succeeds
- unsupported asset fails
- invalid destination fails

### 13.2 SplitAdapter

- valid split sums to `10_000`
- invalid split bps fails
- duplicate settlement window fails

### 13.3 SwapAdapter

- allowed route succeeds
- disallowed route fails
- slippage breach fails

### 13.4 YieldAdapter

- approved strategy deposit succeeds
- exposure cap breach fails
- unapproved strategy fails

## 14. Cross-Contract Integration Tests

- create parent intent then execute child window successfully
- conditional payment with attestation succeeds
- duplicate relayer submission fails safely
- policy migration does not mutate pinned old automation
- frozen account rejects automation execution

## 15. Audit-Focused Tests

- no relayer-created authority
- no replay after pruning window rules
- no automation execution after revocation
- no cumulative cap underflow after compaction
- no account escape through subordinate contract

## 16. Suggested Build Discipline

- keep test fixtures deterministic
- prefer small helper builders for payloads and state
- keep shared constants centralized
- require each new entrypoint to add:
  - positive test
  - negative auth test
  - replay/state transition test
