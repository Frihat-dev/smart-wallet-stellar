# Reference Analysis: Managed Accounts PDF

## 1. Purpose

This document reviews [ManagedAccounts(1).PDF](/home/mohamed/stellar-programmable-finance/ManagedAccounts(1).PDF) as a reference input for the Stellar programmable finance project.

The goal is to separate:

- ideas that are architecturally useful
- ideas that do not fit Stellar
- ideas that should be translated into Soroban-native patterns

## 2. High-Level Assessment

The PDF is useful as a **delegated execution and security architecture reference**.

It is not suitable as a direct implementation blueprint because it is built for:

- Safe
- Zodiac modules
- ERC-4337
- EVM calldata and selector permissions

Our project targets:

- Stellar
- Soroban contract accounts
- `__check_auth`
- relayer-submitted smart-account flows
- Stellar-native wallet and treasury UX

## 3. What the PDF Gets Right

### 3.1 Delegated execution with preserved custody

This is one of the strongest ideas in the reference.

The document aims for a middle ground between:

- giving full custody to a manager
- forcing the owner to manually execute everything

That maps extremely well to our Stellar project.

For our system, the equivalent model is:

- the smart account holds funds
- the owner defines policy
- operators or automations execute only within explicit scope

### 3.2 Granular permissioning

The reference emphasizes:

- target restrictions
- function restrictions
- parameter conditions

This is conceptually excellent.

We should preserve the same security philosophy on Stellar, but express it through:

- adapter allowlists
- action type restrictions
- destination constraints
- asset constraints
- amount caps
- time windows

### 3.3 Delay and cancellation safety

The delay module pattern is highly valuable.

For sensitive treasury actions, a delay window gives:

- time to detect bad automation
- time to cancel malicious or mistaken actions
- better enterprise and institutional trust

This should be translated into our system as:

- execution windows
- delayed high-risk intents
- guardian or owner cancellation before settlement

### 3.4 Security invariants

The PDF does a good job describing core invariants such as:

- only owners can move value outside approved paths
- delegated actors stay inside explicit permissions
- queued transactions can be cancelled before execution

That style is very useful for our specification and audit preparation.

### 3.5 Session keys and policy engine direction

The document lists session keys and policy engines as future extensions.

Those are directly aligned with our roadmap, and in our case they should be first-class design components rather than optional future ideas.

## 4. What Does Not Fit Stellar

### 4.1 Safe architecture

The PDF assumes a Safe proxy and module chain.

That is not the correct model for Stellar.

On Soroban, the treasury account should be a contract account that enforces auth through `__check_auth`, not a Safe-compatible module stack.

### 4.2 ERC-4337 terminology and flow

The PDF depends on:

- EntryPoint
- paymasters
- bundlers
- user operations

These are Ethereum-native concepts.

They should not be imported into the Stellar specification.

On Stellar, the correct model is:

- contract account authorization
- auth entries
- relayer or `G...` source account submission

### 4.3 EOA-first assumptions

The reference assumes owners are EOAs and operators are EVM addresses.

In our project, signer classes are broader and should include:

- passkeys
- admin keys
- guardians
- policy signers
- session keys

### 4.4 Raw selector permissioning as the primary security model

The PDF relies heavily on:

- target contract address
- 4-byte function selector
- calldata parameter constraints

That works naturally in EVM, but it is not the ideal primary abstraction for Soroban.

For Stellar, the safer and more maintainable model is:

- explicit adapter contracts
- typed intent categories
- structured policy constraints

## 5. Translation Map: EVM Reference to Stellar Design

### 5.1 Safe

Reference idea:

- Safe holds funds and executes calls

Stellar translation:

- `SmartAccount` contract account holds funds and enforces auth

### 5.2 Roles module

Reference idea:

- operators receive scoped permissions through roles

Stellar translation:

- signer classes plus `PolicyEngine` rules
- optional operator records for delegated execution

### 5.3 Delay module

Reference idea:

- time-lock queued operator actions

Stellar translation:

- delayed intent execution
- execution windows
- owner/guardian cancellation path

### 5.4 Operator executor

Reference idea:

- generic `execute()` and `multicall()`

Stellar translation:

- strongly scoped adapter entrypoints
- limited, typed execution methods
- optional batched intent execution only where explicitly safe

### 5.5 ERC-4337 gasless flow

Reference idea:

- paymaster and bundler submit account-abstraction operations

Stellar translation:

- relayer submits transaction
- smart account authorizes with auth entries
- fee sponsorship or relayer-paid UX handled outside ERC-4337 semantics

## 6. What We Should Keep

We should keep these ideas:

- delegated execution without surrendering custody
- strong security invariants
- layered permissions
- cooldown and cancellation for risky actions
- operator/session-key limits
- comprehensive testing categories

## 7. What We Should Discard

We should discard these as implementation assumptions:

- Safe proxy architecture
- Zodiac-specific modules
- EOA-centric model
- ERC-4337 framing
- paymaster and bundler terminology
- generic arbitrary call execution as the default interface

## 8. Recommended Adaptation for Our Project

Instead of copying the PDF architecture, we should adapt its strongest principle:

`delegated execution under constrained policy`

Recommended Soroban-native chain:

- signer or passkey authorization
- `PolicyEngine` validation
- `IntentRegistry` and optional delay checks
- `SmartAccount` execution authorization
- adapter-level action enforcement

This is the correct equivalent of the reference model in Stellar terms.

## 9. Impact on Our Repository Spec

The PDF should influence:

- security philosophy
- role separation
- operator model
- execution-delay design
- testing plan

The PDF should not determine:

- contract topology
- auth implementation details
- wallet integration model
- relayer architecture

## 10. Final Verdict

This PDF is a good reference for **how to think about delegated DeFi and treasury control securely**.

It is not a good reference for **how to implement smart accounts on Stellar directly**.

Best use:

- inspiration for invariants, permission boundaries, and control flows

Best avoided:

- direct reuse of EVM modules, execution interfaces, and account abstraction assumptions
