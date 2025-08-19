use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::TransactionStatus;

/// DTO для запроса создания кошелька
#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    /// Идентификатор владельца кошелька (опционально)
    pub owner_id: Option<String>,
}

/// DTO для создания трансфера (TransferRequestDto)
#[derive(Debug, Deserialize)]
pub struct CreateTransferRequest {
    /// ID кошелька отправителя
    pub from_wallet_id: i64,
    /// Сумма заказа в USDT (без комиссии)
    pub order_amount: Decimal,
    /// Референс для связи с внешней системой
    pub reference_id: Option<String>,
    /// Если true, показать только preview без создания трансфера
    pub preview_only: Option<bool>,
}

/// DTO для ответа с информацией о кошельке
#[derive(Debug, Serialize)]
pub struct WalletResponse {
    pub id: i64,
    pub address: String,
    pub owner_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub balance: Option<Decimal>, // Баланс может быть недоступен сразу
}

/// DTO для запроса трансфера
#[derive(Debug, Clone, Deserialize)]
pub struct TransferRequest {
    /// ID кошелька отправителя
    pub from_wallet_id: i64,
    /// Сумма заказа в USDT (без комиссии)
    pub order_amount: Decimal,
    /// Референс для связи с внешней системой
    pub reference_id: Option<String>,
}

/// DTO для превью трансфера (TransferPreviewDto)
#[derive(Debug, Serialize)]
pub struct TransferPreview {
    /// Сумма заказа
    pub order_amount: Decimal,
    /// Общая комиссия
    pub commission: Decimal,
    /// Газовые расходы в USDT эквиваленте
    pub gas_cost_in_usdt: Decimal,
    /// Процентная комиссия
    pub percentage_commission: Decimal,
    /// Общая сумма к списанию
    pub total_amount: Decimal,
    /// Сумма получаемая master wallet
    pub master_wallet_receives: Decimal,
    /// Детальное описание расчета
    pub breakdown: String,
    /// Текущий курс TRX/USDT
    pub trx_to_usdt_rate: Decimal,
    /// ID кошелька отправителя
    pub from_wallet_id: i64,
    /// Референс ID
    pub reference_id: Option<String>,
}

/// DTO для ответа по трансферу
#[derive(Debug, Serialize)]
pub struct TransferResponse {
    pub id: i64,
    pub from_wallet_id: i64,
    pub to_address: String,
    pub amount: Decimal,
    pub status: TransactionStatus,
    pub tx_hash: Option<String>,
    pub reference_id: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// DTO для ответа с информацией о входящей транзакции
#[derive(Debug, Serialize)]
pub struct IncomingTransactionResponse {
    pub id: i64,
    pub wallet_id: i64,
    pub tx_hash: String,
    pub block_number: Option<i64>,
    pub from_address: String,
    pub to_address: String,
    pub amount: Decimal,
    pub status: TransactionStatus,
    pub detected_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}
