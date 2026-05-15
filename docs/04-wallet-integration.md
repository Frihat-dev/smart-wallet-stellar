# Wallet Integration and User Flows

## 1. Purpose

This document explains:

- how end users interact with a Soroban smart account
- how the smart-account layer integrates with existing Stellar wallets
- what technical constraints apply to wallet interoperability on Stellar

## 2. Key Concept

The smart account is not just a UI wrapper around a normal wallet.

It is a Soroban contract account that enforces its own authorization logic through `__check_auth`. This means:

- the smart account holds treasury logic and policy
- the user wallet acts as a signer or control surface
- a relayer or external `G...` account submits transactions

This separation is the right mental model for Stellar.

## 3. User Experience Model

### 3.1 What the user sees

From the user perspective, the product should feel like a wallet with advanced controls:

- create treasury account
- connect passkey or wallet signer
- approve vendors and destinations
- set payment and treasury rules
- create automations
- review and sign sensitive actions
- freeze or recover account if compromised

### 3.2 What the chain sees

Onchain, the actual control path is:

- a contract account authorizes actions
- user signers provide auth material
- relayer submits the transaction
- adapters execute limited treasury actions

## 4. Primary User Flows

### 4.1 Account creation flow

1. User opens app
2. User creates smart account
3. App initializes `SmartAccount`
4. User registers:
   - passkey signer
   - backup admin key
   - optional guardian set
5. App creates default policy profile

Output:

- active smart treasury account
- initial signer set
- initial policy state

### 4.2 Daily payment flow

1. User chooses approved vendor
2. User enters amount and asset
3. App checks local eligibility against account policy
4. App constructs human-readable auth summary and canonical signing payload
5. User signs auth request with passkey or session key
6. Relayer submits transaction
7. Smart account validates scope, nonce, expiry, and current policy version
8. Smart account executes transfer

### 4.3 Scheduled payment setup flow

1. User creates a scheduled payment intent
2. App stores schedule and destination constraints onchain
3. User signs creation request
4. Relayer submits transaction
5. Intent remains pending until execution window
6. Relayer later triggers execution with exact `intent_id` and execution nonce
7. Smart account verifies policy version, state, and adapter rules before transfer
8. Execution record is consumed atomically to prevent duplicates

Important UX implication:

- the user is approving creation of a bounded future automation, not blindly authorizing arbitrary relayer actions

### 4.4 Conditional payment flow

1. User creates conditional payment intent
2. Condition is tied to approved attestor or oracle
3. Attestor produces signed proof event
4. Relayer submits proof and execution request
5. Condition verifier validates proof, attestor version, expiry, and uniqueness
6. Smart account executes if all rules pass

Important UX implication:

- the user pre-authorizes a constrained future action subject to exact attestation rules

### 4.5 Revenue split flow

1. User configures split recipients and basis points
2. User signs split policy creation
3. Inbound funds accumulate
4. Relayer or user triggers split execution for a specific execution window
5. Split adapter routes funds to approved destinations
6. The same window cannot be settled twice

### 4.6 Recovery flow

1. User or guardian detects compromise
2. Guardian quorum triggers freeze
3. Recovery flow begins
4. New signer set is proposed
5. Delay period elapses if configured
6. Recovery is finalized
7. Account resumes with new signer state

## 5. Wallet Integration Model

There are three main ways to integrate the smart-account layer with existing Stellar wallets.

### 5.1 Existing wallet as signer

This is the simplest path.

The wallet is used to:

- approve smart-account actions
- sign auth entries
- confirm admin-level changes

The smart account remains the treasury controller.

This model works well for:

- browser wallets
- embedded wallets
- enterprise signing tools

### 5.2 Existing wallet as onboarding shell

This is the most practical ecosystem integration model.

The wallet or wallet-connected dapp provides:

- user identity
- account discovery
- signer setup
- passkey registration
- transaction review UI

The smart-account layer provides:

- policy enforcement
- treasury automation
- intent execution
- recovery model

### 5.3 Existing wallet as full controller

This is possible only if the wallet fully supports:

- Soroban contract-account UX
- auth-entry signing
- relayed or sponsored transaction flows
- human-readable simulation and display of smart-account actions

This is the strongest integration but also the most demanding.

## 6. Compatibility with Existing Stellar Wallets

