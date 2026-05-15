# System Architecture

## 1. Design Principles

- use contract accounts as first-class smart treasury wallets
- keep execution surface narrow and auditable
- separate authorization from business policy where possible
- default to explicit allowlists
- rely on deterministic contract logic, not trusted operators

## 2. Core Components

### 2.1 SmartAccount

Primary treasury contract account.

Responsibilities:

- hold user balances
- implement `__check_auth`
- validate signer permissions
- validate scoped session keys
- route allowed execution into adapters
- manage pause, freeze, and recovery state

### 2.2 PolicyEngine

Policy validation module for account-level rules.

Responsibilities:

- enforce destination allowlists
- enforce amount caps
- enforce daily outflow windows
- enforce time rules
- enforce protocol exposure limits
- require policy signer participation for high-risk actions

### 2.3 IntentRegistry

Tracks declared intents and execution state.

Responsibilities:

- store intent metadata
- track status and expiration
- prevent duplicate or stale execution
- record attestation use for conditional flows

### 2.4 ExecutionAdapters

Tightly scoped action modules.

Initial adapter set:

- transfer adapter
- split adapter
- swap adapter
- treasury yield adapter

Responsibilities:

- validate action-specific inputs
- interact only with approved contracts
- reject unknown destinations, routes, and assets

### 2.5 ConditionVerifier

Verification module for external trigger proofs.

Responsibilities:

- verify approved attestor signatures
- check event freshness
- bind evidence to account and intent id
- prevent replay of offchain condition events

### 2.6 RecoveryManager

Recovery and emergency control module.

Responsibilities:

- freeze account on compromise
- manage guardian approval flow
- coordinate delayed signer replacement

## 3. Account Model

The treasury wallet is a Soroban contract account identified by a `C...` address.

Important constraints:

- it authorizes actions using `__check_auth`
- it cannot act as transaction source account
- a relayer or external `G...` account submits transactions and pays fees

## 3.1 Authority Boundaries

The `SmartAccount` is the root authority for the system.

Rules:

- only `SmartAccount` can authorize treasury execution
- `PolicyEngine`, `IntentRegistry`, `ConditionVerifier`, and adapters are subordinate components
- subordinate components must never be able to move funds independently
- if a subordinate contract is replaceable, replacement must require strongest auth tier and explicit delay

Recommended v1 model:

- `SmartAccount` stores the canonical addresses of:
  - `PolicyEngine`
  - `IntentRegistry`
  - `ConditionVerifier`
  - approved adapters
- these addresses are immutable after initialization for v1, unless a delayed governance path is explicitly implemented

If mutability is later required:

- changes must be delayed
- changes must emit explicit events
- pending and future executions of existing automations must remain pinned to the policy version under which they were created

## 3.2 Policy Version Pinning

Version `v1` adopts strict policy pinning for automations.

Rules:

- every stored automation capability is permanently bound to the policy version active at creation time
- later policy updates must not silently rewrite existing automation semantics
- if execution semantics need to change, new automations must be created
- administrators may explicitly cancel old automations after migration, but must not mutate them in place

## 4. Signer Model

### 4.1 Signer classes

- `owner_passkey`
- `admin_key`
- `policy_signer`
- `session_key`
- `guardian`

### 4.2 Auth tiers

- `Tier 1`
  - small pre-approved transfers
  - session key allowed
- `Tier 2`
  - larger transfers or non-routine actions
  - passkey required
- `Tier 3`
  - config changes, signer rotation, protocol enablement
  - passkey plus policy signer or equivalent strong quorum
- `Tier 4`
  - recovery and freeze actions
  - guardian quorum and optional delay

## 4.3 Signer Records

Each signer record should include:

- `signer_id`
- `signer_type`
- `status`
- `weight` or approval class if multi-approval is used
- `created_at_ledger`
- `expires_at_ledger`, if temporary
- `scope`, if limited signer
- `metadata_hash`

Supported `signer_type` values:

- `passkey_p256`
- `ed25519_admin`
- `policy_signer`
- `session_key`
- `guardian`

## 4.4 Session Key Scope

Each session key must bind to a narrow capability envelope:

- allowed action types
- allowed assets
- allowed destinations
- maximum cumulative spend
- expiry ledger or timestamp
- optional single-use flag
- optional adapter allowlist

Session keys must not inherit general account authority.

## 5. Intent Model

Intent record fields:

- `intent_id`
- `account`
- `intent_type`
- `asset`
- `amount_rule`
- `destination_rule`
- `trigger_rule`
- `adapter_id`
- `risk_tier`
- `status`
- `created_at`
- `expires_at`
- `metadata_hash`

Supported `intent_type` values for v1:

- `scheduled_payment`
- `conditional_payment`
- `revenue_split`
- `treasury_rebalance`

## 5.1 Intent State Machine

Each intent must follow an explicit state machine:

