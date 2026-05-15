# Security Specification

## 1. Security Goals

- prevent unauthorized asset movement
- prevent replay across intents, sessions, or environments
- constrain automation to explicit scopes
- preserve recovery path after signer compromise
- minimize damage from relayer or attestor failure

## 2. Security Model

The system is built on four control layers:

- signer authentication
- policy enforcement
- execution surface restriction
- operational safeguards

## 3. Trust Assumptions

Trusted within defined scope:

- SmartAccount contract code
- PolicyEngine contract code
- approved adapter code
- approved guardian quorum

Not trusted:

- relayer
- frontend
- AI service
- single attestor for high-risk flows unless explicitly configured

## 4. Threat Model

### 4.1 Account takeover

Threats:

- passkey phishing
- compromised admin key
- malicious session-key abuse

Controls:

- session keys scoped by function, asset, amount, and expiry
- high-risk actions require stronger auth tier
- guardian-based freeze and recovery

### 4.2 Replay

Threats:

- replay of signed action
- replay of offchain condition proof

Controls:

- auth-entry replay resistance
- unique attestation ids
- expiry windows
- execution counters and consumed-proof tracking

### 4.3 Malicious relayer

Threats:

- reordered requests
- spam submission
- failure to submit

Controls:

- relayer has no signing authority
- signed payload binds exact scope
- users or backup relayers can resubmit equivalent requests

### 4.4 Nested call bypass

Threats:

- hiding dangerous subcalls inside approved root call

Controls:

- inspect auth context and subinvocations
- adapter surface must stay narrow
- no generic arbitrary router entrypoint

### 4.5 Protocol integration loss

Threats:

- venue exploit
- pool depeg
- oracle manipulation

Controls:

- venue allowlist
- protocol exposure caps
- slippage bounds
- emergency adapter pause

## 5. Authorization Requirements

`__check_auth` must validate:

- signer identity
- signer class
- function scope
- amount scope
- asset scope
- destination scope
- time validity
- cosigner requirement when applicable
- current policy version
- execution nonce or equivalent anti-replay material

## 5.1 Authorization Payload Requirements

Every signed authorization payload must bind at minimum:

- network domain
- `SmartAccount` contract id
- signer id
- signer type
- action type
- adapter id, if applicable
- intent id, if applicable
- execution nonce
- expiry
- action-specific scope fields

Action-specific scope fields should include only what is necessary, for example:

- asset
- amount or amount cap
- destination
- schedule identifier
- attestation id

## 5.1.1 Interactive vs Stored Authorization

The spec must treat two forms of authorization separately:

- `interactive auth`
- `stored automation capability`

Interactive auth:

- signer approves a specific immediate action
- payload binds exact execution details

Stored automation capability:

- signer approves a bounded future execution envelope
- later relayer-triggered executions are valid only if they fit inside that stored envelope

Security rule:

- relayer-triggered execution must never succeed solely because a relayer asked for it
- it must succeed only because a prior owner-approved capability exists and remains valid

Canonical v1 rule:

- `interactive auth` is verified through `__check_auth`
- `stored automation` is verified through explicit `SmartAccount` execution logic against pinned capability state
- relayer-triggered automation must not be routed through a signature-verification path unless a fresh user signature is actually required

## 5.2 Passkey Verification Requirements

If passkeys are used, the implementation spec must define:

- canonical challenge encoding
- rp/origin expectations in the client layer
- signed payload normalization before onchain verification
- credential binding to account state
- how passkey rotation and revocation are represented

The smart-account spec must avoid ambiguous client-side serialization that could produce mismatched signed intent displays and onchain meaning.

## 5.3 Session Key Verification Requirements

Session-key auth must validate:

- key status is active
- TTL or expiry has not elapsed
- action is inside scope
- cumulative spend does not exceed cap
- destination is allowed
- adapter is allowed
- session has not been revoked or consumed

Authorization must be domain-separated by:

- network
- contract
- action type
- intent id or session id

## 5.4 Automation Capability Verification Requirements

For stored automations, explicit account execution logic must validate:

- parent intent is active
- policy version matches the creation-time pinned version
- execution window is currently valid
- child execution id has not already been consumed
- requested adapter matches authorized adapter
- requested asset and destination are inside stored bounds
- requested amount does not exceed per-execution cap
- cumulative usage does not exceed total cap
- automation has not been revoked, expired, or frozen

## 6. Session Key Security

Session keys are powerful and must be constrained aggressively.

Required controls:

- short expiry
- single-purpose or narrow-purpose scope
- per-session spend cap
- asset allowlist
- destination allowlist
- explicit revocation
- automatic invalidation on account freeze

## 7. Recovery and Freeze

Required recovery features:

- emergency freeze path
- guardian quorum
- delayed recovery finalization
- signer replacement flow
- audit trail events for all recovery actions

Recommended policy:

- recovery cannot instantly move funds
- recovery only changes auth state after delay
- unfreeze after recovery requires explicit guardian quorum

## 7.1 Recovery Failure Modes

The implementation spec must address:

- guardian loss
- guardian collusion
- owner loss of passkey
- simultaneous compromise of session and admin signers

Required policy decision:

- version `v1` chooses safety over liveness when guardians are unavailable

V1 recovery stance:

- if guardians are unavailable, funds may remain frozen rather than allowing a weak escape hatch
- any owner-escape or fallback recovery path, if ever introduced, must be a later explicitly designed feature with stronger review

## 8. Adapter Security Rules

All adapters must:

- reject unknown contracts
- reject unsupported assets
- reject malformed routes
- emit structured events
- use checked arithmetic
- avoid callback complexity where possible

## 9. External Condition Verification

Conditional automation must never trust raw API responses.

Required controls:

- signed attestations from approved keys
- event timestamp freshness
- unique event id
- binding to intent id and account
- threshold quorum for high-value actions

Additional requirements:

- attestation domain separation by verifier and network
- explicit allowed attestor set versioning
- retention strategy for consumed event ids so replay protection does not disappear due to TTL expiry

## 9.1 Best Answer for Attestation Retention

Consumed attestation ids should be retained in a bounded rolling window tied to:

- attestation expiry
- replay relevance window
- parent intent lifecycle

Unbounded retention is not recommended for v1.

Deterministic v1 rule:

- retention thresholds must be expressed in protocol constants, not operator judgment
- pruning may be permissionless after threshold maturity

## 10. Upgrade and Governance Policy

Best-practice preference for v1:

- avoid upgradeability if feasible

If upgradeability is necessary:

- use delayed upgrade execution
- require strongest auth tier
- require pause-before-upgrade for critical modules
- publish migration plan and rollback plan

## 11. Testing Requirements

Pre-testnet:

- unit tests for every auth tier
- negative tests for every forbidden path
- serialization and signature parsing tests
- session key expiry and revocation tests
- attestation replay tests
- TTL extension and expiration tests
- policy version mismatch tests
- duplicate relayer submission tests
- recovery edge-case tests

Pre-mainnet:

- fuzz tests for auth payloads and policy boundaries
- invariant tests for spend limits
- integration tests with relayer flow
- external audit
- long-horizon tests for recurring intents and replay-protection retention
- automation-capability abuse tests
- relayer-triggered execution tests without fresh user signatures

## 12. Monitoring and Incident Response

Monitoring must include:

- failed authorization spikes
- repeated session-key denials
- unusual outflow patterns
- adapter error rate
- freeze and recovery events

Incident response runbooks must define:

- who can freeze
- when to pause adapter vs full account
- how to rotate attestor keys
- how to communicate incidents to users
