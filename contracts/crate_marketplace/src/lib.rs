#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String, log, token,
};

const PLATFORM_ADDRESS_KEY: &str = "plat_addr";
const PLATFORM_FEE_KEY:     &str = "plat_fee";
const TOTAL_SAMPLES_KEY:    &str = "tot_samp";
const TOTAL_VOLUME_KEY:     &str = "tot_vol";

#[contracttype]
#[derive(Clone, Debug)]
pub struct SampleData {
    pub id:              u32,
    pub uploader:        Address,
    pub title:           String,
    pub ipfs_cid:        String,
    pub lease_price:     i128,
    pub premium_price:   i128,
    pub exclusive_price: i128,
    pub genre:           String,
    pub bpm:             u32,
    pub is_exclusive:    bool,
    pub total_sales:     u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum LicenseTier {
    Lease,
    Premium,
    Exclusive,
}

#[contracttype]
pub enum DataKey {
    Sample(u32),
    Earnings(Address),
    License(Address, u32),
}

#[contract]
pub struct CrateMarketplace;

#[contractimpl]
impl CrateMarketplace {
    pub fn __constructor(env: Env, platform_fee: u32, platform_address: Address) {
        let storage = env.storage().instance();
        if storage.has(&PLATFORM_ADDRESS_KEY) {
            panic!("Contract already initialized");
        }
        assert!(platform_fee <= 5000, "Fee must be <= 50%");
        storage.set(&PLATFORM_ADDRESS_KEY, &platform_address);
        storage.set(&PLATFORM_FEE_KEY,     &platform_fee);
        storage.set(&TOTAL_SAMPLES_KEY,    &0u32);
        storage.set(&TOTAL_VOLUME_KEY,     &0i128);
        log!(&env, "Crate marketplace deployed, fee: {}bps", platform_fee);
    }

    pub fn upload_sample(
        env: Env,
        uploader:        Address,
        title:           String,
        ipfs_cid:        String,
        lease_price:     i128,
        premium_price:   i128,
        exclusive_price: i128,
        genre:           String,
        bpm:             u32,
    ) -> u32 {
        uploader.require_auth();
        assert!(title.len() > 0, "Title cannot be empty");
        assert!(ipfs_cid.len() > 0, "IPFS CID cannot be empty");
        assert!(bpm >= 40 && bpm <= 300, "BPM must be 40-300");
        assert!(lease_price > 0 && premium_price > 0 && exclusive_price > 0, "All prices must be positive");
        assert!(lease_price < premium_price && premium_price < exclusive_price, "Prices must be lease < premium < exclusive");

        let storage    = env.storage().instance();
        let sample_id: u32 = storage.get(&TOTAL_SAMPLES_KEY).unwrap_or(0) + 1;

        let sample = SampleData {
            id: sample_id, uploader, title, ipfs_cid,
            lease_price, premium_price, exclusive_price,
            genre, bpm, is_exclusive: false, total_sales: 0,
        };
        env.storage().persistent().set(&DataKey::Sample(sample_id), &sample);
        storage.set(&TOTAL_SAMPLES_KEY, &sample_id);
        env.events().publish((symbol_short!("uploaded"), sample_id), uploader.clone());
        sample_id
    }

    pub fn purchase_license(env: Env, buyer: Address, sample_id: u32, token_address: Address, tier: LicenseTier) {
        buyer.require_auth();

        let mut sample: SampleData = env.storage().persistent()
            .get(&DataKey::Sample(sample_id)).expect("Sample not found");
        assert!(!sample.is_exclusive, "This beat has been sold exclusively");

        let price = match tier {
            LicenseTier::Lease     => sample.lease_price,
            LicenseTier::Premium   => sample.premium_price,
            LicenseTier::Exclusive => sample.exclusive_price,
        };

        let storage      = env.storage().instance();
        let platform_fee: u32     = storage.get(&PLATFORM_FEE_KEY).unwrap_or(1000);
        let platform_addr: Address = storage.get(&PLATFORM_ADDRESS_KEY).unwrap();

        let platform_cut  = price * (platform_fee as i128) / 10_000;
        let producer_cut  = price - platform_cut;

        let token = token::Client::new(&env, &token_address);
        token.transfer(&buyer, &sample.uploader,    &producer_cut);
        token.transfer(&buyer, &platform_addr,       &platform_cut);

        let key = DataKey::Earnings(sample.uploader.clone());
        let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(current + producer_cut));

        env.storage().persistent().set(&DataKey::License(buyer, sample_id), &tier);

        sample.total_sales += 1;
        if tier == LicenseTier::Exclusive { sample.is_exclusive = true; }
        env.storage().persistent().set(&DataKey::Sample(sample_id), &sample);

        let total_vol: i128 = storage.get(&TOTAL_VOLUME_KEY).unwrap_or(0);
        storage.set(&TOTAL_VOLUME_KEY, &(total_vol + price));

        log!(&env, "License sold: sample={}, tier={}, price={}", sample_id, tier, price);
    }

    pub fn get_sample(env: Env, sample_id: u32) -> SampleData {
        env.storage().persistent()
            .get(&DataKey::Sample(sample_id))
            .expect("Sample not found")
    }

    pub fn get_earnings(env: Env, address: Address) -> i128 {
        env.storage().persistent()
            .get(&DataKey::Earnings(address))
            .unwrap_or(0)
    }

    pub fn withdraw_earnings(env: Env, producer: Address, token_address: Address) -> i128 {
        producer.require_auth();
        let key      = DataKey::Earnings(producer.clone());
        let earnings: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        assert!(earnings > 0, "Nothing to withdraw");
        env.storage().persistent().set(&key, &0i128);
        let token = token::Client::new(&env, &token_address);
        token.transfer(&env.current_contract_address(), &producer, &earnings);
        log!(&env, "Withdrawn: {} stroops to {}", earnings, producer);
        earnings
    }

    pub fn get_license(env: Env, buyer: Address, sample_id: u32) -> Option<LicenseTier> {
        env.storage().persistent()
            .get(&DataKey::License(buyer, sample_id))
    }

    pub fn delist_sample(env: Env, uploader: Address, sample_id: u32) {
        uploader.require_auth();
        let sample: SampleData = env.storage().persistent()
            .get(&DataKey::Sample(sample_id)).expect("Sample not found");
        assert!(sample.uploader == uploader, "Not your sample");
        assert!(sample.total_sales == 0, "Cannot delist a sample with existing licenses");
        env.storage().persistent().remove(&DataKey::Sample(sample_id));
    }

    pub fn get_stats(env: Env) -> (u32, i128) {
        let storage = env.storage().instance();
        (
            storage.get(&TOTAL_SAMPLES_KEY).unwrap_or(0),
            storage.get(&TOTAL_VOLUME_KEY).unwrap_or(0),
        )
    }

    pub fn get_platform_fee(env: Env) -> u32 {
        env.storage().instance()
            .get(&PLATFORM_FEE_KEY)
            .unwrap_or(0)
    }

    pub fn bump_instance(env: Env) {
        env.storage().instance().extend_ttl(17_280, 17_280);
    }
}