- `draft`
- `active`
- `queued`
- `executable`
- `executed`
- `cancelled`
- `expired`
- `failed_terminal`

Rules:

- only `active` or `queued` intents may become `executable`
- only `executable` intents may become `executed`
- `executed`, `cancelled`, and `expired` are terminal
- `failed_terminal` is used only when an intent should never be retried

## 5.2 Intent Identity and Replay Resistance

Each intent id must be unique and collision-resistant.

Recommended derivation:

- hash of account id
- creator nonce
- intent type
- creation ledger
- policy version

Each execution attempt must also bind:

- `intent_id`
- `execution_nonce`
- `policy_version`
- `adapter_id`
- `network_passphrase` or equivalent network domain

This prevents replay across:

- different accounts
- different policy versions
- different networks
- different execution attempts

## 5.3 Automation Authorization Model

The system must distinguish between:

- `interactive authorization`
- `stored execution rights`

### Interactive authorization

Used for:

- immediate payments
- signer changes
- policy updates
- recovery actions

In this mode, the signer authorizes a specific action request directly.

### Stored execution rights

Used for:

- scheduled payments
- recurring revenue splits
- recurring treasury rebalances
- conditional execution after approved attestation

In this mode, the signer does **not** sign each future execution individually.
Instead, the signer authorizes creation of a bounded automation capability at intent-creation time.

That capability is then enforced onchain by `SmartAccount`, `PolicyEngine`, and `IntentRegistry`.

## 5.4 Automation Capability Record

Each automatable intent must create a stored capability record containing:

- `intent_id`
- `policy_version`
- `authorized_action_type`
- `authorized_adapter_id`
- `authorized_assets`
- `authorized_destinations`
- `per_execution_cap`
- `cumulative_cap`, if applicable
- `schedule_or_trigger_definition`
- `start_ledger_or_time`
- `end_ledger_or_time`
- `remaining_execution_count` or recurrence rule
- `revocation_status`

Security rule:

- relayers may trigger execution only within the stored capability envelope
- relayers do not supply privileged authority of their own
- stored automation execution must be validated by explicit `SmartAccount` entrypoint logic against onchain capability state
- `__check_auth` is reserved for cases where fresh signature-based authorization is actually being presented

## 5.5 Best Answer for Recurring Intent Structure

The recommended model is:

- one persistent parent intent defining policy and recurrence
- one materialized child execution record per execution window

Why:

- keeps long-lived business logic stable
- keeps execution replay protection bounded per window
- simplifies missed-execution handling
- improves auditability and event tracing

Recommended flow:

1. parent intent stores recurrence and policy
2. each eligible window derives one child execution id
3. that child may be:
   - executed once
   - skipped
   - cancelled
4. replay protection is tracked at child execution level

## 6. Execution Flow

### 6.1 Scheduled payment

1. user creates intent
2. account stores intent in registry
3. intent records next eligible execution ledger/time
4. relayer or user submits execution request with exact `intent_id` and `execution_nonce`
5. account checks current state, policy version, and execution window
6. registry marks execution in progress or consumes nonce atomically
7. transfer adapter executes payment
8. registry marks execution complete

### 6.2 Conditional payment

1. user creates conditional intent
2. approved attestor produces signed event proof
3. relayer submits proof and execution request
4. condition verifier validates proof, freshness, and uniqueness
5. policy engine validates constraints against current policy version
6. registry consumes attestation id atomically
7. adapter executes payment
8. registry marks execution complete

### 6.3 Revenue split

1. inbound balance reaches account
2. split rule becomes executable under explicit trigger conditions
3. authorized execution is requested with current account balance snapshot or bounded spend amount
4. split adapter calculates basis-point distribution
5. transfers are executed to approved destinations
6. execution record prevents duplicate settlement for the same trigger window

### 6.4 Treasury rebalance

1. rebalance rule becomes eligible
2. relayer submits rebalance request
3. policy engine checks exposure, slippage, oracle freshness, and caps
4. approved adapter executes transfer, swap, or deposit
5. post-execution state is recorded for future limit accounting

## 6.5 Missed Execution Policy

Each repeatable intent must specify what happens if execution is missed:

- `skip`
- `catch_up_once`
- `catch_up_all`, only for explicitly safe idempotent cases

Default v1 behavior should be `skip` for safety.

## 6.6 Atomicity Rules

The system must avoid partially-valid execution records.

Required rule:

- if execution fails before value movement completes, no terminal success record may be written

Recommended implementation:

- perform registry consumption and settlement inside one transaction
- use unique execution nonces so duplicate relayer submissions fail safely

## 6.7 Best Answer for Autonomous Execution

The correct v1 answer is:

- future execution must rely on stored onchain intent state plus a bounded automation capability created earlier by the owner
- relayers do not present fresh privileged authorization
- relayers only trigger execution of already-authorized bounded automations
- automation validation happens inside explicit `SmartAccount` execution logic, not through `__check_auth` unless a fresh signature is being checked

