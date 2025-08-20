//! # Блокчейн сущности
//!
//! Структуры данных для работы с блокчейном

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Информация о транзакции из блокчейна
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainTransaction {
    pub tx_hash: String,
    pub block_number: Option<i64>,
    pub from_address: String,
    pub to_address: String,
    pub amount: Decimal,
    pub timestamp: DateTime<Utc>,
    pub confirmations: u32,
}
