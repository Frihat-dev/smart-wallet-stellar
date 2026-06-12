# Technical Specification for SCF Build Submission

## 1. Architecture Overview
Smart Treasury Account (STA) is a Soroban-native programmable treasury wallet. The account itself is a contract account that holds treasury assets, validates authorization through `__check_auth`, enforces onchain policy, and routes approved execution through narrow adapters.

STA is designed around explicit, deterministic policy rather than trusted offchain automation. AI or offchain services may help draft or submit operations, but they do not control assets.

## 2. Core Components
### SmartAccount
The SmartAccount contract is the root authority. It:

- holds treasury balances
- implements `__check_auth`
- stores signer and session-key state
- enforces role and threshold separation
- stores asset, adapter, and destination allowlists
- validates interactive actions
- validates stored automation capabilities
- coordinates pause, freeze, and recovery behavior
- routes execution to approved adapters

### PolicyEngine
The PolicyEngine validates account-level execution rules. V1 policy includes:

- policy version checks
- payment/adapters enabled flags
- asset risk-tier limits
- future expansion for destination, amount, timing, and cumulative outflow policies

### IntentRegistry
The IntentRegistry tracks treasury automation intent state. It is intended to store:

- parent intent metadata
- child execution records
- execution status
- replay-protection records
- cancellation and terminal state

### ConditionVerifier
The ConditionVerifier validates external condition proofs for conditional treasury flows. It:

- maintains approved attestors
- enforces delayed governance for attestor/threshold changes
- checks attestation expiry
- verifies Ed25519 attestor signatures
- prevents duplicate attestation consumption

### RecoveryManager
The RecoveryManager module represents recovery-specific functionality. Current recovery behavior is primarily enforced by SmartAccount, with future scope to separate recovery orchestration if needed.

### Execution Adapters
Adapters provide narrow execution surfaces:

- `transfer_adapter` for SAC transfers
- `split_adapter` for revenue or payout distribution
- `swap_adapter` for approved treasury routes
- `yield_adapter` for approved treasury management actions

Adapters require SmartAccount authorization and do not independently grant spending authority.

## 3. Account Model
Each STA is a Soroban contract account identified by a `C...` address.

Important constraints:

- the contract account authorizes operations through `__check_auth`
- it does not need to be the transaction source account
- a user wallet or relayer submits transactions and pays fees
- the relayer is not trusted for authority
- all treasury movement must remain bounded by onchain policy

## 4. Authorization Model
STA separates authorization into distinct planes:

- spend authority for payments and adapter actions
- management authority for signer/session/policy setup
- governance authority for high-impact configuration
- recovery authority for pause, freeze, and recovery flows

Signer records include:

- signer id
- signer kind
- role bitmap
- status
- weight
- creation ledger
- optional expiry ledger
- metadata hash

Current signer kinds include Ed25519, passkey/P256 placeholder, policy signer, session key, and guardian.

## 5. Interactive Execution
Interactive execution is used when a signer approves a specific immediate action.

Flow:

1. User or application prepares an action payload.
2. Wallet signs the Soroban auth payload.
3. SmartAccount validates signer status, role, weight, and context.
4. SmartAccount checks current policy version.
5. SmartAccount checks asset, adapter, destination, and amount rules.
6. SmartAccount preauthorizes the relevant token movement.
7. The approved adapter executes the action.

Supported interactive action types:

- payment
- split
- swap
- yield action

## 6. Stored Automation
Stored automation is used for scheduled, recurring, or conditional actions that should not require fresh user approval every time.

Automation capability records bind:

- capability id
- parent intent id
- exact action payload
- optional required attestation id
- policy version
- execution window
- max execution count

Execution rules:

- capability must exist and not be revoked
- child execution id must not already be consumed
- ledger must be inside the execution window
- policy version must match
- max execution count must not be exceeded
- required attestation must be consumed successfully when present
- action must pass account policy before adapter dispatch

## 7. Asset and Adapter Scope
V1 is SAC-only.

Rules:

- every supported asset must be explicitly configured
- every adapter must be explicitly configured and enabled
- adapters have type-specific limits
- destinations must be allowlisted for payment and split flows
- arbitrary contract execution is not part of v1

## 8. Condition Verification
Conditional execution uses attestation proofs. Each proof binds:

- smart account address
- condition verifier contract address
- attestation id
- capability id
- expiry ledger
- attestor signatures

The verifier checks:

- attestation has not expired
- attestation id has not been consumed
- attestors are approved
- attestor list has no duplicates
- valid signatures meet threshold

## 9. Recovery and Emergency Controls
STA includes:

- pause/unpause
- freeze
- delayed recovery initiation
- recovery cancellation
- delayed recovery finalization
- recovery signer rotation
- recovery-mode policy engine replacement
- recovery-mode adapter, asset, destination, and capability restrictions

When guardian signers are configured, recovery authority can be separated from normal primary signers.

## 10. Onchain and Offchain Boundaries
Onchain:

- signer state
- session state
- authorization checks
- policy enforcement
- asset and adapter allowlists
- automation capabilities
- replay records
- attestation consumption
- recovery state

Offchain:

- user interface
- passkey/wallet integration
- relayer submission
- attestation generation
- local simulation and eligibility display
- monitoring and operational alerts

## 11. Current Implementation Status
Implemented:

- Rust/Soroban workspace
- SmartAccount custom authorization
- signer roles and thresholds
- scoped session keys
- asset, adapter, and destination allowlists
- interactive execution through transfer, split, swap, and yield adapters
- stored automation capability execution
- attestation-gated execution via ConditionVerifier
- recovery freeze/finalization flow
- extensive SmartAccount and ConditionVerifier tests

Partially implemented:

- PolicyEngine basic policy checks
- adapters as narrow demo/testnet execution modules

Still to complete:

- full IntentRegistry lifecycle
- richer PolicyEngine rules
- production-grade adapter integrations
- relayer integration
- passkey-first frontend/onboarding flow
- testnet deployment package
- mainnet-readiness documentation

## 12. Test Strategy
Current and planned tests cover:

- initialization
- signer management
- weighted thresholds
- role separation
- session scopes
- policy rejection
- adapter dispatch
- stored automation replay protection
- attestation verification
- pause/freeze/recovery behavior

Future tranche tests should add:

- full parent intent lifecycle
- child execution state transitions
- recurring schedule windows
- pruning and TTL behavior
- testnet relayer execution
- wallet/passkey integration flows

## 13. Deployment Plan
1. Finalize SmartAccount MVP and deploy to Stellar testnet.
2. Deploy PolicyEngine, IntentRegistry, ConditionVerifier, and adapters.
3. Configure a sample Smart Treasury Account with approved SAC asset, destination, and adapter policies.
4. Execute testnet demos for payment, split, and conditional execution.
5. Add relayer submission flow and passkey-first onboarding.
6. Freeze interface and contract docs for audit review.
7. Prepare mainnet deployment checklist and operations guide.
