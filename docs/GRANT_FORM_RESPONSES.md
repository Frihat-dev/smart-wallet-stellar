# SCF Build Form Responses

## Project Title
Smart Treasury Account (STA)

## Project Description
Smart Treasury Account (STA) is a programmable treasury wallet for Stellar. It enables businesses, payment providers, DAOs, marketplaces, and onchain organizations to manage treasury operations through predefined onchain policies.

Instead of manually approving every transaction, users configure rules for authorized signers, supported Stellar Asset Contracts, approved recipients, spending limits, timing constraints, execution conditions, and recovery controls. Treasury operations such as recurring payments, payroll, vendor payments, revenue distribution, and treasury rebalancing can then execute under those rules while remaining fully controlled by the account policy framework.

STA is built on Soroban contract accounts, `__check_auth`, and Stellar Asset Contracts. The system uses narrow adapters for transfers, splits, swaps, and yield actions, with SmartAccount as the root authority.

## Category
Financial Protocols

## Current Traction
STA is led by Reto Grau, a financial markets professional with more than 30 years of experience in asset management and treasury operations, including roles at Man Group, JPMorgan, Swiss Life, and ABB Treasury.

The project also benefits from a partnership with Equisafe, a European tokenization platform that has facilitated more than EUR 400M in investments, supported 25,000+ private investors, and enabled 70+ fundraising rounds.

The current repository includes a working Soroban/Rust workspace with SmartAccount, PolicyEngine, IntentRegistry, ConditionVerifier, RecoveryManager, transfer/split/swap/yield adapters, shared contract types, and detailed architecture/security documentation. The existing test suite validates core SmartAccount and ConditionVerifier behavior.

Within 12 months of deployment, STA targets onboarding 50+ organizations, deploying 500+ Smart Treasury Accounts, processing more than 250,000 treasury operations on Stellar, and supporting more than USD 100M in treasury assets managed.

## Website
https://reto-grau-consulting.ch/en

## Planned Stellar Integration
STA integrates with Stellar through Soroban contract accounts, `__check_auth` authorization, and Stellar Asset Contracts.

Each Smart Treasury Account operates as a contract-controlled treasury wallet that enforces onchain policies for authorized signers, approved assets, spending limits, recipients, execution conditions, and recovery controls.

The project uses Soroban's contract-account model to separate authorization from execution. Users approve treasury actions through policies, intents, and scoped session keys. Future execution remains constrained by explicit onchain rules. Relayers may trigger approved operations, but they never receive authority over funds or the ability to bypass policy restrictions.

Treasury actions are executed through approved adapters for transfers, revenue allocation, swaps, and treasury management. STA supports payroll, vendor payments, recurring transfers, treasury rebalancing, and related treasury workflows for organizations operating on Stellar.

## Interested Build Track
Open Track

## Submitter Type
Entity

## Team Description
Reto Grau - Founder and CEO

Reto Grau brings more than 30 years of experience in asset management, treasury operations, alternative investments, and institutional finance. His career includes senior roles at RMF Investment Management, Man Group, Swiss Life Hedge Fund Partners, JPMorgan, and ABB Treasury.

LinkedIn: https://www.linkedin.com/in/retograu/

Alexandre Karako - Business Development

Alexandre Karako brings experience in digital assets, fundraising, tokenization, and business development. Through his work with Equisafe, he has contributed to tokenized investment products, issuer onboarding, investor relations, and ecosystem growth.

LinkedIn: https://www.linkedin.com/in/alexandrekarako/

Clement Roure - Technical Lead

Clement Roure is an experienced software architect and developer. Through his experience at Keyrock and his work on digital platforms serving more than 400,000 users, he brings expertise in backend systems, scalability, reliability, and financial technology infrastructure.

LinkedIn: https://www.linkedin.com/in/clementroure/

Maxime Sarthet - Strategic Advisor

Maxime Sarthet is CEO of Equisafe, a European tokenization platform. Under his leadership, Equisafe has facilitated EUR 400M+ in investments, supported 25,000+ investors, and enabled 70+ fundraising rounds.

LinkedIn: https://www.linkedin.com/in/maxime-sarthet/

## Referral
Yes

## Repository
TODO: add public repository URL

## Technical Architecture
docs/TECHNICAL_ARCHITECTURE.md
