#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String, Vec, log, token,
};

const PLATFORM_ADDRESS_KEY: &str = "plat_addr";
const PLATFORM_FEE_KEY:     &str = "plat_fee";
const PAYMENT_TOKEN_KEY:    &str = "pay_tok";
const TOTAL_SAMPLES_KEY:    &str = "tot_samp";
const TOTAL_VOLUME_KEY:     &str = "tot_vol";
const TOTAL_PRODUCERS_KEY:  &str = "tot_prod";

// ~1 year of ledgers at 5 s/ledger; entries are bumped to this TTL on every access.
const PERSISTENT_BUMP_AMOUNT: u32 = 535_680;
// If the remaining TTL is still above this threshold the bump is a no-op (saves fees).
const PERSISTENT_MIN_TTL: u32 = 17_280; // ~1 day

#[contracttype]
#[derive(Clone, Debug)]
pub struct SampleData {
    pub id:              u32,
    pub uploader:        Address,
    pub owner:           Address,
    pub title:           String,
    pub ipfs_cid:        String,
    pub lease_price:     i128,
    pub premium_price:   i128,
    pub exclusive_price: i128,
    pub genre:           String,
    pub bpm:             u32,
    pub is_exclusive:    bool,
    pub resale_price:    Option<i128>,
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
#[derive(Clone, Debug)]
pub struct Collaborator {
    pub address: Address,
    pub share_bps: u32,
}

#[contracttype]
pub enum DataKey {
    Sample(u32),
    Earnings(Address),
    License(Address, u32),
    Producer(Address),
    Collaborators(u32),
}

#[contract]
pub struct CrateMarketplace;

#[contractimpl]
impl CrateMarketplace {
    pub fn __constructor(env: Env, platform_fee: u32, platform_address: Address, payment_token: Address) {
        let storage = env.storage().instance();
        if storage.has(&PLATFORM_ADDRESS_KEY) {
            panic!("Contract already initialized");
        }
        assert!(platform_fee <= 5000, "Fee must be <= 50%");
        storage.set(&PLATFORM_ADDRESS_KEY, &platform_address);
        storage.set(&PLATFORM_FEE_KEY,     &platform_fee);
        storage.set(&PAYMENT_TOKEN_KEY,    &payment_token);
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
        collaborators:   Vec<Collaborator>,
    ) -> u32 {
        uploader.require_auth();
        assert!(title.len() > 0, "Title cannot be empty");
        assert!(ipfs_cid.len() > 0, "IPFS CID cannot be empty");
        assert!(bpm >= 40 && bpm <= 300, "BPM must be 40-300");
        assert!(lease_price > 0 && premium_price > 0 && exclusive_price > 0, "All prices must be positive");
        assert!(lease_price < premium_price && premium_price < exclusive_price, "Prices must be lease < premium < exclusive");
        assert!(collaborators.len() <= 3, "Max 3 collaborators allowed");

        let mut collab_total: u32 = 0;
        for c in collaborators.iter() {
            assert!(c.share_bps > 0, "Collaborator share must be positive");
            assert!(c.share_bps < 10_000, "Collaborator share must be less than 100%");
            assert!(c.address != uploader, "Collaborator cannot be the uploader");
            collab_total = collab_total.checked_add(c.share_bps)
                .expect("Collaborator shares overflow");
        }
        assert!(collab_total <= 10_000, "Collaborator shares must sum to <= 100%");

        let storage    = env.storage().instance();
        let sample_id: u32 = storage.get(&TOTAL_SAMPLES_KEY).unwrap_or(0) + 1;

        let sample = SampleData {
            id: sample_id, uploader: uploader.clone(), owner: uploader.clone(), title, ipfs_cid,
            lease_price, premium_price, exclusive_price,
            genre, bpm, is_exclusive: false, resale_price: None, total_sales: 0,
        };
        let sample_key = DataKey::Sample(sample_id);
        env.storage().persistent().set(&sample_key, &sample);
        env.storage().persistent().extend_ttl(&sample_key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);

        storage.set(&TOTAL_SAMPLES_KEY, &sample_id);

        // Store collaborators if any
        if collaborators.len() > 0 {
            let collab_key = DataKey::Collaborators(sample_id);
            env.storage().persistent().set(&collab_key, &collaborators);
            env.storage().persistent().extend_ttl(&collab_key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);
        }

        let producer_key = DataKey::Producer(uploader.clone());
        if !env.storage().persistent().has(&producer_key) {
            env.storage().persistent().set(&producer_key, &true);
            env.storage().persistent().extend_ttl(&producer_key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);
            let producers: u32 = storage.get(&TOTAL_PRODUCERS_KEY).unwrap_or(0);
            storage.set(&TOTAL_PRODUCERS_KEY, &(producers + 1));
        }
        env.events().publish((symbol_short!("uploaded"), sample_id), sample.uploader.clone());
        sample_id
    }

