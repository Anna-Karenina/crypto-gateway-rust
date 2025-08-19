use anyhow::Result;
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub grpc: GrpcConfig,
    pub database: DatabaseConfig,
    pub tron: TronConfig,
    pub wallet: WalletConfig,
    pub fees: FeeConfig,
    pub gas_sponsorship: GasSponsorshipConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrpcConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GasSponsorshipConfig {
    pub enabled: bool,
    pub min_trx_amount: rust_decimal::Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TronConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub usdt_contract: String,
    pub usdt_decimals: u8,
    pub master_wallet_address: String,
    pub master_wallet_private_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalletConfig {
    pub use_real_generator: bool,
    pub activation: WalletActivationConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalletActivationConfig {
    pub enabled: bool,
    pub amount: rust_decimal::Decimal, // Количество TRX для активации
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeeConfig {
    pub trx_per_transaction: rust_decimal::Decimal,
    pub trx_to_usdt_rate: rust_decimal::Decimal,
    pub commission_percentage: rust_decimal::Decimal,
    pub min_commission_usdt: rust_decimal::Decimal,
    pub max_commission_usdt: rust_decimal::Decimal,
}

impl Settings {
    /// Загружает конфигурацию из config.toml и переменных окружения
    pub fn new() -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Загружаем из config.toml (с подстановкой переменных окружения)
            .add_source(File::with_name("config").required(false))
            // Переопределяем из переменных окружения с префиксом APP
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        config.try_deserialize()
    }

    /// Загружает конфигурацию только из переменных окружения (.env style)
    pub fn from_env() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(Environment::default())
            .build()?;

        config.try_deserialize()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                cors_enabled: true,
            },
            grpc: GrpcConfig {
                enabled: true,
                host: "0.0.0.0".to_string(),
                port: 50051,
            },
            database: DatabaseConfig {
                url: "postgresql://postgres:postgres123@localhost:5432/tron_gateway".to_string(),
                max_connections: 10,
            },
            tron: TronConfig {
                base_url: "https://api.shasta.trongrid.io".to_string(), // Testnet для разработки
                api_key: None,
                usdt_contract: "TG3XXyExBkPp9nzdajDZsozEu4BkaSJozs".to_string(), // Testnet USDT
                usdt_decimals: 6,
                master_wallet_address: "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3".to_string(), // Из .env
                master_wallet_private_key:
                    "df319c4fe709ad6a9f32b07ada986f4055708f4e064e5ff6cab439561a6eae59".to_string(), // Из .env
            },
            wallet: WalletConfig {
                use_real_generator: true,
                activation: WalletActivationConfig {
                    enabled: true,
                    amount: rust_decimal::Decimal::new(10, 1), // 1.0 TRX
                },
            },
            fees: FeeConfig {
                trx_per_transaction: rust_decimal::Decimal::new(15, 0), // 15 TRX
                trx_to_usdt_rate: rust_decimal::Decimal::new(10, 2),    // 0.10 USDT
                commission_percentage: rust_decimal::Decimal::new(5, 1), // 0.5%
                min_commission_usdt: rust_decimal::Decimal::new(10, 1), // 1.0 USDT
                max_commission_usdt: rust_decimal::Decimal::new(100, 1), // 10.0 USDT
            },
            gas_sponsorship: GasSponsorshipConfig {
                enabled: true,
                min_trx_amount: rust_decimal::Decimal::new(15, 0), // 15.0 TRX
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
        }
    }
}
