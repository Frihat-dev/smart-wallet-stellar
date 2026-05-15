# Authorization and Automation Model

## 1. Purpose

This document defines the exact conceptual model for:

- interactive authorization
- bounded future automation
- recurring execution rights
- replay protection
- retention and compaction

It resolves the core smart-account question:

how can the system support offline automation without trusting the relayer

## 2. Core Answer

The relayer is not an authority source.

The relayer only triggers execution of authority that was granted earlier by the owner in bounded form.

This means the system has two authorization modes.

Canonical v1 implementation rule:

- `__check_auth` is for fresh signature-based authorization
- stored automation is enforced by explicit account logic reading pinned onchain capability state

## 3. Mode A: Interactive Authorization

Used for:

- immediate transfers
- signer changes
- policy updates
- emergency actions

Properties:

- signer approves a single concrete action
- payload includes exact scope
- expires quickly
- cannot be reused for future scheduled execution

## 4. Mode B: Stored Automation Capability

Used for:

- scheduled payments
- conditional payments
- recurring revenue splits
- recurring treasury rebalances

Properties:

- signer approves a bounded future execution envelope
- execution may happen later while signer is offline
- relayer cannot expand scope
- onchain state is the authority source after setup
- policy version is pinned at creation

## 5. Capability Envelope

Each automation capability should include:

- account id
- parent intent id
- policy version
- allowed action type
- allowed adapter id
- allowed asset set
- allowed destination set
- per-execution amount cap
- cumulative cap
- execution start
- execution end
- recurrence or trigger definition
- remaining execution count or recurrence bound
- revocation state

## 6. Recurring Execution Model

Best v1 answer:

- one persistent parent intent
- one child execution record per eligible window

Child execution id derivation should bind:

- parent intent id
- window index or recurrence index
- policy version
- network domain

Each child execution record may end in:

- `executed`
- `skipped`
- `cancelled`
- `failed_terminal`

## 7. Replay Model

Replay protection is split by mode.

### 7.1 Interactive auth replay protection

- execution nonce
- short expiry
- exact action binding

### 7.2 Automation replay protection

- child execution id uniqueness
- consumed attestation ids where applicable
- per-window settlement records
- cumulative usage accounting

## 8. Retention Model

Replay state must not grow without bound.

Best v1 answer:

- use rolling bounded retention windows
- retain records only as long as replay risk remains relevant
- preserve aggregate counters after pruning detailed records

Deterministic v1 pruning rule:

- retention thresholds must be expressed in fixed protocol constants
- pruning eligibility must be derivable from onchain ledger/time state
- pruning may be permissionless after threshold maturity

Examples:

- keep scheduled child execution records until next execution window plus safety buffer
- keep consumed attestation ids until attestation expiry plus replay buffer
- keep cumulative amount accounting at parent intent level even when child records are pruned

## 9. Cancellation and Freeze

At any time before execution, the owner or authorized recovery path may:

- cancel parent intent
- cancel future child executions
- freeze the account
- revoke automation capability

Freeze must invalidate:

- session-key execution
- pending automation execution
- relayer-triggered execution

## 9.1 Recovery Stance

Version `v1` chooses safety over liveness.

If guardians are unavailable:

- the system must not fall back to a weak recovery bypass
- assets may remain frozen until the defined strong recovery path is satisfied

## 10. Auditor Guidance

When implementing this model, the most critical checks are:

- relayer-triggered execution never creates authority
- child execution ids are unique and consumed atomically
- parent intent policy cannot be silently changed midstream
- pruned replay records do not reopen old execution windows
- cumulative caps remain accurate after compaction

## 11. V1 Final Decisions

Adopted v1 answers:

- automation uses stored bounded capabilities
- v1 asset support is `SAC-only`
- recurring actions use parent intents plus child execution records
- replay retention uses bounded rolling windows
- missed recurring executions default to `skip`
- automation semantics are pinned to creation-time policy version
- fresh signatures use `__check_auth`; stored automations use explicit account logic
- recovery chooses safety over liveness