    pub fn purchase_license(env: Env, buyer: Address, sample_id: u32, tier: LicenseTier) {
        buyer.require_auth();

        // Idempotency guard: reject a repeat purchase before any token moves.
        // The license key is (buyer, sample_id), so owning any tier for this
        // sample blocks a second purchase — preventing accidental double-charges
        // (UI double-submit / retry) and griefing-driven repurchase flows.
        let license_key = DataKey::License(buyer.clone(), sample_id);
        if env.storage().persistent().has(&license_key) {
            panic!("License already owned");
        }

        let sample_key = DataKey::Sample(sample_id);
        let mut sample: SampleData = env.storage().persistent()
            .get(&sample_key).expect("Sample not found");
        env.storage().persistent().extend_ttl(&sample_key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);

        assert!(!sample.is_exclusive, "This beat has been sold exclusively");

        let price = match tier {
            LicenseTier::Lease     => sample.lease_price,
            LicenseTier::Premium   => sample.premium_price,
            LicenseTier::Exclusive => sample.exclusive_price,
        };

        let storage      = env.storage().instance();
        let platform_fee: u32     = storage.get(&PLATFORM_FEE_KEY).unwrap_or(1000);
        let platform_addr: Address = storage.get(&PLATFORM_ADDRESS_KEY).unwrap();
        let payment_token: Address = storage.get(&PAYMENT_TOKEN_KEY).unwrap();

        let platform_cut  = price * (platform_fee as i128) / 10_000;
        let producer_cut  = price - platform_cut;

        // The payment token is pinned at construction (see PAYMENT_TOKEN_KEY) rather
        // than accepted as a caller-supplied argument, so a buyer cannot redirect
        // payment through a no-op token contract to mint a license for free.
        //
        // Escrow the producer's cut in the contract; the producer claims it
        // later via withdraw_earnings (which pays out from this balance). The
        // platform fee is forwarded directly. Paying the producer here instead
        // would both skip the earnings ledger and double-pay on withdrawal.
        let token = token::Client::new(&env, &payment_token);
        token.transfer(&buyer, &env.current_contract_address(), &producer_cut);
        token.transfer(&buyer, &platform_addr,                  &platform_cut);

        // Split the producer cut among collaborators (and the uploader gets the remainder)
        let collab_key = DataKey::Collaborators(sample_id);
        let collaborators: Vec<Collaborator> = env.storage().persistent()
            .get(&collab_key)
            .unwrap_or(Vec::new(&env));

        let mut distributed: i128 = 0;
        for c in collaborators.iter() {
            let share = producer_cut * (c.share_bps as i128) / 10_000;
            if share > 0 {
                let earnings_key = DataKey::Earnings(c.address.clone());
                let current: i128 = env.storage().persistent().get(&earnings_key).unwrap_or(0);
                env.storage().persistent().set(&earnings_key, &(current + share));
                env.storage().persistent().extend_ttl(&earnings_key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);
                distributed += share;
            }
        }
        // Remainder goes to the uploader
        let uploader_share = producer_cut - distributed;
        let earnings_key = DataKey::Earnings(sample.uploader.clone());
        let current: i128 = env.storage().persistent().get(&earnings_key).unwrap_or(0);
        env.storage().persistent().set(&earnings_key, &(current + uploader_share));
        env.storage().persistent().extend_ttl(&earnings_key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);

        env.storage().persistent().set(&license_key, &tier);
        env.storage().persistent().extend_ttl(&license_key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);

        sample.total_sales += 1;
        if tier == LicenseTier::Exclusive { 
            sample.is_exclusive = true; 
            sample.owner = buyer.clone();
        }
        env.storage().persistent().set(&sample_key, &sample);
        env.storage().persistent().extend_ttl(&sample_key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);

        let total_vol: i128 = storage.get(&TOTAL_VOLUME_KEY).unwrap_or(0);
        storage.set(&TOTAL_VOLUME_KEY, &(total_vol + price));

        env.events().publish((symbol_short!("licensed"), sample_id), (buyer, price));
        log!(&env, "License sold: sample={}, price={}", sample_id, price);
    }

