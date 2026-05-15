# State Machines and Storage Keys

## 1. Purpose

This document turns the higher-level design into implementation-ready state transition rules and storage-key conventions.

It covers:

- state machines
- allowed transitions
- storage-key enum design
- retention and pruning boundaries

## 2. Global Design Rules

- all state transitions must be explicit
- terminal states must be irreversible unless a separate recovery object exists
- all authorization-critical state must live in persistent storage
- pruning must never reopen an already-consumed execution path

## 3. SmartAccount State Machine

### 3.1 Account lifecycle states

- `uninitialized`
- `active`
- `paused`
- `frozen`

### 3.2 Allowed transitions

- `uninitialized -> active`
- `active -> paused`
- `paused -> active`
- `active -> frozen`
- `paused -> frozen`
- `frozen -> active`, only after successful recovery or authorized unfreeze

### 3.3 Forbidden transitions

- `uninitialized -> paused`
- `uninitialized -> frozen`
- `frozen -> paused`

### 3.4 Transition rules

- only initialization may move account out of `uninitialized`
- `paused` blocks normal execution but preserves signer and intent state
- `frozen` blocks interactive and automation treasury execution
- `frozen` may still allow narrowly scoped recovery actions

## 4. Parent Intent State Machine

### 4.1 States

- `draft`
- `active`
- `queued`
- `executable`
- `executed_terminal`, for single-use intents only
- `cancelled`
- `expired`
- `failed_terminal`

### 4.2 Allowed transitions

- `draft -> active`
- `active -> queued`
- `active -> executable`
- `queued -> executable`
- `executable -> executed_terminal`, for single-use intents after successful settlement
- `active -> cancelled`
- `queued -> cancelled`
- `executable -> cancelled`, only before atomic settlement begins
- `active -> expired`
- `queued -> expired`
- `executable -> expired`, only if no child execution has begun
- `active -> failed_terminal`, only on irreversible invalidation

### 4.3 Notes

- recurring intents typically remain `active` while child executions cycle independently
- single-use intents may end at `executed_terminal`
- policy migration should prefer `cancelled` over in-place mutation

## 5. Child Execution State Machine

### 5.1 States

- `pending`
- `consumed_in_progress`
- `executed`
- `skipped`
- `cancelled`
- `failed_terminal`

### 5.2 Allowed transitions

- `pending -> consumed_in_progress`
- `pending -> skipped`
- `pending -> cancelled`
- `pending -> failed_terminal`
- `consumed_in_progress -> executed`
- `consumed_in_progress -> failed_terminal`

### 5.3 Rules

- each child execution id may be consumed once
- `consumed_in_progress` exists only within atomic settlement flow
- if value movement fails, execution must not end in `executed`
- `skipped` is terminal

## 6. Session Key State Machine

### 6.1 States

- `active`
- `revoked`
- `expired`
- `consumed`, for single-use sessions

### 6.2 Allowed transitions

- `active -> revoked`
- `active -> expired`
- `active -> consumed`

### 6.3 Rules

- frozen account must treat all session keys as unusable
- expired and revoked session keys must never be reactivated

## 7. Recovery State Machine

### 7.1 States

- `none`
- `freeze_pending`
- `frozen`
- `recovery_pending`
- `recovery_ready`
- `recovered`
- `recovery_cancelled`

### 7.2 Allowed transitions

- `none -> freeze_pending`
- `freeze_pending -> frozen`
- `frozen -> recovery_pending`
- `recovery_pending -> recovery_ready`
- `recovery_ready -> recovered`
- `recovery_pending -> recovery_cancelled`

### 7.3 Rules

- recovery flow must not transfer funds
- recovery finalization only changes authorization state
- `recovered` should return account to `active` only through explicit account state update

## 8. Attestation Consumption State Machine

### 8.1 States

- `unused`
- `consumed`
- `pruned`

### 8.2 Allowed transitions

- `unused -> consumed`
- `consumed -> pruned`

### 8.3 Rules

- only a valid attestation may move to `consumed`
- `pruned` means replay safety has shifted to bounded lifecycle rules and retention expiry

## 9. Storage Key Enum Direction

Recommended Rust enum pattern:

```rust
pub enum DataKey {
    Initialized,
    Paused,
    Frozen,
    PolicyVersion,
    PolicyEngine,
    IntentRegistry,
    ConditionVerifier,
    RecoveryManager,
    TtlConfig,
    Signer(BytesN<32>),
    Session(BytesN<32>),
    Adapter(BytesN<32>),
    Asset(Address),
    Destination(Address),
    ParentIntent(BytesN<32>),
    ChildExecution(BytesN<32>),
    IntentUsage(BytesN<32>),
    ConsumedAttestation(BytesN<32>),
    Recovery(BytesN<32>),
}
```

Recommended implementation rule:

- each contract should define its own storage-key enum rather than sharing one giant enum across the full workspace
- shared key shapes may reuse common typed ids, but physical storage keys should remain contract-local

## 10. Storage-Key Rules

- all keys must have stable encoding
- auth-critical keys must not depend on mutable external ordering
- enum variants should be narrow and explicit
- map-like access should be modeled with typed keyed variants, not ad hoc string prefixes

## 11. Retention and Pruning Rules

### 11.1 Prunable records

- terminal child execution records
- consumed attestation records after maturity
- expired session records after maturity

### 11.2 Non-prunable records

- live signer records
- parent intents still capable of future execution
- recovery state not fully finalized
- cumulative counters needed for enforcing caps

### 11.3 Pruning invariants

- pruning must never make a prior execution eligible again
- pruning must never reduce cumulative usage below actual settled usage
- pruning must never remove data needed for current policy enforcement

## 12. Implementation Notes

- prefer small, typed storage objects
- avoid giant nested structures when partial reads/writes are common
- isolate execution and replay keys so tests can assert them directly