This preserves automation without making the relayer trusted.

## 7. Storage Layout

### 7.1 SmartAccount storage

- signer registry
- signer metadata
- session key records
- global spend windows
- allowlists
- adapter status
- pause/freeze flags
- recovery state

### 7.2 IntentRegistry storage

- intent objects
- execution counters
- last execution timestamps
- consumed attestation ids

### 7.3 PolicyEngine storage

- destination allowlists
- asset allowlists
- protocol allowlists
- threshold configuration
- per-tier limits

## 7.4 TTL and Rent Strategy

Soroban storage lifecycle must be treated as a first-class design concern.

Critical persistent entries:

- signer registry
- signer metadata
- session key state
- guardian and recovery state
- active intent records
- consumed attestation ids
- policy version and allowlists

Rules:

- critical security state must use persistent storage
- each state-changing call touching critical entries should extend TTL for those entries
- an explicit maintenance method should exist to bump TTL for:
  - signer state
  - recovery state
  - long-lived intents
  - attestation replay-protection records
- operators and relayers must not be solely responsible for TTL extension of security-critical records

Recommended v1 policy:

- owner/admin callable `extend_ttl()` maintenance methods
- automatic TTL extension on any successful auth, policy update, or execution
- monitoring to alert when critical entries approach low TTL

Risk if omitted:

- signer state can expire
- replay-protection state can expire
- recovery state can disappear
- long-lived scheduled intents can become invalid unexpectedly

## 7.5 Replay-Retention and Compaction Strategy

Replay-protection records must be bounded over time.

Recommended v1 strategy:

- use parent-intent recurrence plus child execution ids
- keep only active and recently settled child execution records onchain
- retain replay-protection data until:
  - the execution window is no longer disputable or replay-relevant
  - and the parent intent policy allows pruning

Suggested retention model:

- `scheduled_payment`
  - retain child execution records until the next scheduled window opens plus `RETENTION_BUFFER_LEDGERS`
- `conditional_payment`
  - retain consumed attestation ids until attestation expiry plus `RETENTION_BUFFER_LEDGERS`
- `revenue_split`
  - retain one record per split window plus `RETENTION_BUFFER_LEDGERS`
- `treasury_rebalance`
  - retain per-window execution ids and cumulative-limit accounting state until the next window plus `RETENTION_BUFFER_LEDGERS`

Compaction rules:

- terminal child records may be pruned only after deterministic retention threshold is met
- parent intent aggregate counters must preserve cumulative limits after child pruning
- consumed attestation ids may be compacted into rolling checkpoint state only if replay safety is preserved

Deterministic v1 rule:

- define `RETENTION_BUFFER_LEDGERS` as a protocol constant in implementation
- pruning eligibility must be computable entirely from onchain state
- pruning must not depend on operator discretion

Pruning authority:

- pruning may be permissionless once deterministic expiry conditions are met
- pruning must never modify parent cumulative counters except through explicitly defined compaction logic

Best v1 answer:

- prefer bounded rolling retention windows over indefinite storage
- do not rely on unbounded historical nonce storage

## 7.6 V1 Asset Model

Version `v1` should be `SAC-only`.

Rules:

- all treasury assets must be Stellar Asset Contracts
- classic/non-SAC asset support is out of scope for v1
- wallet integration and adapters must assume SAC transfer semantics only
- each supported SAC must be explicitly approved per account or global policy

This is the safest and most auditable first release model.

## 8. Adapter Rules

### 8.1 Transfer adapter

- only approved assets
- only approved recipients for lower auth tiers
- strict amount validation

### 8.2 Split adapter

- basis points must sum exactly to `10_000`
- all destinations must be approved
- maximum number of split recipients capped

### 8.3 Swap adapter

- single approved venue in v1
- route must be explicit
- asset pair allowlist required
- slippage cap enforced

### 8.4 Yield adapter

- only audited and approved strategy target
- capped percentage of treasury exposure
- deposit and unwind limits enforced

## 8.5 Adapter Authority Rules

Adapters must be pure execution helpers under account authority.

Rules:

- adapters may never maintain independent withdrawal authority over treasury funds
- adapters must reject calls unless invoked through the authorized account flow
- adapter upgrades or replacements must be controlled by `SmartAccount` policy
- adapter state must not become a hidden second source of truth for account permissions

## 9. Offchain Components

### 9.1 Frontend

- passkey registration and signing
- intent creation wizard
- human-readable policy review
- simulation and preview UX

### 9.2 Relayer

- simulates and submits transactions
- does not custody keys
- rate limits abusive traffic
- stores audit logs

### 9.3 Indexer

- tracks events
- powers intent status UI
- monitors anomalies and failures

### 9.4 AI Copilot

- drafts policies
- suggests treasury actions
- simulates outcomes
- never signs or bypasses policy
