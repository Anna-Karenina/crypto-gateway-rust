// Diesel модели для работы с базой данных

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::infrastructure::database::schema::{incoming_transactions, outgoing_transfers, wallets};

/// Модель кошелька для diesel
#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = wallets)]
pub struct WalletModel {
    pub id: i64,
    pub address: String,
    pub hex_address: String,
    pub private_key: String,
    pub owner_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Модель для создания нового кошелька
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = wallets)]
pub struct NewWallet {
    pub address: String,
    pub hex_address: String,
    pub private_key: String,
    pub owner_id: Option<String>,
}

/// Модель входящей транзакции для diesel
#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = incoming_transactions)]
pub struct IncomingTransactionModel {
    pub id: i64,
    pub wallet_id: i64,
    pub tx_hash: String,
    pub block_number: Option<i64>,
    pub from_address: String,
    pub to_address: String,
    pub amount: BigDecimal,
    pub status: String,
    pub error_message: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

/// Модель для создания новой входящей транзакции
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = incoming_transactions)]
pub struct NewIncomingTransaction {
    pub wallet_id: i64,
    pub tx_hash: String,
    pub block_number: Option<i64>,
    pub from_address: String,
    pub to_address: String,
    pub amount: BigDecimal,
    pub status: String,
    pub error_message: Option<String>,
}

/// Модель исходящего трансфера для diesel
#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = outgoing_transfers)]
pub struct OutgoingTransferModel {
    pub id: i64,
    pub from_wallet_id: i64,
    pub to_address: String,
    pub amount: BigDecimal,
    pub status: String,
    pub tx_hash: Option<String>,
    pub reference_id: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Модель для создания нового исходящего трансфера
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = outgoing_transfers)]
pub struct NewOutgoingTransfer {
    pub from_wallet_id: i64,
    pub to_address: String,
    pub amount: BigDecimal,
    pub status: String,
    pub reference_id: Option<String>,
}
