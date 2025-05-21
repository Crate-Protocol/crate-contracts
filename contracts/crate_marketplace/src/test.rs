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
            &50_000_000i128,
            &250_000_000i128,
            &1_000_000_000i128,
            &String::from_str(&env, "Hip-Hop"),
            &90u32,
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
        );

        assert_eq!(id1, 1u32);
        assert_eq!(id2, 2u32);

        let (total_samples, _) = client.get_stats();
        assert_eq!(total_samples, 2u32);
    }

    #[test]
    fn test_delist_sample_removes_record() {
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
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "R&B"),
            &80u32,
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
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
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
        );

        client.delist_sample(&producer, &sample_id);
        client.get_sample(&sample_id);
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
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Trap"),
            &140u32,
        );

        client.purchase_license(&buyer, &sample_id, &xlm_addr, &LicenseTier::Lease);

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
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let buyer = Address::generate(&env);

        let (xlm_addr, _xlm_client) = create_xlm_token(&env, &buyer);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Exclusive Only"),
            &String::from_str(&env, "QmExclusiveCID"),
            &100_000_000i128,
            &500_000_000i128,
            &5_000_000_000i128,
            &String::from_str(&env, "Drill"),
            &135u32,
        );

        client.purchase_license(&buyer, &sample_id, &xlm_addr, &LicenseTier::Exclusive);

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
        let contract_id = env.register(CrateMarketplace, (&1000u32, &platform));
        let client = CrateMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let buyer = Address::generate(&env);

        let (_xlm_addr, xlm_client) = create_xlm_token(&env, &buyer);

        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "Withdraw Test"),
            &String::from_str(&env, "QmWithdrawCID"),
            &200_000_000i128,
            &1_000_000_000i128,
            &4_000_000_000i128,
            &String::from_str(&env, "Hip-Hop"),
            &90u32,
        );

        client.purchase_license(&buyer, &sample_id, &_xlm_addr, &LicenseTier::Lease);

        assert_eq!(client.get_earnings(&producer), 180_000_000i128);

        let withdrawn = client.withdraw_earnings(&producer, &_xlm_addr);
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
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "Afrobeats"),
            &95u32,
        );

        let (s0, v0) = client.get_stats();
        assert_eq!(s0, 1u32);
        assert_eq!(v0, 0i128);

        client.purchase_license(&buyer, &sample_id, &_xlm_addr, &LicenseTier::Premium);

        let (s1, v1) = client.get_stats();
        assert_eq!(s1, 1u32);
        assert_eq!(v1, 500_000_000i128);
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
            &100_000_000i128,
            &500_000_000i128,
            &2_000_000_000i128,
            &String::from_str(&env, "R&B"),
            &80u32,
        );

        assert_eq!(client.get_license(&buyer, &sample_id), None);

        client.purchase_license(&buyer, &sample_id, &xlm_addr, &LicenseTier::Premium);

        assert_eq!(client.get_license(&buyer, &sample_id), Some(LicenseTier::Premium));
    }
}
