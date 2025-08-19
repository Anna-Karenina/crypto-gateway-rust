//! # Сервис спонсорства газа
//!
//! Автоматическая отправка TRX для покрытия газовых расходов пользователей

use anyhow::Result;
use rust_decimal::Decimal;

use crate::infrastructure::tron::TronGridClient;

use super::TrxTransferService;

/// Сервис спонсорства газа для пользовательских кошельков
/// Автоматически отправляет TRX с master wallet на пользовательские кошельки при необходимости
pub struct SponsorGasService {
    tron_client: TronGridClient,
    trx_transfer_service: TrxTransferService,
    min_trx_amount: Decimal,
    sponsor_enabled: bool,
    master_wallet_address: String,
    master_wallet_private_key: String,
}

impl SponsorGasService {
    /// Создает новый экземпляр сервиса
    pub fn new(
        tron_client: TronGridClient,
        trx_transfer_service: TrxTransferService,
        min_trx_amount: Decimal,
        sponsor_enabled: bool,
        master_wallet_address: String,
        master_wallet_private_key: String,
    ) -> Self {
        Self {
            tron_client,
            trx_transfer_service,
            min_trx_amount,
            sponsor_enabled,
            master_wallet_address,
            master_wallet_private_key,
        }
    }

    /// Обеспечивает наличие TRX для USDT трансфера (ВСЕГДА спонсирует газ)
    pub async fn ensure_gas_for_transfer(
        &self,
        wallet_address: &str,
        _transfer_amount: Decimal,
    ) -> Result<()> {
        if !self.sponsor_enabled {
            tracing::debug!("Gas sponsorship отключен");
            return Ok(());
        }

        tracing::debug!(
            "Обеспечиваем газ для кошелька {} (минимум {} TRX)",
            wallet_address,
            self.min_trx_amount
        );

        // Проверяем текущий TRX баланс для логирования
        let trx_balance = self.tron_client.get_trx_balance(wallet_address).await?;
        tracing::info!(
            "Текущий TRX баланс для кошелька {}: {} TRX",
            wallet_address,
            trx_balance
        );

        // ВСЕГДА спонсируем газ с master wallet независимо от текущего баланса
        tracing::info!(
            "Спонсируем {} TRX для кошелька {} (политика: всегда спонсировать с master wallet)",
            self.min_trx_amount,
            wallet_address
        );

        match self
            .trx_transfer_service
            .send_trx(
                &self.master_wallet_address,
                &self.master_wallet_private_key,
                wallet_address,
                self.min_trx_amount,
            )
            .await
        {
            Ok(tx_hash) => {
                tracing::info!(
                    "Успешно спонсировали {} TRX для кошелька {}, TX: {}",
                    self.min_trx_amount,
                    wallet_address,
                    tx_hash
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Не удалось спонсировать газ для кошелька {}: {}",
                    wallet_address,
                    e
                );
                // Не прерываем выполнение - позволяем оригинальному трансферу продолжиться
                Ok(())
            }
        }
    }
}
