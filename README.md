# sampled-contracts

Soroban smart contract for the Sampled P2P beat/sample marketplace on Stellar blockchain.

## Deployed Contract

**Testnet:** `CA7DGEWWS3VH5J2I4I7FFEB5UHK2MJSYWDKDQKXQM7GDNLI2IRATDTLG`

> The contract is already deployed. The source here matches the deployed bytecode.
> Use `make deploy-testnet` only if deploying a new instance.

## Contract Functions

| Function | Description |
|---|---|
| `__constructor(platform_fee, platform_address)` | One-time init. `platform_fee` in basis points (1000 = 10%) |
| `upload_sample(uploader, title, ipfs_cid, price_xlm, genre, bpm)` | List a beat. Returns `sample_id` |
| `purchase_sample(buyer, sample_id)` | Buy a beat. Splits payment 90% producer / 10% platform |
| `withdraw_earnings(producer)` | Producer pulls their accumulated XLM earnings |
| `get_sample(sample_id)` | Read sample metadata |
| `get_earnings(address)` | Read pending earnings for a producer |
| `get_stats()` | Returns `(total_samples, total_volume_in_stroops)` |
| `delist_sample(uploader, sample_id)` | Take a beat off the marketplace |

## Revenue Split

- **90%** → Producer (accumulated, withdrawal via `withdraw_earnings`)
- **10%** → Platform (transferred immediately on purchase)

## Setup

```bash
# Install Rust + wasm32 target
rustup target add wasm32-unknown-unknown

# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Build
make build

# Test
make test
```

## Deployment (new instance)

```bash
# Create and fund a testnet identity
stellar keys generate my-identity --network testnet
stellar keys fund my-identity --network testnet

# Deploy
make deploy-testnet IDENTITY=my-identity
```

## Network Config

See `environments.toml` for RPC URLs and network passphrases.

## Tech Stack

- **Soroban SDK** v21.7.6
- **Rust** 2021 edition
- **Target:** wasm32-unknown-unknown
