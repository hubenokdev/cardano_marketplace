use envconfig::Envconfig;

#[derive(Envconfig, Debug, Clone)]
pub struct Config {
    #[envconfig(from = "IS_TESTNET")]
    pub is_testnet: bool,

    #[envconfig(from = "SUBMIT_API_BASE_URL")]
    pub submit_api_base_url: String,

    #[envconfig(from = "PORT")]
    pub port: u32,

    #[envconfig(from = "NFT_BECH32_TAXATION_ADDRESS")]
    pub nft_bech32_tax_address: String,

    #[envconfig(from = "DATABASE_URL")]
    pub database_url: String,

    #[envconfig(from = "MARKETPLACE_PRIVATE_KEY_FILE")]
    pub marketplace_private_key_file: String,

    #[envconfig(from = "MARKETPLACE_REVENUE_ADDRESS")]
    pub marketplace_revenue_address: String,

    #[envconfig(from = "PROJECTS_PRIVATE_KEY_FILE")]
    pub projects_private_key_file: String,

    #[envconfig(from = "PROJECTS_REVENUE_ADDRESS")]
    pub projects_revenue_address: String,
}
