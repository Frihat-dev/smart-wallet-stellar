# Contract Specification

## 1. Purpose

This document defines the implementation-facing contract specification for version `v1`.

It covers:

- contract responsibilities
- storage layout
- entrypoints
- events
- error categories
- invariants

This is the bridge between the architectural design and the future Soroban implementation.

## 2. Contract Set

Version `v1` uses the following contracts:

- `SmartAccount`
- `IntentRegistry`
- `PolicyEngine`
- `ConditionVerifier`
- `RecoveryManager`
- `TransferAdapter`
- `SplitAdapter`
- `SwapAdapter`
- `YieldAdapter`

## 3. Shared Design Rules

### 3.1 Root authority

`SmartAccount` is the root authority.

Rules:

- only `SmartAccount` can authorize treasury execution
- subordinate contracts must not move funds on their own
- subordinate contracts must be invoked through controlled account flows

### 3.2 V1 asset scope

Version `v1` is `SAC-only`.

Rules:

- all supported assets must be Stellar Asset Contracts
- all adapters must reject non-approved assets

### 3.3 Authorization split

Two execution paths exist:

- `interactive`: validated through `__check_auth`
- `stored automation`: validated through explicit account execution logic

### 3.4 Cross-Contract Caller Authentication

Subordinate contracts must authenticate their caller.

Rules:

- state-mutating entrypoints in subordinate contracts must verify the caller is the owning `SmartAccount` or an explicitly approved system contract
- no subordinate contract may trust arbitrary external callers merely because the payload looks well-formed
- read-only validation functions may remain public if they do not mutate state

## 4. SmartAccount

### 4.1 Objective

Primary treasury contract account.

Responsibilities:

- hold treasury balances
- implement `__check_auth`
- store signer and automation state
- enforce account policy
- route execution to approved adapters
- coordinate recovery and freeze state

### 4.2 Storage

Recommended persistent keys:

- `initialized: bool`
- `policy_engine: Address`
- `intent_registry: Address`
- `condition_verifier: Address`
- `recovery_manager: Address`
- `paused: bool`
- `frozen: bool`
- `policy_version: u32`
- `next_signer_nonce: u64`
- `next_intent_nonce: u64`
- `next_execution_nonce: u64`
- `signer::<signer_id> -> SignerRecord`
- `session::<signer_id> -> SessionScope`
- `adapter::<adapter_id> -> AdapterConfig`
- `asset::<asset_address> -> AssetConfig`
- `allow_destination::<destination> -> bool`
- `cumulative_usage::<intent_id> -> UsageState`
- `ttl_config -> TtlConfig`

### 4.3 Core data structures

#### `SignerRecord`

- `signer_id: BytesN<32>`
- `signer_type: u32`
- `status: u32`
- `weight: u32`
- `created_ledger: u32`
- `expires_ledger: Option<u32>`
- `metadata_hash: BytesN<32>`

#### `SessionScope`

- `allowed_action_bitmap: u32`
- `allowed_assets: Vec<Address>` or bounded compact set representation
- `allowed_destinations: Vec<Address>` or bounded compact set representation
- `allowed_adapters: Vec<BytesN<32>>` or bounded compact set representation
- `per_execution_cap: i128`
- `cumulative_cap: i128`
- `consumed_amount: i128`
- `expiry_ledger: u32`
- `single_use: bool`

V1 implementation constraint:

- these collections must be strictly bounded in size
- if practical bounds are too small for expected use, prefer hashed membership maps over unbounded vectors

#### `AdapterConfig`

- `adapter_address: Address`
- `enabled: bool`
- `adapter_type: u32`
- `max_exposure_bps: u32`

#### `AssetConfig`

- `enabled: bool`
- `risk_tier: u32`
- `max_single_transfer: i128`

### 4.4 Entry points

#### Initialization and config

- `initialize(config)`
- `set_policy_engine(address)`
- `set_intent_registry(address)`
- `set_condition_verifier(address)`
- `set_recovery_manager(address)`
- `set_adapter(adapter_id, config)`
- `set_asset(asset, config)`
- `set_destination_allowlist(destination, enabled)`
- `pause()`
- `unpause()`

Access control:

- initialization is one-time only
- config mutations require strongest non-recovery admin path
- subordinate contract address updates require delayed governance path if mutability is enabled

#### Signer management

- `add_signer(record)`
- `remove_signer(signer_id)`
- `rotate_signer(old_signer_id, new_record)`
- `create_session_key(signer_id, scope)`
- `revoke_session_key(signer_id)`

Access control:

