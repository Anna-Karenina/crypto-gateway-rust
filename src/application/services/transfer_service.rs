//! # Сервисы переводов
//!
//! Содержит логику для USDT и TRX трансферов

use anyhow::Result;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rust_decimal::Decimal;

use crate::application::dto::*;
use crate::domain::{DomainError, TransactionStatus, TronValidator};
use crate::infrastructure::{
    database::{models::*, schema, DbPool},
    TronGridClient, TronTransactionSigner,
};
use crate::utils::{bigdecimal_to_decimal, decimal_to_bigdecimal};

use super::{SponsorGasService, UnifiedFeeService};

/// Сервис для TRX трансферов (отправка TRX монет)
#[derive(Clone)]
pub struct TrxTransferService {
    tron_client: TronGridClient,
    transaction_signer: TronTransactionSigner,
}

impl TrxTransferService {
    /// Создает новый экземпляр сервиса
    pub fn new(tron_client: TronGridClient) -> Self {
        Self {
            tron_client,
            transaction_signer: TronTransactionSigner::new(),
        }
    }

    /// Отправка TRX с одного адреса на другой
    pub async fn send_trx(
        &self,
        from_address: &str,
        from_private_key: &str,
        to_address: &str,
        amount: Decimal,
    ) -> Result<String> {
        tracing::info!(
            "Отправка {} TRX с {} на {}",
            amount,
            from_address,
            to_address
        );

        // Шаг 1: Создание неподписанной TRX транзакции
        let create_result = self
            .tron_client
            .create_trx_transaction(from_address, to_address, amount)
            .await?;

        tracing::debug!("TRX транзакция создана: {:?}", create_result);

        // Шаг 2: Подписание транзакции
        let signed_transaction = self
            .transaction_signer
            .sign_transaction(&create_result, from_private_key)?;

        // Шаг 3: Отправка транзакции
        let tx_hash = self
            .tron_client
            .broadcast_transaction(&signed_transaction)
            .await?;

        tracing::info!("TRX трансфер успешен. TX Hash: {}", tx_hash);
        Ok(tx_hash)
    }
}

/// Основной сервис для USDT трансферов
pub struct TransferService {
    pub db: DbPool,
    pub tron_client: TronGridClient,
    pub fee_service: UnifiedFeeService,
    pub master_wallet_address: String,
    pub sponsor_gas_service: SponsorGasService,
    pub transaction_signer: TronTransactionSigner,
}

impl TransferService {
    /// Создает новый экземпляр сервиса
    pub fn new(
        db: DbPool,
        tron_client: TronGridClient,
        fee_service: UnifiedFeeService,
        master_wallet_address: String,
        sponsor_gas_service: SponsorGasService,
    ) -> Self {
        Self {
            db,
            tron_client,
            fee_service,
            master_wallet_address,
            sponsor_gas_service,
            transaction_signer: TronTransactionSigner::new(),
        }
    }