    pub fn get_sample(env: Env, sample_id: u32) -> SampleData {
        let key = DataKey::Sample(sample_id);
        let sample: SampleData = env.storage().persistent()
            .get(&key)
            .expect("Sample not found");
        env.storage().persistent().extend_ttl(&key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);
        sample
    }

    pub fn get_earnings(env: Env, address: Address) -> i128 {
        let key = DataKey::Earnings(address);
        let earnings: i128 = env.storage().persistent()
            .get(&key)
            .unwrap_or(0);
        if earnings > 0 {
            env.storage().persistent().extend_ttl(&key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);
        }
        earnings
    }

    pub fn withdraw_earnings(env: Env, producer: Address) -> i128 {
        producer.require_auth();
        let key      = DataKey::Earnings(producer.clone());
        let earnings: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        assert!(earnings > 0, "Nothing to withdraw");
        env.storage().persistent().set(&key, &0i128);
        env.storage().persistent().extend_ttl(&key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);
        let payment_token: Address = env.storage().instance().get(&PAYMENT_TOKEN_KEY).unwrap();
        let token = token::Client::new(&env, &payment_token);
        token.transfer(&env.current_contract_address(), &producer, &earnings);
        log!(&env, "Withdrawn: {} stroops to {}", earnings, producer);
        earnings
    }

    pub fn get_license(env: Env, buyer: Address, sample_id: u32) -> Option<LicenseTier> {
        let key = DataKey::License(buyer, sample_id);
        let result: Option<LicenseTier> = env.storage().persistent().get(&key);
        if result.is_some() {
            env.storage().persistent().extend_ttl(&key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);
        }
        result
    }

    pub fn list_resale(env: Env, owner: Address, sample_id: u32, price: i128) {
        owner.require_auth();
        assert!(price > 0, "Price must be positive");
        let key = DataKey::Sample(sample_id);
        let mut sample: SampleData = env.storage().persistent().get(&key).expect("Sample not found");
        assert!(sample.is_exclusive, "Only exclusive samples can be resold");
        assert!(sample.owner == owner, "Not the owner");
        sample.resale_price = Some(price);
        env.storage().persistent().set(&key, &sample);
        env.storage().persistent().extend_ttl(&key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);
        log!(&env, "Sample {} listed for resale at {} by {}", sample_id, price, owner);
    }

