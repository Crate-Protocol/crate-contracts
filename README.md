<div align="center">

<img src="logo.svg" width="100" height="100" alt="Crate Logo" />

# Crate · Contracts

### Soroban smart contracts — the trustless core of the Crate beat marketplace.

[![License](https://img.shields.io/badge/License-MIT-facc15?style=flat-square&labelColor=000)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.81+-facc15?style=flat-square&labelColor=000&logo=rust&logoColor=white)](https://rust-lang.org)
[![Soroban](https://img.shields.io/badge/Soroban-SDK%2021-facc15?style=flat-square&labelColor=000)](https://soroban.stellar.org)
[![Testnet](https://img.shields.io/badge/Testnet-Deployed-facc15?style=flat-square&labelColor=000)](https://stellar.expert/explorer/testnet/contract/CA7DGEWWS3VH5J2I4I7FFEB5UHK2MJSYWDKDQKXQM7GDNLI2IRATDTLG)

[Overview](#overview) · [Functions](#contract-functions) · [Architecture](#architecture) · [Deploy](#deploy) · [Testing](#testing)

</div>

---

## Currently Building

| Feature | Status | Branch |
|---|---|---|
| Core marketplace contract | ✅ Done | `main` |
| Three license tiers (Lease/Premium/Exclusive) | ✅ Done | `main` |
| 90/10 revenue split with XLM token transfer | ✅ Done | `main` |
| Collaborator revenue split (basis points) | 🔄 In Progress | `feat/collab-splits` |
| Dispute resolution for exclusive purchases | 📋 Planned | — |
| Mainnet deployment | 📋 Planned | — |

---

## Overview

The `crate_marketplace` contract is the source of truth for every beat sale on Crate. It handles payments, licensing, and revenue splits — without any intermediary.

No escrow service. No payment processor. No chargebacks. The contract enforces every rule at the protocol level.

> _"A producer selling a $20 beat on Ethereum loses $5+ in gas. On Crate, the contract takes $2. The producer keeps $18."_

---

## Deployed Contract

| Network | Address |
|---|---|
| **Stellar Testnet** | [`CA7DGEWWS3VH5J2I4I7FFEB5UHK2MJSYWDKDQKXQM7GDNLI2IRATDTLG`](https://stellar.expert/explorer/testnet/contract/CA7DGEWWS3VH5J2I4I7FFEB5UHK2MJSYWDKDQKXQM7GDNLI2IRATDTLG) |
| **Stellar Mainnet** | Coming soon |

---

## Contract Functions

### Write

| Function | Auth | Description |
|---|---|---|
| `__constructor(platform_fee, platform_address)` | — | Initialize with fee (basis points) and platform wallet |
| `upload_sample(uploader, title, ipfs_cid, lease_price, premium_price, exclusive_price, genre, bpm)` | `uploader` | List a beat with all three tier prices |
| `purchase_license(buyer, sample_id, token_address, tier)` | `buyer` | Buy a license — XLM transfers immediately, 90/10 split |
| `withdraw_earnings(producer, token_address)` | `producer` | Pull accumulated balance to wallet |
| `delist_sample(uploader, sample_id)` | `uploader` | Remove listing — fails if any license sold |

### Read

| Function | Returns | Description |
|---|---|---|
| `get_sample(sample_id)` | `SampleData` | Full metadata + prices + sale count + exclusivity |
| `get_earnings(address)` | `i128` | Withdrawable balance in stroops |
| `get_stats()` | `(u32, i128)` | Total samples listed + total XLM volume |

---

## Architecture

```
Producer calls upload_sample()
         │
         ▼
┌───────────────────────────────────────────┐
│           crate_marketplace               │
│                                           │
│  SampleData {                             │
│    id, uploader, title, ipfs_cid          │
│    lease_price, premium_price             │
│    exclusive_price, genre, bpm            │
│    is_exclusive, total_sales              │
│  }                                        │
│                                           │
│  purchase_license(buyer, id, token, tier) │
│    ├─ 90% ──▶ producer earnings ledger    │
│    └─ 10% ──▶ platform address (instant)  │
│                                           │
│  withdraw_earnings(producer, token)       │
│    └─ pull balance ──▶ producer wallet    │
└───────────────────────────────────────────┘
              Soroban / Stellar
```

---

## Data Structure

```rust
pub struct SampleData {
    pub id:              u32,
    pub uploader:        Address,
    pub title:           String,
    pub ipfs_cid:        String,    // IPFS content hash
    pub lease_price:     i128,      // stroops
    pub premium_price:   i128,
    pub exclusive_price: i128,
    pub genre:           String,
    pub bpm:             u32,       // validated: 40–300
    pub is_exclusive:    bool,      // true = sold exclusively, delisted
    pub total_sales:     u32,
}
```

---

## License Tiers

| Tier | Value | What the buyer gets |
|---|---|---|
| `0` — Lease | `lease_price` | Non-exclusive, limited commercial, 100k streams |
| `1` — Premium | `premium_price` | Commercial, unlimited streams, radio rights |
| `2` — Exclusive | `exclusive_price` | Full ownership transfer, beat delisted forever |

---

## Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Stellar CLI
cargo install --locked stellar-cli
```

---

## Build & Test

```bash
# Build WASM
make build

# Run tests
make test

# Optimize WASM size (~40% smaller)
make optimize
```

---

## Deploy

```bash
# Deploy contract to testnet
make deploy

# Initialize (one-time)
stellar contract invoke \
  --id YOUR_CONTRACT_ID \
  --source YOUR_ACCOUNT \
  --network testnet \
  -- __constructor \
  --platform_fee 1000 \
  --platform_address YOUR_PLATFORM_ADDRESS
```

---

## Testing

Tests live in `contracts/crate_marketplace/src/test.rs` and cover:

- `upload_sample` — metadata stored, counter incremented
- `purchase_license` — XLM split verification (90/10 arithmetic)
- `withdraw_earnings` — pull pattern, double-withdraw prevention
- `exclusive purchase` — beat auto-delisted after tier-2 sale
- `get_stats` — volume accumulation

---

## Contributing

```bash
# Fork → clone → branch
git checkout -b feat/your-feature

# Make changes, run tests
cargo test

# Open a PR
```

---

## Ecosystem

| Repo | Description |
|---|---|
| [crate-frontend](https://github.com/Crate-Protocol/crate-frontend) | React 18 + TypeScript web app |
| [crate-backend](https://github.com/Crate-Protocol/crate-backend) | Node.js API, IPFS proxy, analytics |
| [crate-mobile](https://github.com/Crate-Protocol/crate-mobile) | React Native mobile app |

---

## License

MIT — see [LICENSE](LICENSE)

---

<div align="center">
  <img src="logo.svg" width="40" alt="Crate" />
  <br/>
  <sub>Built on Stellar · Soroban · Open Source</sub>
  <br/><br/>

  [![Stars](https://img.shields.io/github/stars/Crate-Protocol/crate-contracts?style=flat-square&labelColor=000&color=facc15)](https://github.com/Crate-Protocol/crate-contracts/stargazers)
  [![Forks](https://img.shields.io/github/forks/Crate-Protocol/crate-contracts?style=flat-square&labelColor=000&color=facc15)](https://github.com/Crate-Protocol/crate-contracts/network/members)
  [![Issues](https://img.shields.io/github/issues/Crate-Protocol/crate-contracts?style=flat-square&labelColor=000&color=facc15)](https://github.com/Crate-Protocol/crate-contracts/issues)
</div>
