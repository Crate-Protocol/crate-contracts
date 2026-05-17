#[cfg(test)]
mod tests {
    use crate::{SampledMarketplace, SampledMarketplaceClient};
    use soroban_sdk::{
        testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
        token, Address, Env, IntoVal, String,
    };

    // Helper: create a native XLM SAC and mint tokens to an address
    fn create_xlm_token(
        env: &Env,
        admin: &Address,
    ) -> (Address, token::Client) {
        let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
        let token_client = token::Client::new(env, &contract_address.address());
        let token_admin = token::StellarAssetClient::new(env, &contract_address.address());
        token_admin.mint(admin, &1_000_000_000_000i128);
        (contract_address.address(), token_client)
    }

    #[test]
    fn test_upload_and_get_sample() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(SampledMarketplace, (&1000u32, &platform));
        let client = SampledMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let title = String::from_str(&env, "Lo-Fi Beat #1");
        let cid = String::from_str(&env, "QmTestCID123");
        let genre = String::from_str(&env, "Lo-Fi");

        let sample_id = client.upload_sample(&producer, &title, &cid, &10i128, &genre, &95u32);
        assert_eq!(sample_id, 1u64);

        let sample = client.get_sample(&sample_id);
        assert_eq!(sample.title, title);
        assert_eq!(sample.price, 100_000_000i128); // 10 XLM in stroops
        assert_eq!(sample.bpm, 95u32);
        assert!(sample.active);
        assert_eq!(sample.sales_count, 0u64);
    }

    #[test]
    fn test_get_stats_initial() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(SampledMarketplace, (&1000u32, &platform));
        let client = SampledMarketplaceClient::new(&env, &contract_id);

        let (total_samples, total_volume) = client.get_stats();
        assert_eq!(total_samples, 0u64);
        assert_eq!(total_volume, 0i128);
    }

    #[test]
    fn test_get_earnings_zero_initial() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(SampledMarketplace, (&1000u32, &platform));
        let client = SampledMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        assert_eq!(client.get_earnings(&producer), 0i128);
    }

    #[test]
    fn test_multiple_uploads_increment_counter() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(SampledMarketplace, (&1000u32, &platform));
        let client = SampledMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);

        let id1 = client.upload_sample(
            &producer,
            &String::from_str(&env, "Beat 1"),
            &String::from_str(&env, "QmCID1"),
            &5i128,
            &String::from_str(&env, "Hip-Hop"),
            &90u32,
        );
        let id2 = client.upload_sample(
            &producer,
            &String::from_str(&env, "Beat 2"),
            &String::from_str(&env, "QmCID2"),
            &15i128,
            &String::from_str(&env, "Trap"),
            &140u32,
        );

        assert_eq!(id1, 1u64);
        assert_eq!(id2, 2u64);

        let (total_samples, _) = client.get_stats();
        assert_eq!(total_samples, 2u64);
    }

    #[test]
    fn test_delist_sample() {
        let env = Env::default();
        env.mock_all_auths();

        let platform = Address::generate(&env);
        let contract_id = env.register(SampledMarketplace, (&1000u32, &platform));
        let client = SampledMarketplaceClient::new(&env, &contract_id);

        let producer = Address::generate(&env);
        let sample_id = client.upload_sample(
            &producer,
            &String::from_str(&env, "My Beat"),
            &String::from_str(&env, "QmTestCID"),
            &10i128,
            &String::from_str(&env, "R&B"),
            &80u32,
        );

        client.delist_sample(&producer, &sample_id);

        let sample = client.get_sample(&sample_id);
        assert!(!sample.active);
    }
}
