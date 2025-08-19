//! # Сервис активации кошельков
//! 
//! Автоматическая активация новых кошельков отправкой TRX

use anyhow::Result;
use rust_decimal::Decimal;



use super::TrxTransferService;

/// Сервис автоматической активации кошельков (отправка TRX для активации в сети TRON)
pub struct WalletActivationService {
    trx_transfer_service: TrxTransferService,
    master_wallet_address: String,
    master_wallet_private_key: String,
    activation_amount: Decimal,
    auto_activation_enabled: bool,
}

impl WalletActivationService {
    /// Создает новый экземпляр сервиса
    pub fn new(
        trx_transfer_service: TrxTransferService,
        master_wallet_address: String,
        master_wallet_private_key: String,
        activation_amount: Decimal,
        auto_activation_enabled: bool,
    ) -> Self {
        Self {
            trx_transfer_service,
            master_wallet_address,
            master_wallet_private_key,
            activation_amount,
            auto_activation_enabled,
        }
    }

    /// Активация кошелька отправкой TRX с мастер-кошелька
    pub async fn activate_wallet(&self, wallet_address: &str) -> Result<bool> {
        if !self.auto_activation_enabled {
            tracing::info!(
                "Автоактивация кошельков отключена. Пропускаем активацию для кошелька: {}",
                wallet_address
            );
            return Ok(false);
        }

        tracing::info!(
            "Активация кошелька {} отправкой {} TRX с мастер-кошелька {}",
            wallet_address,
            self.activation_amount,
            self.master_wallet_address
        );

        match self
            .trx_transfer_service
            .send_trx(
                &self.master_wallet_address,
                &self.master_wallet_private_key,
                wallet_address,
                self.activation_amount,
            )
            .await
        {
            Ok(tx_hash) => {
                tracing::info!(
                    "Кошелек {} активирован успешно. Activation TX Hash: {}",
                    wallet_address,
                    tx_hash
                );
                Ok(true)
            }
            Err(e) => {
                tracing::error!("Не удалось активировать кошелек {}: {}", wallet_address, e);
                Ok(false)
            }
        }
    }

    /// Получение суммы активации
    pub fn get_activation_amount(&self) -> Decimal {
        self.activation_amount
    }
}
