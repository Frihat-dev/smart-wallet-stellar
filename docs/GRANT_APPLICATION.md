# Stellar Community Fund Grant Application

## Project Title
Smart Treasury Account (STA)

## One-Line Description
A Soroban-native programmable treasury wallet for Stellar that lets organizations automate payments, revenue distribution, and treasury operations under explicit onchain policies.

## Problem Statement
Business treasury operations are still largely handled through manual wallet approvals, spreadsheets, and bespoke back-office processes. For companies, payment providers, marketplaces, DAOs, and tokenized-asset platforms, this creates operational risk and makes recurring payroll, vendor payments, revenue sharing, and treasury allocation hard to automate safely.

Existing wallet flows are not designed for treasury teams that need:

- clear signer permissions and approval thresholds
- approved asset and recipient lists
- spending limits and timing rules
- recurring or conditional execution
- recovery controls for compromised signers
- auditability across every treasury action

Stellar is already a strong fit for payments, stablecoins, tokenization, and low-cost financial workflows. The missing layer is a Soroban-native treasury account that turns those primitives into reusable, policy-controlled operations.

## Solution
Smart Treasury Account (STA) is a programmable treasury wallet built with Soroban contract accounts. Each STA acts as a contract-controlled account that holds Stellar Asset Contract (SAC) assets, validates authorization through `__check_auth`, and routes approved operations through narrow execution adapters.

Users configure onchain policy for:

- authorized signers and signer roles
- supported SAC assets
- approved recipients and venues
- amount caps and adapter limits
- scoped session keys
- stored automation capabilities
- recovery, pause, and freeze controls

Treasury operations can then execute under these predefined rules without giving relayers, operators, or offchain automation services authority over funds. Relayers can submit transactions, but the contract account remains the root authority and rejects anything outside the stored policy envelope.

## Why Stellar
STA is designed specifically for Stellar rather than as a generic account-abstraction port.

Stellar provides the right primitives for this product:

- Soroban contract accounts and custom authorization
- Stellar Asset Contracts for stablecoin and tokenized-asset workflows
- low-cost, high-frequency payment rails
- strong fit for treasury, payout, marketplace, and tokenization use cases
- passkey-friendly user experience potential
- growing stablecoin and real-world asset adoption

STA can help Stellar serve organizations that need programmable treasury workflows without requiring each team to build custom smart-account infrastructure from scratch.

## Current Progress
The current repository already includes a working Rust/Soroban workspace for the core contract suite:

- `contracts/smart_account`
- `contracts/intent_registry`
- `contracts/policy_engine`
- `contracts/condition_verifier`
- `contracts/recovery_manager`
- `contracts/adapters/transfer_adapter`
- `contracts/adapters/split_adapter`
- `contracts/adapters/swap_adapter`
- `contracts/adapters/yield_adapter`
- shared crates for `types`, `events`, `errors`, `auth`, and `testutils`

Implemented and tested capabilities currently include:

- `SmartAccount` initialization
- custom account authorization via `__check_auth`
- signer roles and weighted approval thresholds
- management, governance, spend, and recovery planes
- scoped session keys
- asset, adapter, and destination allowlists
- interactive payment, split, swap, and yield adapter dispatch
- stored automation capabilities
- attestation-gated conditional execution
- replay protection for child execution IDs
- pause, freeze, and delayed recovery flows
- condition verifier governance and attestor quorum validation

The workspace test suite currently passes with coverage across SmartAccount and ConditionVerifier behavior.

## Market and Business Case
STA targets organizations that need reliable, auditable, policy-bound treasury workflows:

- businesses managing stablecoin operations
- payment providers and marketplaces
- tokenization platforms
- DAOs and protocol treasuries
- payroll and vendor payment operators
- organizations managing recurring revenue distribution

The project benefits from business traction and market access through Reto Grau Consulting and Equisafe.

Reto Grau brings more than 30 years of financial markets, asset management, and treasury operations experience, including roles at Man Group, JPMorgan, Swiss Life, and ABB Treasury.

Equisafe is a European tokenization platform that has facilitated more than EUR 400M in investments, supported 25,000+ private investors, and enabled 70+ fundraising rounds. This provides a relevant path to real-world treasury, tokenization, and payment workflows that can benefit from Stellar-native smart accounts.

Within 12 months of deployment, STA targets:

- 50+ onboarded organizations
- 500+ deployed Smart Treasury Accounts
- 250,000+ treasury operations on Stellar
- more than USD 100M in treasury assets managed through STA policies

