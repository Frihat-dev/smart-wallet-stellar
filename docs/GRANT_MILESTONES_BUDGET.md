# Grant Milestones and Budget

## Total Request
- **150,000 XLM**
- Estimated timeline: 6 months
- Payment structure: 10% acceptance, then 20% / 30% / 40% tranche payments

## Budget Rationale
The grant supports future development of Smart Treasury Account (STA) from the current Soroban contract workspace into a testnet-demo-ready and mainnet-ready programmable treasury product.

The budget is focused on:

- Soroban smart-account engineering
- intent and policy implementation
- treasury adapter hardening
- relayer workflow integration
- passkey-first onboarding design
- deterministic testing and documentation
- deployment and operational readiness

It excludes audit costs, legal costs, marketing spend, and reimbursement for past work.

## Payment Breakdown
| Payment | Amount | Purpose |
|---|---:|---|
| Acceptance | 15,000 XLM | project kickoff, repository baseline, public work plan, acceptance test plan |
| Tranche 1 | 30,000 XLM | SmartAccount MVP, signer roles, session keys, pause/freeze, recovery controls |
| Tranche 2 | 45,000 XLM | IntentRegistry, PolicyEngine, scheduled payment, split and conditional execution primitives |
| Tranche 3 | 60,000 XLM | treasury adapters, relayer workflow, testnet demo, mainnet-ready package |

## Timeline
| Period | Duration | Focus |
|---|---:|---|
| Month 0-1 | 1 month | kickoff, repository hardening, grant baseline, acceptance test plan |
| Month 1-3 | 2 months | SmartAccount MVP, signer model, session keys, recovery controls |
| Month 3-5 | 2 months | intent engine, policy engine, scheduled and conditional treasury flows |
| Month 5-6 | 1 month | adapter hardening, relayer flow, testnet demo, deployment package |

## Acceptance - 10%
Deliverables:

- public project work plan or milestone tracker
- repository and documentation baseline
- acceptance test plan
- SmartAccount initialization and authorization model documented
- confirmation of v1 scope: SAC-only assets, narrow adapters, no arbitrary execution

Verification:

- repository contains grant docs, architecture docs, and contract workspace
- acceptance criteria are mapped to tests or concrete demos

## Tranche 1 - 20% - SmartAccount MVP
Deliverables:

- SmartAccount contract hardened for core treasury wallet use
- signer registry with role separation
- weighted management, governance, spend, and recovery thresholds
- scoped session key creation and enforcement
- pause, freeze, and recovery controls
- interactive execution path through `__check_auth`

Verification:

- passing tests for signer management, threshold enforcement, session scopes, and pause/freeze behavior
- documented SmartAccount initialization flow
- documented signer-management and recovery flow

## Tranche 2 - 30% - Intent Engine and Policy
Deliverables:

- IntentRegistry parent intent records
- child execution records and replay protection
- PolicyEngine rules for supported actions, asset risk tier, destinations, and spending limits
- policy version pinning for automation capabilities
- scheduled payment primitive
- revenue split primitive
- conditional execution integrated with ConditionVerifier

Verification:

- tests for parent intent lifecycle
- tests for child execution uniqueness and replay rejection
- tests for policy rejection paths
- documented scheduled, recurring, and conditional execution flows

## Tranche 3 - 40% - Treasury Adapters and Mainnet Readiness
Deliverables:

- hardened transfer, split, swap, and yield adapter boundaries
- adapter allowlist and asset allowlist documentation
- relayer submission flow
- passkey-first onboarding UX specification
- end-to-end testnet demo
- mainnet deployment checklist
- operational monitoring and incident response guidance

Verification:

- successful end-to-end testnet execution of at least:
  - scheduled payment
  - split/revenue distribution
  - conditional payment with attestation
- audit-ready documentation package
- mainnet-readiness checklist

## Success Metrics
- 50+ organizations onboarded within 12 months of deployment
- 500+ Smart Treasury Accounts deployed
- 250,000+ treasury operations processed on Stellar
- more than USD 100M in treasury assets managed through STA policies
- successful testnet demo of relayed, policy-bound execution
- evidence that relayers cannot move funds outside explicit policy authority

## Notes
- The current implementation already validates a substantial part of SmartAccount and ConditionVerifier behavior.
- Future grant work should focus on completing IntentRegistry, expanding PolicyEngine, hardening adapters, and building the testnet relayer/onboarding path.
