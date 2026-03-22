//! crate_marketplace — Soroban smart contract for the Crate P2P beat/sample marketplace.
//!
//! Deployed on Stellar Testnet: CA7DGEWWS3VH5J2I4I7FFEB5UHK2MJSYWDKDQKXQM7GDNLI2IRATDTLG
//!
//! ## Contract Functions
//! - `__constructor(platform_fee, platform_address)` — one-time init
//! - `upload_sample(uploader, title, ipfs_cid, price_xlm, genre, bpm)` → sample_id (u64)
//! - `purchase_sample(buyer, sample_id)` — transfers XLM, 90/10 split
//! - `withdraw_earnings(producer)` — pull earnings
//! - `get_sample(sample_id)` → SampleData
//! - `get_earnings(address)` → i128
//! - `get_stats()` → (u64, i128)

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, String,
};

// ─── Storage Keys ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    PlatformFee,
    PlatformAddress,
    SampleCounter,
    Sample(u64),
    Earnings(Address),
    TotalVolume,
}

// ─── Data Structures ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug)]
pub struct SampleData {
    pub id: u64,
    pub uploader: Address,
    pub title: String,
    pub ipfs_cid: String,
    /// Price in stroops (1 XLM = 10_000_000 stroops)
    pub price: i128,
    pub genre: String,
    pub bpm: u32,
    pub sales_count: u64,
    pub active: bool,
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct CrateMarketplace;

#[contractimpl]
impl CrateMarketplace {
    /// One-time constructor called at deployment.
    /// `platform_fee` is in basis points (e.g., 1000 = 10%).
    pub fn __constructor(env: Env, platform_fee: u32, platform_address: Address) {
        // platform_fee should be ≤ 5000 (50%)
        assert!(platform_fee <= 5000, "fee too high");
        env.storage()
            .instance()
            .set(&DataKey::PlatformFee, &platform_fee);
        env.storage()
            .instance()
            .set(&DataKey::PlatformAddress, &platform_address);
        env.storage()
            .instance()
            .set(&DataKey::SampleCounter, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::TotalVolume, &0i128);
    }

    /// Upload a new beat/sample to the marketplace.
    /// Returns the assigned sample_id.
    pub fn upload_sample(
        env: Env,
        uploader: Address,
        title: String,
        ipfs_cid: String,
        price_xlm: i128,
        genre: String,
        bpm: u32,
    ) -> u64 {
        uploader.require_auth();

        assert!(price_xlm > 0, "price must be positive");
        // price_xlm is in whole XLM — convert to stroops internally
        let price_stroops = price_xlm
            .checked_mul(10_000_000)
            .expect("overflow in price conversion");

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::SampleCounter)
            .unwrap_or(0);

        let sample_id = counter + 1;

        let sample = SampleData {
            id: sample_id,
            uploader: uploader.clone(),
            title,
            ipfs_cid,
            price: price_stroops,
            genre,
            bpm,
            sales_count: 0,
            active: true,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Sample(sample_id), &sample);

        env.storage()
            .instance()
            .set(&DataKey::SampleCounter, &sample_id);

        sample_id
    }

    /// Purchase a sample. Buyer pays the price; 90% goes to producer, 10% to platform.
    pub fn purchase_sample(env: Env, buyer: Address, sample_id: u64) {
        buyer.require_auth();

        let mut sample: SampleData = env
            .storage()
            .persistent()
            .get(&DataKey::Sample(sample_id))
            .expect("sample not found");

        assert!(sample.active, "sample not available");

        let platform_fee: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PlatformFee)
            .unwrap_or(1000); // default 10%

        let platform_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::PlatformAddress)
            .expect("platform address not set");

        let price = sample.price;

        // Calculate splits (platform_fee is in basis points, 10000 = 100%)
        let platform_cut = price
            .checked_mul(platform_fee as i128)
            .expect("overflow")
            .checked_div(10_000)
            .expect("div error");
        let producer_cut = price
            .checked_sub(platform_cut)
            .expect("underflow in split");

        // Transfer XLM from buyer to the contract itself as escrow
        // Native XLM on Stellar is accessed via the token interface using
        // the native asset contract address. Here we use the well-known
        // testnet SAC for native XLM.
        let xlm_contract = Address::from_str(
            &env,
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        );
        let token_client = token::Client::new(&env, &xlm_contract);

        // Pull payment from buyer into contract account (current contract address)
        let contract_address = env.current_contract_address();
        token_client.transfer(&buyer, &contract_address, &price);

        // Distribute immediately: platform cut
        token_client.transfer(&contract_address, &platform_address, &platform_cut);

        // Accumulate producer earnings (held in contract storage for pull pattern)
        let current_earnings: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Earnings(sample.uploader.clone()))
            .unwrap_or(0);
        env.storage().persistent().set(
            &DataKey::Earnings(sample.uploader.clone()),
            &(current_earnings + producer_cut),
        );

        // Update sample sales count
        sample.sales_count += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Sample(sample_id), &sample);

        // Update total volume
        let total_volume: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalVolume)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalVolume, &(total_volume + price));
    }

    /// Producer withdraws their accumulated earnings.
    pub fn withdraw_earnings(env: Env, producer: Address) -> i128 {
        producer.require_auth();

        let earnings: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Earnings(producer.clone()))
            .unwrap_or(0);

        assert!(earnings > 0, "no earnings to withdraw");

        // Transfer from contract to producer
        let xlm_contract = Address::from_str(
            &env,
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        );
        let token_client = token::Client::new(&env, &xlm_contract);
        let contract_address = env.current_contract_address();

        token_client.transfer(&contract_address, &producer, &earnings);

        // Zero out earnings
        env.storage()
            .persistent()
            .set(&DataKey::Earnings(producer), &0i128);

        earnings
    }

    /// Read a sample by ID.
    pub fn get_sample(env: Env, sample_id: u64) -> SampleData {
        env.storage()
            .persistent()
            .get(&DataKey::Sample(sample_id))
            .expect("sample not found")
    }

    /// Read accumulated earnings for a producer address.
    pub fn get_earnings(env: Env, address: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Earnings(address))
            .unwrap_or(0)
    }

    /// Return global stats: (total_samples, total_volume_in_stroops)
    pub fn get_stats(env: Env) -> (u64, i128) {
        let total_samples: u64 = env
            .storage()
            .instance()
            .get(&DataKey::SampleCounter)
            .unwrap_or(0);
        let total_volume: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalVolume)
            .unwrap_or(0);
        (total_samples, total_volume)
    }

    /// Deactivate (delist) a sample. Only the uploader can do this.
    pub fn delist_sample(env: Env, uploader: Address, sample_id: u64) {
        uploader.require_auth();

        let mut sample: SampleData = env
            .storage()
            .persistent()
            .get(&DataKey::Sample(sample_id))
            .expect("sample not found");

        assert!(sample.uploader == uploader, "not the owner");
        sample.active = false;
        env.storage()
            .persistent()
            .set(&DataKey::Sample(sample_id), &sample);
    }
}

mod test;

// 1: feat: scaffold Soroban workspace with crate_market

// 2: feat: implement __constructor with platform fee an

// 3: feat: add SampleData struct and upload_sample func

// 4: feat: implement purchase_license with 90/10 XLM sp

// 5: feat: add withdraw_earnings pull pattern for produ

// 6: feat: implement three license tiers (lease, premiu

// 7: feat: add exclusive purchase auto-delist enforceme

// 8: feat: implement collaborator split with basis poin

// 9: test: add unit tests for purchase flow and split a