### 6.1 What is possible

Existing wallets can generally integrate if they can support:

- transaction approval UX
- auth-entry or invocation signing
- Soroban contract interactions
- relayer-compatible user flow
- human-readable rendering of signed action scope
- clear display of whether the user is granting immediate execution or future bounded automation rights

They can be used as:

- admin signer
- backup signer
- treasury operator signer
- signer-management interface

### 6.2 What is not a perfect fit

A smart account is not a drop-in replacement for every classic Stellar account use case.

Limitations include:

- the smart account is a `C...` contract account, not a normal `G...` source account
- contract accounts do not submit transactions directly
- some legacy systems expect classic account behavior
- some exchange or memo-based workflows may not fit smart-account UX cleanly

### 6.3 Best integration target

The best integration targets are:

- Soroban-native wallets
- passkey-enabled apps
- treasury dashboards
- enterprise payment tools
- dapps that already support contract invocations

## 7. Relayer Interaction Model

The relayer is part of the UX path, but it must not be a trust assumption for fund safety.

### 7.1 Relayer responsibilities

- simulate action before submission
- build transaction envelope
- submit network transaction
- pay fees if using gasless or sponsored UX
- return result and status to client

### 7.2 Relayer trust boundaries

The relayer must never:

- hold user private keys
- change action payload after signature
- have authority to bypass account policy
- become the only party capable of extending TTL for critical account state

### 7.3 Backup path

The system should support multiple relayers or fallback submission paths so user funds are not dependent on a single operator.

## 7.4 Best Answer for Wallet UX

Wallets and frontends should clearly separate:

- `Sign now`
- `Authorize future automation`

The second case must show:

- action category
- approved assets
- approved destinations
- per-execution cap
- recurrence or trigger rule
- expiry
- cancellation path

## 8. Passkey Integration

Passkeys are one of the strongest UX opportunities for this project.

Recommended model:

- use passkey as primary day-to-day signer
- use admin key or guardian path as recovery layer
- require stronger auth for sensitive config changes

Good passkey use cases:

- approve payment intent
- approve vendor setup
- approve treasury rebalance under user-defined limits

Avoid using passkey alone for:

- full recovery override
- emergency governance without backup mechanism
- high-value config migration without secondary approval

## 9. Session Key Integration

Session keys enable a smoother UX for repeated low-risk actions.

Recommended use:

- short-lived operator session
- low-value recurring execution
- approved assets only
- approved destinations only
- explicit per-session limits

Not recommended:

- open-ended treasury movement
- unrestricted DeFi access
- long-lived high-value authority

## 10. Recommended Product Integration Strategy

For this project, the best product strategy is:

- do not try to replace all Stellar wallets
- build a programmable account layer
- expose SDKs and clear wallet integration points
- offer a reference frontend for treasury users

This gives:

- better ecosystem adoption
- lower integration friction
- stronger security boundaries
- easier grant positioning

## 11. Recommended Technical Deliverables

To make wallet integration practical, the project should eventually include:

- wallet integration SDK
- relayer client library
- auth request schema
- signer capability matrix
- reference passkey integration
- reference browser wallet integration
- test harness for smart-account signing flows
- canonical signing-payload specification

## 12. Open Integration Questions

These should be resolved before full implementation:

- which existing Stellar wallets can support auth-entry signing cleanly
- whether passkey is native in the chosen frontend stack or brokered through wallet middleware
- how to represent human-readable smart-account action previews
- whether fee sponsorship or relayer-paid UX is default or optional

## 12.1 Best Answers Adopted for V1

- v1 asset support should be `SAC-only`
- offline automation should rely on stored bounded capabilities created at intent setup time
- recurring automations should use parent intents plus child execution records per window
- replay-protection retention should use bounded rolling windows, not indefinite onchain history
- existing automations should remain pinned to the policy version active at creation
- fresh signatures should use `__check_auth`, while stored automations should execute via explicit account logic

## 13. Summary

Using a smart account on Stellar is practical and ecosystem-compatible if the system is designed with the right model:

- wallet as signer and UX surface
- smart account as treasury logic and policy engine
- relayer as transaction submitter

The correct goal is not to replace every wallet.

The correct goal is to add a secure programmable finance layer that existing wallets and Stellar apps can plug into.
