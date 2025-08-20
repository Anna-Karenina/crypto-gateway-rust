//! # Сервис мониторинга транзакций
//!
//! Отслеживает входящие USDT транзакции для всех кошельков

use anyhow::Result;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rust_decimal::Decimal;
use std::collections::HashSet;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::domain::{BlockchainTransaction, TransactionStatus};
use crate::infrastructure::database::{
    models::{IncomingTransactionModel, NewIncomingTransaction, WalletModel},
    schema, DbPool,
};
use crate::infrastructure::tron::TronGridClient;
use crate::utils::conversions::decimal_to_bigdecimal;

/// Сервис для мониторинга входящих транзакций
#[derive(Clone)]
pub struct TransactionMonitoringService {
    db: DbPool,
    tron_client: TronGridClient,
    usdt_contract: String,
    monitoring_enabled: bool,
}

impl TransactionMonitoringService {
    /// Создает новый экземпляр сервиса
    pub fn new(
        db: DbPool,
        tron_client: TronGridClient,
        usdt_contract: String,
        monitoring_enabled: bool,
    ) -> Self {
        Self {
            db,
            tron_client,
            usdt_contract,
            monitoring_enabled,
        }
    }

    /// Запускает фоновый мониторинг входящих транзакций
    pub async fn start_monitoring(&self) -> Result<()> {
        if !self.monitoring_enabled {
            info!("Мониторинг транзакций отключен в конфигурации");
            return Ok(());
        }

        info!("🔍 Запуск мониторинга входящих USDT транзакций...");

        let mut monitoring_interval = interval(Duration::from_secs(30)); // Каждые 30 секунд

        loop {
            monitoring_interval.tick().await;

            if let Err(e) = self.scan_for_incoming_transactions().await {
                error!("Ошибка сканирования входящих транзакций: {}", e);
                // Продолжаем работу, не падаем
            }
        }
    }

    /// Сканирует входящие транзакции для всех кошельков (публичный метод)
    pub async fn scan_for_incoming_transactions(&self) -> Result<()> {
        info!("🔍 Сканирование входящих транзакций...");

        // 1. Получаем все активные кошельки
        let mut conn = self.db.get().await?;
        let wallets: Vec<WalletModel> = schema::wallets::table
            .select(WalletModel::as_select())
            .load(&mut conn)
            .await?;

        info!("Найдено {} кошельков для мониторинга", wallets.len());

        // 2. Для каждого кошелька проверяем входящие транзакции
        for wallet in wallets {
            if let Err(e) = self.scan_wallet_transactions(&wallet).await {
                warn!("Ошибка сканирования кошелька {}: {}", wallet.address, e);
                // Продолжаем с другими кошельками
            }
        }

        Ok(())
    }