    pub fn buy_resale(env: Env, buyer: Address, sample_id: u32) {
        buyer.require_auth();
        let license_key = DataKey::License(buyer.clone(), sample_id);
        if env.storage().persistent().has(&license_key) {
            panic!("License already owned");
        }

        let key = DataKey::Sample(sample_id);
        let mut sample: SampleData = env.storage().persistent().get(&key).expect("Sample not found");
        assert!(sample.is_exclusive, "Sample not exclusive");
        let price = sample.resale_price.expect("Sample not listed for resale");
        
        let storage = env.storage().instance();
        let platform_fee: u32 = storage.get(&PLATFORM_FEE_KEY).unwrap_or(1000);
        let platform_addr: Address = storage.get(&PLATFORM_ADDRESS_KEY).unwrap();
        let payment_token: Address = storage.get(&PAYMENT_TOKEN_KEY).unwrap();

        let platform_cut = price * (platform_fee as i128) / 10_000;
        let producer_cut = price * 1000 / 10_000; // 10% royalty
        let owner_cut = price - platform_cut - producer_cut;

        let token = token::Client::new(&env, &payment_token);
        token.transfer(&buyer, &platform_addr, &platform_cut);
        token.transfer(&buyer, &env.current_contract_address(), &producer_cut);
        token.transfer(&buyer, &sample.owner, &owner_cut);

        let earnings_key = DataKey::Earnings(sample.uploader.clone());
        let current: i128 = env.storage().persistent().get(&earnings_key).unwrap_or(0);
        env.storage().persistent().set(&earnings_key, &(current + producer_cut));
        env.storage().persistent().extend_ttl(&earnings_key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);

        let old_license_key = DataKey::License(sample.owner.clone(), sample_id);
        env.storage().persistent().remove(&old_license_key);
        env.storage().persistent().set(&license_key, &LicenseTier::Exclusive);
        env.storage().persistent().extend_ttl(&license_key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);

        // Ownership transfers
        let new_owner = buyer.clone();
        sample.owner = new_owner.clone();
        sample.resale_price = None;
        sample.total_sales += 1;
        
        env.storage().persistent().set(&key, &sample);
        env.storage().persistent().extend_ttl(&key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);

        let total_vol: i128 = storage.get(&TOTAL_VOLUME_KEY).unwrap_or(0);
        storage.set(&TOTAL_VOLUME_KEY, &(total_vol + price));

        env.events().publish((symbol_short!("resold"), sample_id), (buyer, price));
        log!(&env, "Sample {} resold to {} for {}", sample_id, new_owner, price);
    }

    pub fn delist_sample(env: Env, uploader: Address, sample_id: u32) {
        uploader.require_auth();
        let sample: SampleData = env.storage().persistent()
            .get(&DataKey::Sample(sample_id)).expect("Sample not found");
        assert!(sample.uploader == uploader, "Not your sample");
        assert!(sample.total_sales == 0, "Cannot delist a sample with existing licenses");
        env.storage().persistent().remove(&DataKey::Sample(sample_id));
    }

    pub fn get_stats(env: Env) -> (u32, i128, u32) {
        let storage = env.storage().instance();
        (
            storage.get(&TOTAL_SAMPLES_KEY).unwrap_or(0),
            storage.get(&TOTAL_VOLUME_KEY).unwrap_or(0),
            storage.get(&TOTAL_PRODUCERS_KEY).unwrap_or(0),
        )
    }

    pub fn get_platform_fee(env: Env) -> u32 {
        env.storage().instance()
            .get(&PLATFORM_FEE_KEY)
            .unwrap_or(0)
    }

    pub fn get_payment_token(env: Env) -> Address {
        env.storage().instance()
            .get(&PAYMENT_TOKEN_KEY)
            .unwrap()
    }

    // Allows anyone (e.g. keepers, the frontend) to extend a sample's on-chain TTL
    // without triggering a purchase. Useful for active listings approaching expiry.
    pub fn bump_sample(env: Env, sample_id: u32) {
        let key = DataKey::Sample(sample_id);
        assert!(env.storage().persistent().has(&key), "Sample not found");
        env.storage().persistent().extend_ttl(&key, PERSISTENT_MIN_TTL, PERSISTENT_BUMP_AMOUNT);
    }

    pub fn bump_instance(env: Env) {
        env.storage().instance().extend_ttl(17_280, 17_280);
    }
}

#[cfg(test)]
mod test;
