//! # Сервис управления кошельками
//!
//! Основной сервис для создания, получения и управления TRON кошельками

use anyhow::Result;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rust_decimal::Decimal;

use crate::application::dto::WalletResponse;
use crate::domain::DomainError;
use crate::infrastructure::{
    database::{models::*, schema, DbPool},
    TronGridClient, TronWalletGenerator,
};

use super::WalletActivationService;

/// Сервис для работы с кошельками
pub struct WalletService {
    db: DbPool,
    tron_client: TronGridClient,
    wallet_generator: TronWalletGenerator,
    wallet_activation_service: Option<WalletActivationService>,
}

impl WalletService {
    /// Создает новый экземпляр сервиса
    pub fn new(
        db: DbPool,
        tron_client: TronGridClient,
        wallet_generator: TronWalletGenerator,
        wallet_activation_service: Option<WalletActivationService>,
    ) -> Self {
        Self {
            db,
            tron_client,
            wallet_generator,
            wallet_activation_service,
        }
    }

    /// Создание нового кошелька с автоматической активацией
    pub async fn create_wallet(
        &self,
        owner_id: Option<String>,
    ) -> Result<WalletResponse, DomainError> {
        // 1. Генерируем TRON кошелек
        let (address, hex_address, private_key) =
            self.wallet_generator.generate_wallet().map_err(|e| {
                DomainError::ConfigurationError {
                    message: format!("Ошибка генерации кошелька: {}", e),
                }
            })?;

        // 2. Создаем запись в БД
        let new_wallet = NewWallet {
            address: address.clone(),
            hex_address,
            private_key: private_key.clone(),
            owner_id: owner_id.clone(),
        };

        let mut conn = self
            .db
            .get()
            .await
            .map_err(|_| DomainError::ConfigurationError {
                message: "Ошибка подключения к БД".to_string(),
            })?;

        let wallet: WalletModel = diesel::insert_into(schema::wallets::table)
            .values(&new_wallet)
            .get_result(&mut conn)
            .await
            .map_err(|_| DomainError::ConfigurationError {
                message: "Ошибка создания кошелька в БД".to_string(),
            })?;

        // 3. Автоматическая активация кошелька (если включена)
        if let Some(ref activation_service) = self.wallet_activation_service {
            activation_service.activate_wallet(&wallet.address).await
                .map_err(|e| DomainError::ConfigurationError {
                    message: format!("Ошибка активации кошелька: {}", e),
                })?;
        }

        Ok(WalletResponse {
            id: wallet.id,
            address: wallet.address,
            owner_id: wallet.owner_id,
            created_at: wallet.created_at,
            balance: Some(Decimal::ZERO), // Новый кошелек имеет нулевой баланс
        })
    }

    /// Получение кошелька по ID
    pub async fn get_wallet(&self, wallet_id: i64) -> Result<Option<WalletResponse>, DomainError> {
        let mut conn = self
            .db
            .get()
            .await
            .map_err(|_| DomainError::ConfigurationError {
                message: "Ошибка подключения к БД".to_string(),
            })?;

        let wallet_result = schema::wallets::table
            .find(wallet_id)
            .first::<WalletModel>(&mut conn)
            .await;

        match wallet_result {
            Ok(wallet) => {
                // Получаем баланс
                let (usdt_balance, _trx_balance) = self
                    .get_wallet_balance(wallet_id)
                    .await
                    .unwrap_or((Decimal::ZERO, Decimal::ZERO));

                Ok(Some(WalletResponse {
                    id: wallet.id,
                    address: wallet.address,
                    owner_id: wallet.owner_id,
                    created_at: wallet.created_at,
                    balance: Some(usdt_balance),
                }))
            }
            Err(diesel::result::Error::NotFound) => {
                Err(DomainError::WalletNotFound { id: wallet_id })
            }
            Err(_) => Err(DomainError::ConfigurationError {
                message: "Ошибка БД".to_string(),
            }),
        }
    }

    /// Получение баланса кошелька через TRON API
    pub async fn get_wallet_balance(&self, wallet_id: i64) -> Result<(Decimal, Decimal)> {
        // 1. Получаем адрес кошелька из БД
        let mut conn = self.db.get().await?;
        let wallet: WalletModel = schema::wallets::table
            .find(wallet_id)
            .first(&mut conn)
            .await?;

        // 2. Запрашиваем баланс через TRON API
        let usdt_balance = self.tron_client.get_usdt_balance(&wallet.address).await?;
        let trx_balance = self.tron_client.get_trx_balance(&wallet.address).await?;

        Ok((usdt_balance, trx_balance))
    }

    /// Получение баланса кошелька по адресу (для мастер-кошелька и др.)
    pub async fn get_wallet_balance_by_address(&self, address: &str) -> Result<(Decimal, Decimal)> {
        // Получаем реальные балансы из TRON сети
        let usdt_balance = self.tron_client.get_usdt_balance(address).await?;
        let trx_balance = self.tron_client.get_trx_balance(address).await?;

        Ok((usdt_balance, trx_balance))
    }

    /// Ручная активация кошелька (отправка TRX)
    pub async fn activate_wallet_by_address(&self, address: &str) -> Result<bool, DomainError> {
        // Проверяем что кошелек существует в нашей БД
        let wallet = self.get_wallet_by_address(address).await?;
        if wallet.is_none() {
            return Err(DomainError::WalletNotFoundByAddress {
                address: address.to_string(),
            });
        }

        // Если есть сервис активации - используем его
        if let Some(ref activation_service) = self.wallet_activation_service {
            match activation_service.activate_wallet(address).await {
                Ok(success) => Ok(success),
                Err(e) => {
                    tracing::error!("Ошибка активации кошелька {}: {}", address, e);
                    Err(DomainError::ConfigurationError {
                        message: format!("Не удалось активировать кошелек: {}", e),
                    })
                }
            }
        } else {
            // Если сервис активации отключен
            tracing::warn!(
                "Попытка активации кошелька {}, но сервис активации отключен",
                address
            );
            Err(DomainError::ConfigurationError {
                message: "Сервис активации кошельков отключен".to_string(),
            })
        }
    }

    /// Получение кошелька по адресу
    pub async fn get_wallet_by_address(
        &self,
        address: &str,
    ) -> Result<Option<WalletModel>, DomainError> {
        let mut conn = self
            .db
            .get()
            .await
            .map_err(|_| DomainError::ConfigurationError {
                message: "Ошибка подключения к БД".to_string(),
            })?;

        let wallet_result = schema::wallets::table
            .filter(schema::wallets::address.eq(address))
            .first::<WalletModel>(&mut conn)
            .await;

        match wallet_result {
            Ok(wallet) => Ok(Some(wallet)),
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(_) => Err(DomainError::ConfigurationError {
                message: "Ошибка БД".to_string(),
            }),
        }
    }
}
