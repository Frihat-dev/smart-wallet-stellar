# SCF Submission Checklist

## Required Before Final Submission
- Replace `TODO: add public repository URL` in grant docs with the actual public GitHub URL.
- Confirm final legal submitter name and entity details.
- Confirm final requested amount: currently `150,000 XLM`.
- Confirm whether the project should be submitted as `Smart Treasury Account (STA)` everywhere.
- Confirm the website URL: `https://reto-grau-consulting.ch/en`.
- Confirm team bios and LinkedIn links.
- Confirm Equisafe partnership wording is approved for public use.
- Add any demo video, deck, or public roadmap link if available.

## Repository Evidence
- `README.md`
- `docs/00-overview.md`
- `docs/01-architecture.md`
- `docs/02-security.md`
- `docs/04-wallet-integration.md`
- `docs/06-authorization-and-automation-model.md`
- `docs/07-contract-specification.md`
- `docs/10-rust-layout-and-test-matrix.md`
- `docs/TECHNICAL_ARCHITECTURE.md`
- `docs/GRANT_APPLICATION.md`
- `docs/GRANT_FORM_RESPONSES.md`
- `docs/GRANT_MILESTONES_BUDGET.md`
- `docs/GRANT_TECHNICAL_SPEC.md`

## Technical Evidence
- Workspace builds with `cargo test --workspace`.
- SmartAccount tests cover signer roles, thresholds, session keys, adapter dispatch, automation, attestation, and recovery.
- ConditionVerifier tests cover delayed governance, attestor quorum, duplicate attestors, and proof consumption.

## Suggested Submission Attachments
- Grant application: `docs/GRANT_APPLICATION.md`
- Form responses: `docs/GRANT_FORM_RESPONSES.md`
- Milestones and budget: `docs/GRANT_MILESTONES_BUDGET.md`
- Technical architecture: `docs/TECHNICAL_ARCHITECTURE.md`
- Technical specification: `docs/GRANT_TECHNICAL_SPEC.md`

## Follow-Up Work After Submission
- Publish repository or confirm repository visibility.
- Add testnet deployment scripts once contract addresses are available.
- Add relayer workflow documentation.
- Add passkey onboarding UX flow.
- Complete IntentRegistry implementation.
- Expand PolicyEngine rules and tests.
