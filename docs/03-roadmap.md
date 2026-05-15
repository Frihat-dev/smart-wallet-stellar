# Roadmap and Milestones

## 1. Delivery Strategy

The project should be built in layers so that the most sensitive part, authorization, is validated before treasury automation and DeFi integration are expanded.

## 2. Phase 1: Specification and Threat Model

Deliverables:

- full architecture specification
- threat model
- signer and policy matrix
- contract boundaries
- TTL and storage lifecycle strategy
- canonical auth payload specification

Exit criteria:

- internal design review complete
- implementation plan approved

## 3. Phase 2: SmartAccount MVP

Deliverables:

- SmartAccount contract
- passkey and admin signer flow
- pause and freeze logic
- session key support

Exit criteria:

- unit tests pass
- integration tests cover main auth flows

## 4. Phase 3: Intent Engine

Deliverables:

- IntentRegistry
- PolicyEngine
- scheduled payment primitive
- revenue split primitive

Exit criteria:

- deterministic execution tests pass
- policy rejection paths validated

## 5. Phase 4: Conditional Execution

Deliverables:

- ConditionVerifier
- approved attestor format
- conditional payment primitive

Exit criteria:

- proof replay protection validated
- attestation expiry and misuse tests pass

## 6. Phase 5: Treasury Adapter

Deliverables:

- one approved swap adapter
- one approved yield adapter
- protocol exposure limits
- slippage controls

Exit criteria:

- integration tests with approved venue
- emergency pause behavior validated

## 7. Phase 6: Relayer and UX

Deliverables:

- relayer integration
- passkey onboarding flow
- human-readable signing prompts
- intent management interface

Exit criteria:

- end-to-end testnet demo
- successful repeated relayed execution flow

## 8. Phase 7: Audit Readiness

Deliverables:

- documentation freeze
- test coverage report
- invariants and known limitations list
- audit package

Exit criteria:

- external audit started
- medium and above issues triaged

## 9. Phase 8: Mainnet Rollout

Deliverables:

- staged mainnet deployment
- account caps and guarded launch
- monitoring dashboards
- incident response procedures

Exit criteria:

- successful limited pilot
- no unresolved critical findings

## 10. Suggested Grant Milestone Mapping

Milestone 1:

- specification
- SmartAccount MVP

Milestone 2:

- policy engine
- intent execution
- testnet demo

Milestone 3:

- treasury adapters
- relayer UX
- audit-ready package

Milestone 4:

- audited mainnet pilot
