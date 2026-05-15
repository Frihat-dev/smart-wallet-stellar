# Project Overview

## 1. Objective

Build a production-grade programmable finance layer on Stellar using Soroban contract accounts.

The product gives users and businesses a smart treasury account that can:

- hold assets directly
- enforce policy-based authorization
- execute approved intents
- support passkey-based UX
- safely delegate limited actions to session keys or approved automations

## 2. Problem Statement

Current wallet UX on most chains is still centered around manual approval of individual actions. That model is weak for treasury, payroll, vendor payments, and recurring business operations.

Stellar already has the right primitives for a better model:

- contract accounts
- custom authorization
- passkey compatibility
- policy signers
- session keys
- relayer-based UX

The missing layer is a secure treasury product that turns those primitives into programmable finance workflows.

## 3. Product Thesis

The core idea is not "AI wallet."

The core idea is:

`a policy-controlled treasury account that executes financial intents under explicit onchain rules`

AI is optional and offchain. It can help users draft or simulate intents, but it never controls assets directly.

## 4. Target Users

Primary target users:

- startups and SMBs using stablecoin treasury flows
- payment operators and service businesses
- onchain teams managing payroll and vendor operations
- DAOs and protocol treasuries on Stellar

## 5. Supported V1 Use Cases

### 5.1 Scheduled payments

Examples:

- pay contractor every Friday
- send monthly tool subscription payment
- pay recurring treasury allocation

### 5.2 Conditional payments

Examples:

- pay supplier after signed delivery confirmation
- release funds after approved milestone completion

### 5.3 Revenue splitting

Examples:

- split revenue across treasury, operations, and tax wallets
- split incoming payment across partners by fixed basis points

### 5.4 Treasury rebalancing

Examples:

- maintain minimum stable balance
- sweep idle balance into approved yield adapter
- rebalance between approved stable positions within limits

## 6. Non-Goals for V1

- arbitrary contract execution
- unsandboxed autonomous AI agents
- cross-chain orchestration
- margin, leverage, or liquidation strategies
- exchange-facing flows requiring unsupported account behavior

## 7. Product Constraints

- must align with Soroban contract-account model
- must not assume `C...` accounts can be transaction source accounts
- must keep relayer trust minimized
- must not rely on unverifiable offchain triggers
- must keep all privileged actions behind strong auth tiers
- must keep v1 asset handling narrow and explicit

## 7.1 V1 Asset Support Policy

Version `v1` should be `SAC-only`.

This means:

- the treasury account supports Stellar Asset Contract representations only
- every supported asset must be explicitly allowlisted
- adapters must reject assets outside the approved SAC set

Why:

- narrows integration and transfer semantics
- simplifies wallet and contract behavior expectations
- reduces ambiguity during audit and testing

V1 should not attempt to support a mixed asset model.

## 8. High-Level Functional Requirements

- create and initialize a smart treasury account
- register and rotate signers
- create, scope, and revoke session keys
- create and manage intents
- execute valid intents through approved adapters
- pause or freeze account activity
- recover account access via guardian path
- emit events for every critical action

## 9. Success Criteria

The system is successful if it achieves:

- secure passkey-first user onboarding
- reliable execution of deterministic financial intents
- strict enforcement of account policy
- clear separation between user auth, policy auth, and recovery auth
- audit-ready contract boundaries
