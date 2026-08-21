#[cfg(test)]
mod tests {
    use crate::{Collaborator, CrateMarketplace, CrateMarketplaceClient, LicenseTier};
    use soroban_sdk::{
        testutils::Address as _,
        token, Address, Env, String, Vec,
    };

    fn create_xlm_token<'a>(env: &'a Env, admin: &Address) -> (Address, token::Client<'a>) {
        let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
        let token_client = token::Client::new(env, &contract_address.address());
        let token_admin = token::StellarAssetClient::new(env, &contract_address.address());
        token_admin.mint(admin, &10_000_000_000i128);
        (contract_address.address(), token_client)
    }

    // Most tests never move tokens, so a freshly generated address is enough
    // to satisfy the constructor's payment-token parameter.
    fn dummy_token(env: &Env) -> Address {
        Address::generate(env)
    }

    #[test]
    fn test_upload_and_get_sample() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
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
            &Vec::new(&env),
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
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let (total_samples, total_volume, _) = client.get_stats();
        assert_eq!(total_samples, 0u32);
        assert_eq!(total_volume, 0i128);
    }

    #[test]
    fn test_get_earnings_zero_initial() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        assert_eq!(client.get_earnings(&producer), 0i128);
    }

    #[test]
    fn test_multiple_uploads_increment_counter() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);

        let id1 = client.upload_sample(
            &producer,
            &String::from_str(&env, "Beat 1"),
            &String::from_str(&env, "QmCID1"),
            &50_000_000i128,
            &250_000_000i128,
            &1_000_000_000i128,
            &String::from_str(&env, "Hip-Hop"),
            &90u32,
            &Vec::new(&env),
        );
        let id2 = client.upload_sample(
            &producer,
            &String::from_str(&env, "Beat 2"),
            &String::from_str(&env, "QmCID2"),
            &150_000_000i128,
            &750_000_000i128,
            &3_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
            &Vec::new(&env),
        );

        assert_eq!(id1, 1u32);
        assert_eq!(id2, 2u32);

        let (total_samples, _, _) = client.get_stats();
        assert_eq!(total_samples, 2u32);
    }

    #[test]
    fn test_delist_sample_removes_record() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "My Beat"),
            &String::from_str(&env, "QmTestCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "R&B"),
            &80u32,
            &Vec::new(&env),
        );

        client.delist_sample(&producer, &sample_id);
        assert_eq!(client.get_earnings(&producer), 0i128);
    }

    #[test]
    #[should_panic(expected = "Sample not found")]
    fn test_get_sample_after_delist_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "My Beat"),
            &String::from_str(&env, "QmTestCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "R&B"),
            &80u32,
            &Vec::new(&env),
        );

        client.delist_sample(&producer, &sample_id);
        client.get_sample(&sample_id);
    }

    #[test]
    fn test_purchase_lease_tier_splits_correctly() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let buyer = Address::generate(&env);
        let (xlm_addr, xlm_client) = create_xlm_token(&env, &buyer);

        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &xlm_addr)); // 10% fee
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Lease Me"),
            &String::from_str(&env, "QmLeaseCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
            &Vec::new(&env),
        );

        client.purchase_license(&buyer, &sample_id, &LicenseTier::Lease);

        let earnings = client.get_earnings(&producer);
        assert_eq!(earnings, 90_000_000i128);

        let platform_balance = xlm_client.balance(&platform);
        assert_eq!(platform_balance, 10_000_000i128);

        let sample = client.get_sample(&sample_id);
        assert_eq!(sample.total_sales, 1u32);
        assert!(!sample.is_exclusive);
    }

    #[test]
    fn test_purchase_exclusive_tier_deists_sample() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let buyer = Address::generate(&env);
        let (xlm_addr, _xlm_client) = create_xlm_token(&env, &buyer);

        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &xlm_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Exclusive Only"),
            &String::from_str(&env, "QmExclusiveCID"),
            &100_000_000i128,
            &500_000_000i128,
            &5_000_000_000i128,
            &String::from_str(&env, "Drill"),
            &135u32,
            &Vec::new(&env),
        );

        client.purchase_license(&buyer, &sample_id, &LicenseTier::Exclusive);

        // Sample should now be marked exclusive (unavailable)
        let sample = client.get_sample(&sample_id);
        assert!(sample.is_exclusive);
        assert_eq!(sample.total_sales, 1u32);

        let earnings = client.get_earnings(&producer);
        assert_eq!(earnings, 4_500_000_000i128);
    }

    #[test]
    fn test_withdraw_earnings() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let buyer = Address::generate(&env);
        let (xlm_addr, xlm_client) = create_xlm_token(&env, &buyer);

        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &xlm_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Withdraw Test"),
            &String::from_str(&env, "QmWithdrawCID"),
            &200_000_000i128,
            &1_000_000_000i128,
            &4_000_000_000i128,
            &String::from_str(&env, "Hip-Hop"),
            &90u32,
            &Vec::new(&env),
        );

        client.purchase_license(&buyer, &sample_id, &LicenseTier::Lease);

        assert_eq!(client.get_earnings(&producer), 180_000_000i128);

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
        let buyer = Address::generate(&env);
        let (xlm_addr, _xlm_client) = create_xlm_token(&env, &buyer);

        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &xlm_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Stats Beat"),
            &String::from_str(&env, "QmStatsCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Afrobeats"),
            &95u32,
            &Vec::new(&env),
        );

        let (s0, v0, _) = client.get_stats();
        assert_eq!(s0, 1u32);
        assert_eq!(v0, 0i128);

        client.purchase_license(&buyer, &sample_id, &LicenseTier::Premium);

        let (s1, v1, _) = client.get_stats();
        assert_eq!(s1, 1u32);
        assert_eq!(v1, 500_000_000i128);
    }

    #[test]
    fn test_get_license_records_tier() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let buyer = Address::generate(&env);
        let (xlm_addr, _xlm_client) = create_xlm_token(&env, &buyer);

        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &xlm_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "License Test"),
            &String::from_str(&env, "QmLicCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "R&B"),
            &80u32,
            &Vec::new(&env),
        );

        assert_eq!(client.get_license(&buyer, &sample_id), None);

        client.purchase_license(&buyer, &sample_id, &LicenseTier::Premium);

        assert_eq!(client.get_license(&buyer, &sample_id), Some(LicenseTier::Premium));
    }

    #[test]
    #[should_panic(expected = "BPM must be 40-300")]
    fn test_upload_invalid_bpm_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        let producer = Address::generate(&env);
        client.upload_sample(
            &producer,
            &String::from_str(&env, "Bad BPM"),
            &String::from_str(&env, "QmBPMCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &350u32,
            &Vec::new(&env),
        );
    }

    #[test]
    #[should_panic(expected = "Not your sample")]
    fn test_delist_non_owner_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        let producer = Address::generate(&env);
        let intruder = Address::generate(&env);
        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Protected"),
            &String::from_str(&env, "QmProtCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "R&B"),
            &80u32,
            &Vec::new(&env),
        );
        client.delist_sample(&intruder, &sample_id);
    }

    #[test]
    #[should_panic(expected = "This beat has been sold exclusively")]
    fn test_purchase_after_exclusive_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let buyer1 = Address::generate(&env);
        let buyer2 = Address::generate(&env);
        let (xlm_addr, _) = create_xlm_token(&env, &buyer1);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &xlm_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        let producer = Address::generate(&env);
        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Exclusive"),
            &String::from_str(&env, "QmExclCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Drill"),
            &120u32,
            &Vec::new(&env),
        );
        client.purchase_license(&buyer1, &sample_id, &LicenseTier::Exclusive);
        client.purchase_license(&buyer2, &sample_id, &LicenseTier::Lease);
    }

    #[test]
    #[should_panic(expected = "License already owned")]
    fn test_repeat_purchase_same_buyer_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let buyer = Address::generate(&env);
        let (xlm_addr, _) = create_xlm_token(&env, &buyer);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &xlm_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        let producer = Address::generate(&env);
        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Buy Once"),
            &String::from_str(&env, "QmOnceCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
            &Vec::new(&env),
        );
        client.purchase_license(&buyer, &sample_id, &LicenseTier::Lease);
        // Second purchase by the same buyer for the same sample must be rejected.
        client.purchase_license(&buyer, &sample_id, &LicenseTier::Premium);
    }

    #[test]
    #[should_panic(expected = "Prices must be lease < premium < exclusive")]
    fn test_upload_invalid_price_ordering_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        let producer = Address::generate(&env);
        client.upload_sample(
            &producer,
            &String::from_str(&env, "Bad Prices"),
            &String::from_str(&env, "QmBadCID"),
            &500_000_000i128,
            &100_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
            &Vec::new(&env),
        );
    }

    #[test]
    #[should_panic(expected = "Nothing to withdraw")]
    fn test_withdraw_zero_earnings_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let producer = Address::generate(&env);
        let (xlm_addr, _) = create_xlm_token(&env, &producer);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &xlm_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        client.withdraw_earnings(&producer);
    }

    #[test]
    fn test_get_platform_fee() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&750u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        assert_eq!(client.get_platform_fee(), 750u32);
    }

    #[test]
    fn test_get_payment_token() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        assert_eq!(client.get_payment_token(), token);
    }

    #[test]
    #[should_panic(expected = "Title cannot be empty")]
    fn test_upload_empty_title_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        let producer = Address::generate(&env);
        client.upload_sample(
            &producer,
            &String::from_str(&env, ""),
            &String::from_str(&env, "QmCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
            &Vec::new(&env),
        );
    }

    #[test]
    fn test_bump_instance_does_not_panic() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        client.bump_instance();
    }

    #[test]
    fn test_get_license_returns_none_before_purchase() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        let buyer = Address::generate(&env);
        assert_eq!(client.get_license(&buyer, &999u32), None);
    }

    #[test]
    #[should_panic(expected = "Fee must be <= 50%")]
    fn test_constructor_rejects_fee_over_5000() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        env.register(CrateMarketplace, (&6000u32, &platform, &token));
    }

    #[test]
    fn test_bump_sample_extends_ttl_without_panic() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        let producer = Address::generate(&env);
        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Bump Test"),
            &String::from_str(&env, "QmBumpCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
            &Vec::new(&env),
        );
        // Must not panic and sample must still be readable afterwards.
        client.bump_sample(&sample_id);
        let sample = client.get_sample(&sample_id);
        assert_eq!(sample.id, sample_id);
    }

    #[test]
    #[should_panic(expected = "Sample not found")]
    fn test_bump_sample_nonexistent_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        client.bump_sample(&999u32);
    }

    #[test]
    #[should_panic(expected = "IPFS CID cannot be empty")]
    fn test_upload_empty_cid_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);
        let producer = Address::generate(&env);
        client.upload_sample(
            &producer,
            &String::from_str(&env, "Some Beat"),
            &String::from_str(&env, ""),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
            &Vec::new(&env),
        );
    }

    #[test]
    fn test_list_resale_and_buy_resale() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let buyer = Address::generate(&env);
        let buyer2 = Address::generate(&env);
        let (xlm_addr, xlm_client) = create_xlm_token(&env, &platform);
        
        // Mint to buyer and buyer2
        let token_admin = token::StellarAssetClient::new(&env, &xlm_addr);
        token_admin.mint(&buyer, &10_000_000_000i128);
        token_admin.mint(&buyer2, &10_000_000_000i128);

        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &xlm_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Resale Test"),
            &String::from_str(&env, "QmResale"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
            &Vec::new(&env),
        );

        // Buyer buys exclusive
        client.purchase_license(&buyer, &sample_id, &LicenseTier::Exclusive);
        
        let sample_after_buy = client.get_sample(&sample_id);
        assert!(sample_after_buy.is_exclusive);
        assert_eq!(sample_after_buy.owner, buyer);

        // Buyer lists for resale
        client.list_resale(&buyer, &sample_id, &3_000_000_000i128);
        
        let sample_after_list = client.get_sample(&sample_id);
        assert_eq!(sample_after_list.resale_price, Some(3_000_000_000i128));

        // Buyer2 buys resale
        client.buy_resale(&buyer2, &sample_id);
        
        let sample_after_resale = client.get_sample(&sample_id);
        assert_eq!(sample_after_resale.owner, buyer2);
        assert_eq!(sample_after_resale.resale_price, None);
        assert_eq!(sample_after_resale.total_sales, 2u32);

        // Check splits
        // Price was 3B. Platform gets 10% (300M). Producer gets 10% (300M). Owner gets 80% (2.4B).
        let platform_balance = xlm_client.balance(&platform);
        // Platform gets 200M from original 2B purchase, plus 300M from resale = 500M
        assert_eq!(platform_balance, 500_000_000i128);

        // Producer gets 1.8B from original purchase, plus 300M from resale = 2.1B pending
        let earnings = client.get_earnings(&producer);
        assert_eq!(earnings, 2_100_000_000i128);

        // Buyer gets 2.4B back into wallet
        // Started with 10B, spent 2B, gained 2.4B = 10.4B
        let buyer_balance = xlm_client.balance(&buyer);
        assert_eq!(buyer_balance, 10_400_000_000i128);
    }

    #[test]
    fn test_producer_count_is_unique_per_address() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer_a = Address::generate(&env);
        let producer_b = Address::generate(&env);

        // producer_a uploads 3 samples — should only count as 1 unique producer
        for i in 1u32..=3 {
            let title = String::from_str(&env, if i == 1 { "Beat A1" } else if i == 2 { "Beat A2" } else { "Beat A3" });
            client.upload_sample(
                &producer_a,
                &title,
                &String::from_str(&env, "QmCIDa"),
                &100_000_000i128,
                &500_000_000i128,
                &2_000_000_000i128,
                &String::from_str(&env, "Hip-Hop"),
                &90u32,
            &Vec::new(&env),
            );
        }

        let (samples, _, producers) = client.get_stats();
        assert_eq!(samples, 3u32, "3 samples uploaded");
        assert_eq!(producers, 1u32, "only 1 unique producer after 3 uploads by same address");

        // producer_b uploads 1 sample — producer count should become 2
        client.upload_sample(
            &producer_b,
            &String::from_str(&env, "Beat B1"),
            &String::from_str(&env, "QmCIDb"),
            &200_000_000i128,
            &800_000_000i128,
            &3_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
            &Vec::new(&env),
        );

        let (samples2, _, producers2) = client.get_stats();
        assert_eq!(samples2, 4u32, "4 total samples");
        assert_eq!(producers2, 2u32, "2 unique producers");
    }

    // Note: there is no `test_double_init_panics`. Under soroban-sdk 22 the
    // `__constructor` runs exactly once at registration and is not exposed on
    // the generated client, so a second call is impossible through the public
    // API. The `has(PLATFORM_ADDRESS_KEY)` guard in the contract remains as
    // defense-in-depth, but it is unreachable from a test.

    // ── Collaborator revenue-split tests ──────────────────────────────────

    #[test]
    fn test_upload_with_collaborators() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let collab_a = Address::generate(&env);
        let collab_b = Address::generate(&env);

        let collaborators = Vec::from_array(&env, [
            Collaborator { address: collab_a.clone(), share_bps: 3000 },
            Collaborator { address: collab_b.clone(), share_bps: 2000 },
        ]);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Collab Beat"),
            &String::from_str(&env, "QmCollabCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Hip-Hop"),
            &90u32,
            &collaborators,
        );

        let stored = client.get_collaborators(&sample_id);
        assert_eq!(stored.len(), 2u32);
        assert_eq!(stored.get_unchecked(0).address, collab_a);
        assert_eq!(stored.get_unchecked(0).share_bps, 3000u32);
        assert_eq!(stored.get_unchecked(1).address, collab_b);
        assert_eq!(stored.get_unchecked(1).share_bps, 2000u32);
    }

    #[test]
    fn test_collaborator_split_on_purchase() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let (token_addr, _xlm_client) = create_xlm_token(&env, &platform);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let collab_a = Address::generate(&env);
        let collab_b = Address::generate(&env);
        let buyer = Address::generate(&env);

        // Mint tokens to buyer
        let token_admin = token::StellarAssetClient::new(&env, &token_addr);
        token_admin.mint(&buyer, &10_000_000_000i128);

        let collaborators = Vec::from_array(&env, [
            Collaborator { address: collab_a.clone(), share_bps: 3000 },
            Collaborator { address: collab_b.clone(), share_bps: 2000 },
        ]);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Split Beat"),
            &String::from_str(&env, "QmSplitCID"),
            &100_000_000i128, // 10 XLM lease
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
            &collaborators,
        );

        // Purchase lease: 100_000_000 stroops
        // Platform cut (10%): 10_000_000
        // Producer cut: 90_000_000
        // Collab A (30%): 27_000_000
        // Collab B (20%): 18_000_000
        // Producer remainder: 45_000_000
        client.purchase_license(&buyer, &sample_id, &LicenseTier::Lease);

        assert_eq!(client.get_earnings(&collab_a), 27_000_000i128);
        assert_eq!(client.get_earnings(&collab_b), 18_000_000i128);
        assert_eq!(client.get_earnings(&producer), 45_000_000i128);
    }

    #[test]
    fn test_collaborators_can_withdraw_independently() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let (token_addr, _xlm_client) = create_xlm_token(&env, &platform);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let collab = Address::generate(&env);
        let buyer = Address::generate(&env);

        let token_admin = token::StellarAssetClient::new(&env, &token_addr);
        token_admin.mint(&buyer, &10_000_000_000i128);

        let collaborators = Vec::from_array(&env, [
            Collaborator { address: collab.clone(), share_bps: 5000 },
        ]);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Withdraw Collab"),
            &String::from_str(&env, "QmWdCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "R&B"),
            &80u32,
            &collaborators,
        );

        client.purchase_license(&buyer, &sample_id, &LicenseTier::Lease);

        // Collab gets 50% of 90M = 45M
        assert_eq!(client.get_earnings(&collab), 45_000_000i128);
        let withdrawn = client.withdraw_earnings(&collab);
        assert_eq!(withdrawn, 45_000_000i128);
        assert_eq!(client.get_earnings(&collab), 0i128);

        // Producer gets remainder
        assert_eq!(client.get_earnings(&producer), 45_000_000i128);
    }

    #[test]
    fn test_collaborator_split_on_resale() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let (token_addr, _xlm_client) = create_xlm_token(&env, &platform);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let collab = Address::generate(&env);
        let buyer = Address::generate(&env);
        let resale_buyer = Address::generate(&env);

        let token_admin = token::StellarAssetClient::new(&env, &token_addr);
        token_admin.mint(&buyer, &10_000_000_000i128);
        token_admin.mint(&resale_buyer, &10_000_000_000i128);

        let collaborators = Vec::from_array(&env, [
            Collaborator { address: collab.clone(), share_bps: 4000 },
        ]);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Resale Collab"),
            &String::from_str(&env, "QmResCID"),
            &100_000_000i128,
            &500_000_000i128,
            &5_000_000_000i128,
            &String::from_str(&env, "Drill"),
            &135u32,
            &collaborators,
        );

        // Buyer purchases exclusive to enable resale
        client.purchase_license(&buyer, &sample_id, &LicenseTier::Exclusive);

        // List for resale at 1_000_000_000 (100 XLM)
        client.list_resale(&buyer, &sample_id, &1_000_000_000i128);

        // Resale splits:
        // Platform (10%): 100_000_000
        // Producer royalty (10%): 100_000_000
        //   Collab (40% of royalty): 40_000_000
        //   Producer (remainder): 60_000_000
        // Owner gets: 800_000_000
        client.buy_resale(&resale_buyer, &sample_id);

        // Collab total: exclusive purchase (1_800_000_000) + resale royalty (40_000_000)
        assert_eq!(client.get_earnings(&collab), 1_840_000_000i128);
        // Producer total: exclusive purchase (2_700_000_000) + resale royalty (60_000_000)
        assert_eq!(client.get_earnings(&producer), 2_760_000_000i128);
    }

    #[test]
    #[should_panic(expected = "Max 3 collaborators allowed")]
    fn test_upload_too_many_collaborators_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let collaborators = Vec::from_array(&env, [
            Collaborator { address: Address::generate(&env), share_bps: 1000 },
            Collaborator { address: Address::generate(&env), share_bps: 1000 },
            Collaborator { address: Address::generate(&env), share_bps: 1000 },
            Collaborator { address: Address::generate(&env), share_bps: 1000 },
        ]);

        client.upload_sample(
            &producer,
            &String::from_str(&env, "Too Many"),
            &String::from_str(&env, "QmTooMany"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Pop"),
            &120u32,
            &collaborators,
        );
    }

    #[test]
    #[should_panic(expected = "Collaborator shares must sum to <= 100%")]
    fn test_collaborator_shares_exceed_100_percent_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let collaborators = Vec::from_array(&env, [
            Collaborator { address: Address::generate(&env), share_bps: 6000 },
            Collaborator { address: Address::generate(&env), share_bps: 5000 },
        ]);

        client.upload_sample(
            &producer,
            &String::from_str(&env, "Over Shares"),
            &String::from_str(&env, "QmOverCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Jazz"),
            &110u32,
            &collaborators,
        );
    }

    #[test]
    #[should_panic(expected = "Collaborator cannot be the uploader")]
    fn test_collaborator_cannot_be_uploader_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let collaborators = Vec::from_array(&env, [
            Collaborator { address: producer.clone(), share_bps: 5000 },
        ]);

        client.upload_sample(
            &producer,
            &String::from_str(&env, "Self Collab"),
            &String::from_str(&env, "QmSelfCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Funk"),
            &100u32,
            &collaborators,
        );
    }

    #[test]
    fn test_upload_without_collaborators_gives_producer_full_share() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let (token_addr, _xlm_client) = create_xlm_token(&env, &platform);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token_addr));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let buyer = Address::generate(&env);

        let token_admin = token::StellarAssetClient::new(&env, &token_addr);
        token_admin.mint(&buyer, &10_000_000_000i128);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "No Collab"),
            &String::from_str(&env, "QmNoCollabCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Lo-Fi"),
            &95u32,
            &Vec::new(&env),
        );

        client.purchase_license(&buyer, &sample_id, &LicenseTier::Lease);

        // Without collaborators, producer gets the full 90% cut
        assert_eq!(client.get_earnings(&producer), 90_000_000i128);
        assert_eq!(client.get_collaborators(&sample_id).len(), 0u32);
    }

    #[test]
    fn test_get_collaborators_empty_for_sample_without_collabs() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Solo Beat"),
            &String::from_str(&env, "QmSoloCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Lo-Fi"),
            &95u32,
            &Vec::new(&env),
        );

        let collabs = client.get_collaborators(&sample_id);
        assert_eq!(collabs.len(), 0u32);
    }

    #[test]
    #[should_panic(expected = "Collaborator share must be positive")]
    fn test_zero_share_collaborator_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let token = dummy_token(&env);
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform, &token));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let collaborators = Vec::from_array(&env, [
            Collaborator { address: Address::generate(&env), share_bps: 0 },
        ]);

        client.upload_sample(
            &producer,
            &String::from_str(&env, "Zero Share"),
            &String::from_str(&env, "QmZeroCID"),
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Jazz"),
            &110u32,
            &collaborators,
        );
    }
}