    /// Сканирует транзакции для конкретного кошелька
    async fn scan_wallet_transactions(&self, wallet: &WalletModel) -> Result<()> {
        // Получаем последние 50 транзакций для кошелька
        let transactions = self
            .tron_client
            .get_trc20_transactions(&wallet.address, &self.usdt_contract, 50)
            .await?;

        info!(
            "Найдено {} транзакций для кошелька {}",
            transactions.len(),
            wallet.address
        );

        // Получаем уже известные tx_hash из БД
        let mut conn = self.db.get().await?;
        let known_hashes: Vec<String> = schema::incoming_transactions::table
            .filter(schema::incoming_transactions::wallet_id.eq(wallet.id))
            .select(schema::incoming_transactions::tx_hash)
            .load(&mut conn)
            .await?;

        let known_hashes: HashSet<String> = known_hashes.into_iter().collect();

        // Обрабатываем новые транзакции
        for tx in transactions {
            if !known_hashes.contains(&tx.tx_hash) {
                // Проверяем, что это входящая транзакция (to = наш кошелек)
                if tx.to_address.eq_ignore_ascii_case(&wallet.address) {
                    if let Err(e) = self.process_new_incoming_transaction(wallet, &tx).await {
                        error!("Ошибка обработки новой транзакции {}: {}", tx.tx_hash, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Обрабатывает новую входящую транзакцию
    async fn process_new_incoming_transaction(
        &self,
        wallet: &WalletModel,
        tx: &BlockchainTransaction,
    ) -> Result<()> {
        info!(
            "📥 Новая входящая транзакция: {} USDT на кошелек {} (tx: {})",
            tx.amount, wallet.address, tx.tx_hash
        );

        let mut conn = self.db.get().await?;

        // Определяем статус на основе количества подтверждений
        let status = if tx.confirmations >= 19 {
            // 19+ подтверждений = окончательно подтвержден
            TransactionStatus::Completed
        } else if tx.confirmations >= 1 {
            // 1+ подтверждений = в процессе подтверждения
            TransactionStatus::Processing
        } else {
            // 0 подтверждений = в ожидании
            TransactionStatus::Pending
        };

        // Сохраняем в БД
        let new_transaction = NewIncomingTransaction {
            wallet_id: wallet.id,
            tx_hash: tx.tx_hash.clone(),
            block_number: tx.block_number,
            from_address: tx.from_address.clone(),
            to_address: tx.to_address.clone(),
            amount: decimal_to_bigdecimal(tx.amount),
            status: format!("{:?}", status),
            error_message: None,
        };

        diesel::insert_into(schema::incoming_transactions::table)
            .values(&new_transaction)
            .execute(&mut conn)
            .await?;

        // Если транзакция подтверждена, обновляем баланс кошелька
        if status == TransactionStatus::Completed {
            self.update_wallet_balance(wallet.id, tx.amount).await?;

            info!(
                "✅ Баланс кошелька {} пополнен на {} USDT",
                wallet.address, tx.amount
            );
        }

        Ok(())
    }

    /// Обновляет баланс кошелька (заглушка - баланс вычисляется динамически)
    async fn update_wallet_balance(&self, _wallet_id: i64, _amount: Decimal) -> Result<()> {
        // В текущей реализации баланс вычисляется динамически через TronGrid API
        // Здесь можно добавить кэширование баланса в БД если нужно
        Ok(())
    }

    /// Получает все входящие транзакции для кошелька
    pub async fn get_wallet_incoming_transactions(
        &self,
        wallet_id: i64,
    ) -> Result<Vec<IncomingTransactionModel>> {
        let mut conn = self.db.get().await?;

        let transactions = schema::incoming_transactions::table
            .filter(schema::incoming_transactions::wallet_id.eq(wallet_id))
            .order(schema::incoming_transactions::detected_at.desc())
            .load(&mut conn)
            .await?;

        Ok(transactions)
    }

    /// Получает сводку по входящим транзакциям
    pub async fn get_monitoring_stats(&self) -> Result<MonitoringStats> {
        let mut conn = self.db.get().await?;

        // Общее количество обработанных транзакций
        let total_transactions: i64 = schema::incoming_transactions::table
            .count()
            .get_result(&mut conn)
            .await?;

        // Количество транзакций по статусам
        let pending_count: i64 = schema::incoming_transactions::table
            .filter(schema::incoming_transactions::status.eq("Pending"))
            .count()
            .get_result(&mut conn)
            .await?;

        let processing_count: i64 = schema::incoming_transactions::table
            .filter(schema::incoming_transactions::status.eq("Processing"))
            .count()
            .get_result(&mut conn)
            .await?;

        let completed_count: i64 = schema::incoming_transactions::table
            .filter(schema::incoming_transactions::status.eq("Completed"))
            .count()
            .get_result(&mut conn)
            .await?;

        Ok(MonitoringStats {
            total_transactions,
            pending_count,
            processing_count,
            completed_count,
            monitoring_enabled: self.monitoring_enabled,
        })
    }
}

/// Статистика мониторинга
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitoringStats {
    pub total_transactions: i64,
    pub pending_count: i64,
    pub processing_count: i64,
    pub completed_count: i64,
    pub monitoring_enabled: bool,
}
