# Stellar Programmable Finance

Soroban-native smart accounts for programmable treasury operations, policy-driven automation, and secure intent execution on Stellar.

## Vision

This project defines and implements programmable treasury accounts on Stellar using Soroban contract accounts. The account itself is a smart contract that holds balances, enforces authorization through `__check_auth`, and executes tightly scoped financial intents such as:

- scheduled supplier payouts
- conditional business payments
- revenue splitting across wallets
- treasury rebalancing between approved assets and venues

The system is designed around deterministic onchain controls, passkey-first UX, and best-practice Web3 security.

## Principles

- Stellar-native architecture, not a direct Ethereum AA port
- deterministic onchain enforcement
- AI only as an advisory layer
- default-deny execution model
- narrowly scoped adapters instead of arbitrary call execution
- strong signer separation for daily use, policy approval, and recovery

## Documentation

- [Project Overview](./docs/00-overview.md)
- [System Architecture](./docs/01-architecture.md)
- [Security Specification](./docs/02-security.md)
- [Roadmap and Milestones](./docs/03-roadmap.md)
- [Wallet Integration and User Flows](./docs/04-wallet-integration.md)
- [Reference Analysis: Managed Accounts PDF](./docs/05-reference-analysis-managed-accounts.md)
- [Authorization and Automation Model](./docs/06-authorization-and-automation-model.md)
- [Contract Specification](./docs/07-contract-specification.md)
- [Payloads, Events, and Errors](./docs/08-payloads-events-errors.md)
- [State Machines and Storage Keys](./docs/09-state-machines-and-storage-keys.md)
- [Rust Layout and Test Matrix](./docs/10-rust-layout-and-test-matrix.md)
- [Technical Architecture](./docs/TECHNICAL_ARCHITECTURE.md)
- [SCF Grant Application](./docs/GRANT_APPLICATION.md)
- [SCF Form Responses](./docs/GRANT_FORM_RESPONSES.md)
- [SCF Milestones and Budget](./docs/GRANT_MILESTONES_BUDGET.md)
- [SCF Technical Specification](./docs/GRANT_TECHNICAL_SPEC.md)
- [SCF Submission Checklist](./docs/SCF_SUBMISSION_CHECKLIST.md)

## Initial Scope

Version `v1` focuses on:

- contract-account treasury wallet
- passkey and admin signer support
- session keys with strict scope
- policy-enforced intent execution
- relayed submission flow
- payment, split, and rebalance primitives

## Implementation Direction

The first implementation phase will target:

- Soroban smart contracts in Rust
- contract-account authorization via `__check_auth`
- offchain relayer integration for transaction submission
- browser UX with passkey support

## Current Scaffold

The repository now includes the first Rust workspace scaffold for:

- `SmartAccount`
- `IntentRegistry`
- `PolicyEngine`
- `ConditionVerifier`
- `RecoveryManager`
- `TransferAdapter`
- `SplitAdapter`
- `SwapAdapter`
- `YieldAdapter`
- shared crates for `types`, `events`, `errors`, `auth`, and `testutils`

The current code intentionally keeps privileged execution paths fail-closed until the full auth and policy logic is implemented.