## Team
### Reto Grau - Founder and CEO
Reto Grau brings more than 30 years of experience in asset management, treasury operations, alternative investments, and institutional finance. His career includes senior roles at RMF Investment Management, Man Group, Swiss Life Hedge Fund Partners, JPMorgan, and ABB Treasury. He provides strategic leadership, industry expertise, and institutional network access relevant to treasury management, payments, and digital assets.

LinkedIn: https://www.linkedin.com/in/retograu/

### Alexandre Karako - Business Development
Alexandre Karako brings experience in digital assets, fundraising, tokenization, and business development. Through his work with Equisafe, he has contributed to tokenized investment products, issuer onboarding, investor relations, and ecosystem growth. His expertise supports partnerships, adoption, and real-world treasury and tokenization use cases.

LinkedIn: https://www.linkedin.com/in/alexandrekarako/

### Clement Roure - Technical Lead
Clement Roure is an experienced software architect and developer. Through his experience at Keyrock and his work on digital platforms serving more than 400,000 users, he brings expertise in backend systems, scalability, reliability, and financial technology infrastructure.

LinkedIn: https://www.linkedin.com/in/clementroure/

### Maxime Sarthet - Strategic Advisor
Maxime Sarthet is CEO of Equisafe, a European tokenization platform. Under his leadership, Equisafe has facilitated EUR 400M+ in investments, supported 25,000+ investors, and enabled 70+ fundraising rounds. He provides strategic guidance and adoption channels across the tokenization ecosystem.

LinkedIn: https://www.linkedin.com/in/maxime-sarthet/

## Budget Request
Requested amount: **150,000 XLM**

The request supports future development toward a testnet-demo-ready and mainnet-ready Smart Treasury Account product. It excludes audit costs, legal costs, marketing spend, and reimbursement for past work.

## Tranche Plan
### Acceptance - 10%
Deliverables:

- public project work plan and acceptance criteria
- finalized repository baseline and grant documentation
- SmartAccount initialization and authorization model documented
- test plan aligned to tranche deliverables

### Tranche 1 - 20% - SmartAccount MVP
Deliverables:

- SmartAccount contract hardened for core treasury use
- passkey/admin signer flow specification
- signer roles and threshold management
- pause/freeze controls
- scoped session-key creation and enforcement

Verification:

- passing unit and integration tests for authorization flows
- documented initialization and signer-management flow
- testnet deployment plan for SmartAccount MVP

### Tranche 2 - 30% - Intent Engine and Policy
Deliverables:

- IntentRegistry implementation for parent intents and child execution state
- expanded PolicyEngine rules for assets, destinations, spending caps, and policy version pinning
- scheduled payment and revenue split primitives
- conditional execution path connected to ConditionVerifier

Verification:

- deterministic tests for intent lifecycle and replay protection
- policy rejection tests
- documented scheduled and conditional execution flows

### Tranche 3 - 40% - Treasury Adapters and Mainnet Readiness
Deliverables:

- production-oriented transfer, split, swap, and yield adapter boundaries
- relayer submission workflow
- passkey-first onboarding UX specification
- testnet end-to-end demo
- deployment checklist and operational guidance

Verification:

- successful testnet demo for relayed execution
- audit-ready contract and documentation package
- mainnet-readiness checklist

## Success Metrics
- 50+ organizations onboarded within 12 months of deployment
- 500+ Smart Treasury Accounts deployed
- 250,000+ treasury operations processed on Stellar
- more than USD 100M in treasury assets managed through STA policies
- successful testnet demo for scheduled, conditional, and split treasury operations
- clear evidence that relayers cannot bypass onchain policy

## Risks and Mitigations
- Relayer trust risk: relayers are submit-only. They do not receive authority over funds, and all execution remains bound to onchain policy.
- Replay risk: stored automation uses child execution IDs, execution windows, policy version checks, and consumed execution records.
- Scope creep risk: v1 remains SAC-only and uses approved adapters instead of arbitrary contract execution.
- Treasury security risk: signer roles, weighted thresholds, pause/freeze controls, and delayed recovery separate daily operation from emergency authority.
- UX risk: passkey-first flows and explicit human-readable policy setup reduce complexity for non-technical treasury users.

## Source Links
- Website: https://reto-grau-consulting.ch/en
- Repository: `TODO: add public repository URL`
- Technical architecture: `docs/TECHNICAL_ARCHITECTURE.md`
- Core docs: `docs/00-overview.md`, `docs/01-architecture.md`, `docs/04-wallet-integration.md`, `docs/06-authorization-and-automation-model.md`, `docs/07-contract-specification.md`
