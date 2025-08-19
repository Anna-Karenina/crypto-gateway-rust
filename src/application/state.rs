// Состояние приложения для dependency injection

use std::sync::Arc;

use crate::application::services::{
    FeeCalculationService, SponsorGasService, TransferService, TrxTransferService,
    WalletActivationService, WalletService,
};
use crate::config::Settings;
use crate::infrastructure::{database::create_db_pool, TronGridClient, TronWalletGenerator};

/// Состояние приложения с всеми сервисами
#[derive(Clone)]
pub struct AppState {
    pub wallet_service: Arc<WalletService>,
    pub transfer_service: Arc<TransferService>,
    pub fee_service: Arc<FeeCalculationService>,
}

impl AppState {
    /// Создание нового состояния приложения
    pub async fn new(settings: Settings) -> anyhow::Result<Self> {
        // 1. Создаем пул соединений с БД
        let db_pool = create_db_pool(&settings.database.url).await?;

        // 2. Создаем TRON клиент
        let tron_client = TronGridClient::new(settings.tron.clone());

        // 3. Создаем генератор кошельков
        let wallet_generator = TronWalletGenerator::new();

        // 4. Создаем сервисы
        let fee_service = FeeCalculationService::new(
            tron_client.clone(),
            settings.fees.trx_per_transaction,
            settings.fees.trx_to_usdt_rate,
            settings.fees.commission_percentage,
            settings.fees.min_commission_usdt,
            settings.fees.max_commission_usdt,
            settings.tron.master_wallet_address.clone(),
        );

        // 5. Создаем TRX transfer service для активации кошельков
        let trx_transfer_service = TrxTransferService::new(tron_client.clone());

        // 6. Создаем wallet activation service (если включен в конфиге)
        let wallet_activation_service = if settings.wallet.activation.enabled {
            Some(WalletActivationService::new(
                trx_transfer_service.clone(),
                settings.tron.master_wallet_address.clone(),
                settings.tron.master_wallet_private_key.clone(),
                settings.wallet.activation.amount,
                settings.wallet.activation.enabled,
            ))
        } else {
            None
        };

        let wallet_service = WalletService::new(
            db_pool.clone(),
            tron_client.clone(),
            wallet_generator,
            wallet_activation_service,
        );

        // 7. Создаем sponsor gas service для автоматической отправки TRX для газа
        let sponsor_gas_service = SponsorGasService::new(
            tron_client.clone(),
            trx_transfer_service.clone(),
            rust_decimal::Decimal::new(15, 0), // 15.0 TRX
            true,                              // включен по умолчанию
            settings.tron.master_wallet_address.clone(),
            settings.tron.master_wallet_private_key.clone(),
        );

        let transfer_service = TransferService::new(
            db_pool,
            tron_client,
            fee_service.clone(),
            settings.tron.master_wallet_address.clone(),
            sponsor_gas_service,
        );

        Ok(Self {
            wallet_service: Arc::new(wallet_service),
            transfer_service: Arc::new(transfer_service),
            fee_service: Arc::new(fee_service),
        })
    }
}