- signer management requires admin or stronger configured approval path
- session-key creation must validate bounded scope at creation time

#### Intent and execution

- `create_intent(intent_payload)`
- `cancel_intent(intent_id)`
- `execute_interactive(action_payload, signature_payload)`
- `execute_automation(intent_id, child_execution_id, execution_context)`

Access control:

- `execute_interactive` requires fresh signature-based authorization
- `execute_automation` must reject calls unless a valid stored capability exists

#### Recovery

- `freeze()`
- `unfreeze()`
- `begin_recovery(recovery_payload)`
- `finalize_recovery(recovery_id)`

Access control:

- recovery entrypoints are restricted to defined recovery path roles and guardian thresholds

#### Maintenance

- `extend_ttl(targets)`
- `prune_replay_state(intent_id, prune_range)`

Access control:

- `extend_ttl` may be callable by owner/admin and optionally permissionless for narrowly defined safe targets
- `prune_replay_state` may be permissionless only after deterministic maturity conditions are satisfied

### 4.5 Events

- `Initialized`
- `PolicyEngineUpdated`
- `IntentRegistryUpdated`
- `ConditionVerifierUpdated`
- `RecoveryManagerUpdated`
- `AdapterConfigured`
- `AssetConfigured`
- `DestinationAllowlistUpdated`
- `Paused`
- `Unpaused`
- `SignerAdded`
- `SignerRemoved`
- `SignerRotated`
- `SessionKeyCreated`
- `SessionKeyRevoked`
- `IntentCreated`
- `IntentCancelled`
- `InteractiveExecutionSucceeded`
- `AutomationExecutionSucceeded`
- `FreezeTriggered`
- `RecoveryStarted`
- `RecoveryFinalized`
- `TtlExtended`
- `ReplayStatePruned`

### 4.6 Errors

- `ERR_ALREADY_INITIALIZED`
- `ERR_UNAUTHORIZED`
- `ERR_PAUSED`
- `ERR_FROZEN`
- `ERR_INVALID_SIGNER`
- `ERR_SIGNER_EXPIRED`
- `ERR_SESSION_SCOPE`
- `ERR_POLICY_VERSION_MISMATCH`
- `ERR_ADAPTER_NOT_ALLOWED`
- `ERR_ASSET_NOT_ALLOWED`
- `ERR_DESTINATION_NOT_ALLOWED`
- `ERR_AMOUNT_EXCEEDS_CAP`
- `ERR_EXECUTION_REPLAY`
- `ERR_INVALID_EXECUTION_WINDOW`
- `ERR_INVALID_RECOVERY_STATE`

### 4.7 Invariants

- only `SmartAccount` may authorize treasury value movement
- no automation execution may exceed stored capability scope
- no stored automation may bypass freeze
- no asset outside approved SAC set may be used
- every successful execution must be uniquely recorded
- cumulative usage must never exceed stored caps

## 5. IntentRegistry

### 5.1 Objective

Track parent intents, child execution records, status transitions, and replay state.

### 5.2 Storage

- `parent_intent::<intent_id> -> ParentIntent`
- `child_execution::<child_execution_id> -> ChildExecution`
- `attestation_consumed::<attestation_id> -> bool`
- `intent_usage::<intent_id> -> UsageState`

### 5.3 Core data structures

#### `ParentIntent`

- `intent_id: BytesN<32>`
- `owner_account: Address`
- `policy_version: u32`
- `intent_type: u32`
- `adapter_id: BytesN<32>`
- `asset_set: Vec<Address>`
- `destination_set: Vec<Address>`
- `per_execution_cap: i128`
- `cumulative_cap: Option<i128>`
- `trigger_mode: u32`
- `schedule_definition_hash: BytesN<32>`
- `status: u32`
- `created_ledger: u32`
- `start_ledger: Option<u32>`
- `end_ledger: Option<u32>`
- `remaining_execution_count: Option<u32>`
- `metadata_hash: BytesN<32>`

V1 implementation constraint:

- destination and asset sets must be bounded
- recurrence definition must be encoded in a deterministic, versioned format

#### `ChildExecution`

- `child_execution_id: BytesN<32>`
- `parent_intent_id: BytesN<32>`
- `window_index: u64`
- `status: u32`
- `executed_ledger: Option<u32>`
- `amount_used: Option<i128>`

#### `UsageState`

- `cumulative_used: i128`
- `last_window_index: Option<u64>`

### 5.4 Entry points

- `register_parent_intent(intent)`
- `cancel_parent_intent(intent_id)`
- `consume_child_execution(child_execution_id, parent_intent_id, window_index)`
- `mark_child_executed(child_execution_id, amount)`
- `mark_child_skipped(child_execution_id)`
- `consume_attestation(attestation_id)`
- `prune_terminal_children(intent_id, until_window)`