    /// Получение трансфера по reference_id
    pub async fn get_transfer_by_reference(
        &self,
        reference_id: &str,
    ) -> anyhow::Result<Option<TransferResponse>> {
        let mut conn = self.db.get().await?;

        let transfer_result: Result<OutgoingTransferModel, diesel::result::Error> =
            schema::outgoing_transfers::table
                .filter(schema::outgoing_transfers::reference_id.eq(reference_id))
                .first(&mut conn)
                .await;

        match transfer_result {
            Ok(transfer) => Ok(Some(self.model_to_response(transfer))),
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    /// Получение всех трансферов кошелька
    pub async fn get_wallet_transfers(
        &self,
        wallet_id: i64,
    ) -> anyhow::Result<Vec<TransferResponse>> {
        let mut conn = self.db.get().await?;

        let transfers: Vec<OutgoingTransferModel> = schema::outgoing_transfers::table
            .filter(schema::outgoing_transfers::from_wallet_id.eq(wallet_id))
            .order(schema::outgoing_transfers::created_at.desc())
            .load(&mut conn)
            .await?;

        let transfer_responses: Vec<TransferResponse> = transfers
            .into_iter()
            .map(|transfer| self.model_to_response(transfer))
            .collect();

        Ok(transfer_responses)
    }

    /// Получение трансфера по хешу транзакции
    pub async fn get_transfer_by_tx_hash(
        &self,
        tx_hash: &str,
    ) -> anyhow::Result<Option<TransferResponse>> {
        let mut conn = self.db.get().await?;

        let transfer_result: Result<OutgoingTransferModel, diesel::result::Error> =
            schema::outgoing_transfers::table
                .filter(schema::outgoing_transfers::tx_hash.eq(tx_hash))
                .first(&mut conn)
                .await;

        match transfer_result {
            Ok(transfer) => Ok(Some(self.model_to_response(transfer))),
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    /// Превью трансфера с расчетом комиссий
    pub async fn preview_transfer(&self, request: TransferRequest) -> Result<TransferPreview> {
        // 1. Проверяем существование кошелька в БД напрямую
        let mut conn = self.db.get().await?;
        let wallet: WalletModel = schema::wallets::table
            .find(request.from_wallet_id)
            .first(&mut conn)
            .await
            .map_err(|_| DomainError::WalletNotFound {
                id: request.from_wallet_id,
            })?;

        // 2. Рассчитываем комиссии (делаем mutable clone для вызова)
        let mut fee_service = self.fee_service.clone();
        let (gas_cost_usdt, percentage_commission, final_commission, total_amount) = fee_service
            .calculate_total_amount(request.order_amount, &wallet.address)
            .await?;

        Ok(TransferPreview {
            order_amount: request.order_amount,
            commission: final_commission,
            gas_cost_in_usdt: gas_cost_usdt,
            percentage_commission,
            total_amount,
            master_wallet_receives: request.order_amount,
            breakdown: format!(
                "Order: {} USDT + Commission: {} USDT (Gas: {} + Service: {}) = Total: {} USDT",
                request.order_amount,
                final_commission,
                gas_cost_usdt,
                percentage_commission,
                total_amount
            ),
            trx_to_usdt_rate: self.fee_service.get_config().trx_to_usdt_rate,
            from_wallet_id: request.from_wallet_id,
            reference_id: request.reference_id,
        })
    }

    /// Получение трансфера по ID
    pub async fn get_transfer(
        &self,
        transfer_id: i64,
    ) -> Result<Option<TransferResponse>, DomainError> {
        let mut conn = self
            .db
            .get()
            .await
            .map_err(|_| DomainError::ConfigurationError {
                message: "Ошибка подключения к БД".to_string(),
            })?;

        let transfer_result = schema::outgoing_transfers::table
            .find(transfer_id)
            .first::<OutgoingTransferModel>(&mut conn)
            .await;

        match transfer_result {
            Ok(transfer) => Ok(Some(self.model_to_response(transfer))),
            Err(diesel::result::Error::NotFound) => {
                Err(DomainError::TransferNotFound { id: transfer_id })
            }
            Err(_) => Err(DomainError::ConfigurationError {
                message: "Ошибка БД".to_string(),
            }),
        }
    }

    /// Создание нового USDT трансфера (сохранение в БД как PENDING)
    pub async fn create_transfer(
        &self,
        request: CreateTransferRequest,
    ) -> Result<TransferResponse> {
        tracing::info!("Создание нового трансфера: {:?}", request);

        // 1. Валидация входных данных
        TronValidator::validate_amount(request.order_amount)
            .map_err(|e| anyhow::anyhow!("Валидация суммы: {}", e))?;

        if let Some(ref_id) = &request.reference_id {
            TronValidator::validate_reference_id(ref_id)
                .map_err(|e| anyhow::anyhow!("Валидация reference_id: {}", e))?;
        }

        // 2. Получаем кошелек отправителя
        let mut conn = self.db.get().await?;
        let wallet: WalletModel = schema::wallets::table
            .find(request.from_wallet_id)
            .first(&mut conn)
            .await
            .map_err(|_| anyhow::anyhow!("Кошелек с ID {} не найден", request.from_wallet_id))?;

        // 3. Проверяем баланс кошелька
        let wallet_balance = self.tron_client.get_usdt_balance(&wallet.address).await?;
        
        // 4. Рассчитываем общую сумму включая комиссии (делаем mutable clone)
        let mut fee_service = self.fee_service.clone();
        let (gas_cost_usdt, percentage_commission, final_commission, total_amount) = fee_service
            .calculate_total_amount(request.order_amount, &wallet.address)
            .await?;

        tracing::info!(
            "Расчет комиссий: газ={} USDT, процент={} USDT, итого={} USDT, общая сумма={} USDT",
            gas_cost_usdt, percentage_commission, final_commission, total_amount
        );

        // 5. Проверяем достаточность баланса
        if wallet_balance < total_amount {
            return Err(anyhow::anyhow!(
                "Недостаточно средств на кошельке {}. Требуется: {} USDT, доступно: {} USDT",
                wallet.address, total_amount, wallet_balance
            ));
        }

        tracing::info!(
            "Проверка баланса прошла успешно: доступно {} USDT, требуется {} USDT",
            wallet_balance, total_amount
        );

        // 6. Создаем новый трансфер в БД со статусом PENDING
        let new_transfer = NewOutgoingTransfer {
            from_wallet_id: request.from_wallet_id,
            to_address: self.master_wallet_address.clone(),
            amount: decimal_to_bigdecimal(request.order_amount),
            status: "PENDING".to_string(),
            reference_id: request.reference_id.clone(),
        };

        let transfer: OutgoingTransferModel =
            diesel::insert_into(schema::outgoing_transfers::table)
                .values(&new_transfer)
                .get_result(&mut conn)
                .await?;

        Ok(TransferResponse {
            id: transfer.id,
            from_wallet_id: transfer.from_wallet_id,
            to_address: transfer.to_address,
            amount: bigdecimal_to_decimal(transfer.amount),
            status: TransactionStatus::Pending,
            tx_hash: transfer.tx_hash,
            reference_id: transfer.reference_id,
            error_message: None,
            created_at: transfer.created_at,
            completed_at: transfer.completed_at,
        })
    }

    /// Обработка pending трансферов
    pub async fn process_pending_transfers(&self) -> Result<()> {
        // Получаем все pending трансферы из БД
        let mut conn = self.db.get().await?;
        let pending_transfers: Vec<OutgoingTransferModel> = schema::outgoing_transfers::table
            .filter(schema::outgoing_transfers::status.eq("PENDING"))
            .order(schema::outgoing_transfers::created_at.asc())
            .load(&mut conn)
            .await?;

        tracing::info!(
            "Обрабатываем {} pending трансферов",
            pending_transfers.len()
        );

        for transfer in pending_transfers {
            match self.process_transfer(&transfer).await {
                Ok(_) => {
                    tracing::info!("Трансфер ID: {} обработан успешно", transfer.id);
                }
                Err(e) => {
                    tracing::error!("Не удалось обработать трансфер ID: {}: {}", transfer.id, e);
                    self.mark_transfer_failed(&transfer, &e.to_string()).await?;
                }
            }
        }
        Ok(())
    }

    /// Обработка одного трансфера
    async fn process_transfer(&self, transfer: &OutgoingTransferModel) -> Result<()> {
        // Получаем кошелек отправителя
        let mut conn = self.db.get().await?;
        let wallet: WalletModel = schema::wallets::table
            .find(transfer.from_wallet_id)
            .first(&mut conn)
            .await?;

        tracing::info!(
            "Обрабатываем трансфер ID: {} с кошелька {} на {}",
            transfer.id,
            wallet.address,
            transfer.to_address
        );

        // Шаг 0: Предварительно заправляем пользовательский кошелек TRX для газа
        tracing::info!(
            "Предварительно заправляем пользовательский кошелек {} TRX для газа",
            wallet.address
        );
        self.sponsor_gas_service
            .ensure_gas_for_transfer(
                &wallet.address,
                bigdecimal_to_decimal(transfer.amount.clone()),
            )
            .await?;

        // Ждем немного для подтверждения TRX транзакции
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Шаг 1: Создаем неподписанную USDT транзакцию
        let tx_result = self
            .tron_client
            .create_trc20_transaction(
                &wallet.address,
                &transfer.to_address,
                bigdecimal_to_decimal(transfer.amount.clone()),
            )
            .await?;

        // Шаг 2: Подписываем транзакцию
        let signed_transaction = self
            .transaction_signer
            .sign_transaction(&tx_result, &wallet.private_key)?;

        // Шаг 3: Отправляем транзакцию
        let tx_hash = self
            .tron_client
            .broadcast_transaction(&signed_transaction)
            .await?;

        // Помечаем трансфер как подтвержденный и сохраняем хеш транзакции
        self.mark_transfer_completed(transfer, &tx_hash).await?;

        tracing::info!(
            "Трансфер ID: {} завершен успешно. TX Hash: {}",
            transfer.id,
            tx_hash
        );

        Ok(())
    }

    /// Помечает трансфер как завершенный
    async fn mark_transfer_completed(
        &self,
        transfer: &OutgoingTransferModel,
        tx_hash: &str,
    ) -> Result<()> {
        let mut conn = self.db.get().await?;

        diesel::update(schema::outgoing_transfers::table.find(transfer.id))
            .set((
                schema::outgoing_transfers::status.eq("CONFIRMED"),
                schema::outgoing_transfers::tx_hash.eq(tx_hash),
                schema::outgoing_transfers::completed_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    /// Помечает трансфер как неудачный
    async fn mark_transfer_failed(
        &self,
        transfer: &OutgoingTransferModel,
        error_message: &str,
    ) -> Result<()> {
        let mut conn = self.db.get().await?;

        diesel::update(schema::outgoing_transfers::table.find(transfer.id))
            .set((
                schema::outgoing_transfers::status.eq("FAILED"),
                schema::outgoing_transfers::error_message.eq(error_message),
                schema::outgoing_transfers::completed_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    /// Конвертирует модель в ответ
    fn model_to_response(&self, transfer: OutgoingTransferModel) -> TransferResponse {
        TransferResponse {
            id: transfer.id,
            from_wallet_id: transfer.from_wallet_id,
            to_address: transfer.to_address,
            amount: bigdecimal_to_decimal(transfer.amount),
            status: match transfer.status.as_str() {
                "PENDING" => TransactionStatus::Pending,
                "CONFIRMED" => TransactionStatus::Completed,
                "COMPLETED" => TransactionStatus::Completed,
                "FAILED" => TransactionStatus::Failed,
                _ => TransactionStatus::Pending, // для неизвестных статусов
            },
            tx_hash: transfer.tx_hash,
            reference_id: transfer.reference_id,
            error_message: transfer.error_message,
            created_at: transfer.created_at,
            completed_at: transfer.completed_at,
        }
    }
}
