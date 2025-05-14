#[cfg(test)]
mod tests {
    use crate::{CrateMarketplace, CrateMarketplaceClient, LicenseTier};
    use soroban_sdk::{
        testutils::Address as _,
        token, Address, Env, String,
    };

    fn create_xlm_token(env: &Env, admin: &Address) -> (Address, token::Client) {
        let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
        let token_client = token::Client::new(env, &contract_address.address());
        let token_admin = token::StellarAssetClient::new(env, &contract_address.address());
        token_admin.mint(admin, &10_000_000_000i128);
        (contract_address.address(), token_client)
    }

    #[test]
    fn test_upload_and_get_sample() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let title = String::from_str(&env, "Lo-Fi Beat #1");
        let cid = String::from_str(&env, "QmTestCID123");
        let genre = String::from_str(&env, "Lo-Fi");

        let sample_id = client.upload_sample(
            &producer,
            &title,
            &cid,
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &genre,
            &95u32,
        );
        assert_eq!(sample_id, 1u32);

        let sample = client.get_sample(&sample_id);
        assert_eq!(sample.title, title);
        assert_eq!(sample.lease_price, 100_000_000i128);
        assert_eq!(sample.premium_price, 500_000_000i128);
        assert_eq!(sample.exclusive_price, 2_000_000_000i128);
        assert_eq!(sample.bpm, 95u32);
        assert!(!sample.is_exclusive);
        assert_eq!(sample.total_sales, 0u32);
    }

    #[test]
    fn test_get_stats_initial() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let (total_samples, total_volume) = client.get_stats();
        assert_eq!(total_samples, 0u32);
        assert_eq!(total_volume, 0i128);
    }

    #[test]
    fn test_get_earnings_zero_initial() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        assert_eq!(client.get_earnings(&producer), 0i128);
    }

    #[test]
    fn test_multiple_uploads_increment_counter() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);

        let id1 = client.upload_sample(
            &producer,
            &String::from_str(&env, "Beat 1"),
            &String::from_str(&env, "QmCID1"),
            &5i128,
            &25i128,
            &100i128,
            &String::from_str(&env, "Hip-Hop"),
            &90u32,
        );
        let id2 = client.upload_sample(
            &producer,
            &String::from_str(&env, "Beat 2"),
            &String::from_str(&env, "QmCID2"),
            &15i128,
            &75i128,
            &300i128,
            &String::from_str(&env, "Trap"),
            &140u32,
        );

        assert_eq!(id1, 1u32);
        assert_eq!(id2, 2u32);

        let (total_samples, _) = client.get_stats();
        assert_eq!(total_samples, 2u32);
    }

    #[test]
    fn test_delist_sample() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "My Beat"),
            &String::from_str(&env, "QmTestCID"),
            &10i128,
            &50i128,
            &200i128,
            &String::from_str(&env, "R&B"),
            &80u32,
        );

        client.delist_sample(&producer, &sample_id);

        let sample = client.get_sample(&sample_id);
        assert!(sample.is_exclusive); // reused as "unavailable" flag
    }

    #[test]
    fn test_purchase_lease_tier_splits_correctly() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform)); // 10% fee
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let buyer = Address::generate(&env);

        let (xlm_addr, xlm_client) = create_xlm_token(&env, &buyer);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Lease Me"),
            &String::from_str(&env, "QmLeaseCID"),
            &10i128,   // lease 10 XLM
            &50i128,
            &200i128,
            &String::from_str(&env, "Trap"),
            &140u32,
        );

        // Buyer purchases Lease tier (tier=0)
        client.purchase_license(&buyer, &sample_id, &0u32);

        // Producer should have 90% of 10 XLM = 9 XLM = 90_000_000 stroops
        let earnings = client.get_earnings(&producer);
        assert_eq!(earnings, 90_000_000i128);

        // Platform address should have 10% = 1 XLM = 10_000_000 stroops
        let platform_balance = xlm_client.balance(&platform);
        assert_eq!(platform_balance, 10_000_000i128);

        // Sample stats updated
        let sample = client.get_sample(&sample_id);
        assert_eq!(sample.total_sales, 1u32);
        assert!(!sample.is_exclusive); // Lease doesn't delist
    }

    #[test]
    fn test_purchase_exclusive_tier_deists_sample() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let buyer = Address::generate(&env);

        let (_xlm_addr, _xlm_client) = create_xlm_token(&env, &buyer);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Exclusive Only"),
            &String::from_str(&env, "QmExclusiveCID"),
            &10i128,
            &50i128,
            &500i128,  // exclusive: 500 XLM
            &String::from_str(&env, "Drill"),
            &135u32,
        );

        // Purchase Exclusive tier (tier=2)
        client.purchase_license(&buyer, &sample_id, &2u32);

        // Sample should now be marked exclusive (unavailable)
        let sample = client.get_sample(&sample_id);
        assert!(sample.is_exclusive);
        assert_eq!(sample.total_sales, 1u32);

        // Producer earns 90% of 500 XLM = 450 XLM = 4_500_000_000 stroops
        let earnings = client.get_earnings(&producer);
        assert_eq!(earnings, 4_500_000_000i128);
    }

    #[test]
    fn test_withdraw_earnings() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let buyer = Address::generate(&env);

        let (_xlm_addr, xlm_client) = create_xlm_token(&env, &buyer);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Withdraw Test"),
            &String::from_str(&env, "QmWithdrawCID"),
            &20i128,
            &100i128,
            &400i128,
            &String::from_str(&env, "Hip-Hop"),
            &90u32,
        );

        // Buy a Lease license
        client.purchase_license(&buyer, &sample_id, &0u32);

        // Producer has 18 XLM earnings (90% of 20 XLM)
        assert_eq!(client.get_earnings(&producer), 180_000_000i128);

        // Withdraw
        let withdrawn = client.withdraw_earnings(&producer);
        assert_eq!(withdrawn, 180_000_000i128);

        // Earnings zeroed out after withdrawal
        assert_eq!(client.get_earnings(&producer), 0i128);

        // Producer wallet received the XLM
        let producer_balance = xlm_client.balance(&producer);
        assert_eq!(producer_balance, 180_000_000i128);
    }

    #[test]
    fn test_stats_track_volume() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let buyer = Address::generate(&env);

        let (_xlm_addr, _xlm_client) = create_xlm_token(&env, &buyer);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Stats Beat"),
            &String::from_str(&env, "QmStatsCID"),
            &10i128,
            &50i128,
            &200i128,
            &String::from_str(&env, "Afrobeats"),
            &95u32,
        );

        let (s0, v0) = client.get_stats();
        assert_eq!(s0, 1u32);
        assert_eq!(v0, 0i128);

        // Purchase Premium tier: 50 XLM
        client.purchase_license(&buyer, &sample_id, &1u32);

        let (s1, v1) = client.get_stats();
        assert_eq!(s1, 1u32);
        assert_eq!(v1, 500_000_000i128); // 50 XLM in stroops
    }

    #[test]
    fn test_get_license_records_tier() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let buyer = Address::generate(&env);

        let (_xlm_addr, _xlm_client) = create_xlm_token(&env, &buyer);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "License Test"),
            &String::from_str(&env, "QmLicCID"),
            &10i128,
            &50i128,
            &200i128,
            &String::from_str(&env, "R&B"),
            &80u32,
        );

        // No license initially
        assert_eq!(client.get_license(&buyer, &sample_id), None);

        // Buy Premium license (tier=1)
        client.purchase_license(&buyer, &sample_id, &1u32);

        assert_eq!(client.get_license(&buyer, &sample_id), Some(LicenseTier::Premium));
    }
}
