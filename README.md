# Contract — Fractional Region Strategy Game on Stellar

A blockchain strategy game where players own fractionalized virtual regions, earn automated yield distributions, govern territories, and trade region shares — all powered by Soroban smart contracts on Stellar.

---

## Overview

The world is divided into regions (ports, farms, mines, trade hubs). Each region has a fixed share supply. Players buy fractions, earn proportional revenue, vote on governance, and trade on a decentralized marketplace. Soroban handles all accounting, yield calculation, and ownership tracking on-chain.

**Core loop:**
```
Buy region shares → Region generates yield → Claim proportional payout → Reinvest or trade
```

---

## Smart Contracts

| Contract | Description |
|---|---|
| `region-factory` | Create and configure regions, set tax rates, upgrade productivity |
| `fractional-ownership` | Mint and transfer fractional shares per region |
| `yield-distributor` | Deposit revenue pools and distribute proportional payouts to shareholders |
| `marketplace` | List, buy, and cancel share listings |
| `governance-dao` | Create proposals, vote with share-weighted power, finalize outcomes |
| `treasury` | Per-region fund management with admin-controlled deposits and withdrawals |

---

## Project Structure

```
contracts/
├── region-factory/
├── fractional-ownership/
├── yield-distributor/
├── marketplace/
├── governance-dao/
└── treasury/
Cargo.toml        # workspace
```

---

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- `wasm32-unknown-unknown` target

```bash
rustup target add wasm32-unknown-unknown
```

---

## Build

```bash
cargo build
```

To build optimized WASM for deployment:

```bash
cargo build --release --target wasm32-unknown-unknown
```

---

## Test

```bash
cargo test
```

All contracts include unit tests using `soroban-sdk`'s `testutils` with mocked auth.

---

## Contract Interactions

### Region Factory

```rust
// Create a region (admin only)
create_region(name, region_type, total_shares, tax_rate, governor) -> region_id

// Get region data
get_region(id) -> Region

// Governor sets tax rate (0–100)
set_tax_rate(id, tax_rate)
```

### Fractional Ownership

```rust
// Mint shares to an address (admin only)
mint(region_id, to, amount)

// Transfer shares between players
transfer(region_id, from, to, amount)

// Query balance and supply
balance(region_id, owner) -> u128
total_shares(region_id) -> u128
```

### Yield Distributor

```rust
// Deposit game revenue into a region pool (admin only)
deposit_revenue(region_id, amount)

// Claim proportional yield
claim(region_id, claimant) -> i128

// Preview claimable amount
claimable(region_id, claimant) -> i128
```

**Payout formula:** `payout = (shares / total_shares) × revenue_pool`

### Marketplace

```rust
// List shares for sale
list_shares(seller, region_id, shares, price_per_share) -> listing_id

// Buy shares from a listing
buy_shares(buyer, listing_id, amount) -> total_cost

// Cancel a listing
cancel_listing(listing_id)
```

### Governance DAO

```rust
// Create a proposal for a region
create_proposal(region_id, proposer, description, voting_period_ledgers) -> proposal_id

// Vote (share-weighted)
vote(proposal_id, voter, support: bool)

// Finalize after voting period ends
finalize(proposal_id) -> ProposalStatus
```

**Voting power** = shares held in the region at snapshot time.

### Treasury

```rust
// Deposit funds into a region treasury (admin only)
deposit(region_id, amount)

// Withdraw to a recipient (admin only)
withdraw(region_id, amount, recipient)

// Query balances
balance(region_id) -> i128
total_balance() -> i128
```

---

## Token Model

| Token | Purpose |
|---|---|
| Region Shares | Fractional ownership of a region (e.g. `REGION_NEO_LAGOS`) |
| GOV Token | Governance voting weight (staked alongside shares) |
| GOLD / CREDITS | In-game currency for upgrades, marketplace fees, governance |

---

## Roadmap

- **MVP** — Region minting, fractional ownership, marketplace, yield distribution
- **Alpha** — Governance, staking, treasury management, map simulation
- **Beta** — PvP wars, alliances, advanced economy, dynamic productivity
- **Production** — AI-driven NPCs, seasonal economies, cross-game integrations

---

## Network

Targets **Stellar Testnet** for development and **Stellar Mainnet** for production. Uses [Soroban SDK v22](https://docs.rs/soroban-sdk/22.0.0/soroban_sdk/).