Access control:

- registry mutation entrypoints must only be callable by the owning `SmartAccount`
- pruning may be permissionless only where docs explicitly allow it

### 5.5 Events

- `ParentIntentRegistered`
- `ParentIntentCancelled`
- `ChildExecutionConsumed`
- `ChildExecutionExecuted`
- `ChildExecutionSkipped`
- `AttestationConsumed`
- `TerminalChildrenPruned`

### 5.6 Errors

- `ERR_PARENT_INTENT_NOT_FOUND`
- `ERR_CHILD_ALREADY_CONSUMED`
- `ERR_PARENT_INTENT_TERMINAL`
- `ERR_WINDOW_ALREADY_SETTLED`
- `ERR_ATTESTATION_ALREADY_CONSUMED`
- `ERR_INVALID_PARENT_CHILD_RELATION`

### 5.7 Invariants

- every child execution id is unique
- a child execution may be settled at most once
- cumulative usage monotonically increases unless explicitly reset by terminal lifecycle rules
- terminal parent intents cannot create new child executions

## 6. PolicyEngine

### 6.1 Objective

Validate whether a requested action is permitted by the policy pinned to an account or intent.

### 6.2 Storage

- `policy_rules::<policy_version> -> PolicyRules`
- `global_asset::<asset> -> GlobalAssetRule`
- `global_adapter::<adapter_id> -> GlobalAdapterRule`

### 6.3 Core data structures

#### `PolicyRules`

- `policy_version: u32`
- `allowed_assets: Vec<Address>`
- `allowed_adapters: Vec<BytesN<32>>`
- `max_daily_outflow: i128`
- `max_single_outflow: i128`
- `high_risk_threshold: i128`
- `require_policy_cosign_above: Option<i128>`

### 6.4 Entry points

- `validate_interactive(account, action_context) -> bool`
- `validate_automation(account, parent_intent, child_execution, execution_context) -> bool`
- `validate_recovery(account, recovery_context) -> bool`

Access control:

- validation functions may be public and pure/view-like
- any future policy mutation path must be restricted to approved governance controls

### 6.5 Events

- `PolicyValidated`
- `PolicyRejected`

### 6.6 Errors

- `ERR_POLICY_NOT_FOUND`
- `ERR_POLICY_REJECTED`
- `ERR_ASSET_POLICY_REJECTED`
- `ERR_DESTINATION_POLICY_REJECTED`
- `ERR_ADAPTER_POLICY_REJECTED`
- `ERR_OUTFLOW_LIMIT_EXCEEDED`

### 6.7 Invariants

- validation is pure with respect to treasury funds
- policy evaluation must not mutate authorization-critical state
- the same input under the same policy version must always yield the same decision

## 7. ConditionVerifier

### 7.1 Objective

Verify attestation-based execution conditions for conditional intents.

### 7.2 Storage

- `attestor_set_version: u32`
- `attestor::<key_id> -> AttestorRecord`

### 7.3 Core data structures

#### `AttestorRecord`

- `key_id: BytesN<32>`
- `pubkey: Bytes`
- `enabled: bool`
- `weight: u32`

#### `AttestationPayload`

- `attestation_id: BytesN<32>`
- `intent_id: BytesN<32>`
- `account: Address`
- `event_type: u32`
- `event_time: u64`
- `expiry_time: u64`
- `payload_hash: BytesN<32>`
- `attestor_set_version: u32`

### 7.4 Entry points

- `verify_attestation(payload, signatures) -> bool`

Access control:

- attestation verification may be public
- any future attestor-set mutation path must be delayed and strongly authorized

### 7.5 Events

- `AttestationVerified`
- `AttestationRejected`

### 7.6 Errors

- `ERR_INVALID_ATTESTATION`
- `ERR_ATTESTATION_EXPIRED`
- `ERR_ATTESTOR_SET_MISMATCH`
- `ERR_ATTESTATION_THRESHOLD_NOT_MET`

### 7.7 Invariants

- attestation verification must be deterministic
- attestation validity must be domain-separated by account, intent, and network
- verifier must never record replay consumption itself unless explicitly designed to do so

## 8. RecoveryManager

### 8.1 Objective

Coordinate guardian-based freeze and recovery.

### 8.2 Storage

- `guardian_threshold: u32`
- `guardian::<guardian_id> -> GuardianRecord`
- `recovery::<recovery_id> -> RecoveryRecord`

### 8.3 Core data structures

#### `GuardianRecord`

- `guardian_id: BytesN<32>`
- `address_or_key_ref: Bytes`
- `enabled: bool`

#### `RecoveryRecord`

- `recovery_id: BytesN<32>`
- `status: u32`
- `proposed_signer_root: BytesN<32>`
- `start_ledger: u32`
- `finalize_after_ledger: u32`

### 8.4 Entry points

- `start_freeze(account, guardian_proof)`
- `start_recovery(account, recovery_payload)`
- `approve_recovery(recovery_id, guardian_proof)`
- `finalize_recovery(recovery_id)`

Access control:

- each mutating entrypoint must verify guardian approval thresholds or authorized recovery role

### 8.5 Events

- `FreezeStarted`
- `RecoveryStarted`
- `RecoveryApproved`
- `RecoveryFinalized`

### 8.6 Errors

- `ERR_GUARDIAN_UNAUTHORIZED`
- `ERR_RECOVERY_NOT_FOUND`
- `ERR_RECOVERY_NOT_READY`
- `ERR_RECOVERY_THRESHOLD_NOT_MET`

### 8.7 Invariants

- recovery must not directly transfer treasury assets
- freeze must prevent automation execution
- finalization must respect configured delay

## 9. TransferAdapter

### 9.1 Objective

Execute direct SAC transfers under account authority.

### 9.2 Entry points

- `execute_transfer(asset, to, amount)`

### 9.3 Events

- `TransferExecuted`

### 9.4 Errors

- `ERR_TRANSFER_FAILED`
- `ERR_UNSUPPORTED_ASSET`

### 9.5 Invariants

- adapter only transfers approved SAC assets
- adapter does not maintain autonomous authority

## 10. SplitAdapter

### 10.1 Objective

Execute revenue or treasury splits to approved destinations.

### 10.2 Data structures

- `SplitRule`
  - `destinations: Vec<Address>`
  - `bps: Vec<u32>`

### 10.3 Entry points

- `execute_split(asset, amount, split_rule)`

### 10.4 Events

- `SplitExecuted`

### 10.5 Errors

- `ERR_INVALID_BPS_SUM`
- `ERR_INVALID_SPLIT_DESTINATION`
- `ERR_SPLIT_EXECUTION_FAILED`

### 10.6 Invariants

- basis points sum to `10_000`
- each destination is approved before transfer

## 11. SwapAdapter

### 11.1 Objective

Execute tightly bounded swaps through an approved v1 venue.

### 11.2 Entry points

- `execute_swap(asset_in, asset_out, amount_in, min_out, route_hash)`

### 11.3 Events

- `SwapExecuted`

### 11.4 Errors

- `ERR_SWAP_ROUTE_NOT_ALLOWED`
- `ERR_SLIPPAGE_EXCEEDED`
- `ERR_SWAP_EXECUTION_FAILED`

### 11.5 Invariants

- swap route must match approved route constraints
- slippage checks must be enforced before final acceptance

## 12. YieldAdapter

### 12.1 Objective

Deposit and unwind treasury positions in one approved v1 yield venue.

### 12.2 Entry points

- `deposit(asset, amount, strategy_ref)`
- `withdraw(asset, amount, strategy_ref)`

### 12.3 Events

- `YieldDepositExecuted`
- `YieldWithdrawExecuted`

### 12.4 Errors

- `ERR_STRATEGY_NOT_ALLOWED`
- `ERR_EXPOSURE_LIMIT_EXCEEDED`
- `ERR_YIELD_EXECUTION_FAILED`

### 12.5 Invariants

- strategy exposure remains within configured cap
- adapter cannot redirect funds to arbitrary destinations

## 13. Global Event Guidelines

All state-changing flows should emit:

- account
- policy version
- intent id, if applicable
- child execution id, if applicable
- adapter id, if applicable
- asset and amount where relevant

Event versioning rule:

- every event family should have a stable version identifier in implementation, either by topic convention or payload field
- breaking event-shape changes must use a new version

## 14. Global Error Guidelines

Use deterministic, documented numeric error codes in implementation.

Recommended categories:

- auth errors
- policy errors
- replay errors
- asset errors
- adapter errors
- recovery errors
- maintenance errors

Recommended ownership rule:

- each contract should own a reserved numeric subrange inside the global category ranges
- error codes must be stable once published

## 15. Global Invariants

- no relayer may create authority
- no subordinate contract may escape account control
- no automation may exceed stored capability scope
- no replayed execution may succeed
- no frozen account may process treasury automation
- no unsupported asset may move through v1 adapters
