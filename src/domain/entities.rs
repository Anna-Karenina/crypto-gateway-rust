use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::enums::TransactionStatus;

/// Custodial кошелек для приема TRC-20 токенов на сети TRON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// Уникальный идентификатор кошелька в БД
    pub id: Option<i64>,

    /// Base58 адрес TRON (публичный адрес для получения платежей)
    pub address: String,

    /// Hex адрес TRON (используется в API запросах)
    pub hex_address: String,

    /// Приватный ключ в hex формате (хранится зашифрованным в продакшене)
    pub private_key: String,

    /// Идентификатор владельца (опционально, для связи с пользователем)
    pub owner_id: Option<String>,

    /// Время создания кошелька
    pub created_at: DateTime<Utc>,
}

impl Wallet {
    /// Создает новый кошелек с сгенерированными ключами
    pub fn new(
        address: String,
        hex_address: String,
        private_key: String,
        owner_id: Option<String>,
    ) -> Self {
        Self {
            id: None,
            address,
            hex_address,
            private_key,
            owner_id,
            created_at: Utc::now(),
        }
    }
}

/// Входящая транзакция TRC-20 токенов на один из custodial кошельков
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingTransaction {
    /// Уникальный идентификатор в БД
    pub id: Option<i64>,

    /// ID кошелька-получателя
    pub wallet_id: i64,

    /// Хэш транзакции в блокчейне TRON
    pub tx_hash: String,

    /// Номер блока
    pub block_number: Option<i64>,

    /// Адрес отправителя
    pub from_address: String,

    /// Адрес получателя (должен соответствовать одному из наших кошельков)
    pub to_address: String,

    /// Сумма в USDT
    pub amount: Decimal,

    /// Статус обработки транзакции
    pub status: TransactionStatus,

    /// Сообщение об ошибке (если статус Failed)
    pub error_message: Option<String>,

    /// Время обнаружения транзакции
    pub detected_at: DateTime<Utc>,

    /// Время подтверждения в блокчейне
    pub confirmed_at: Option<DateTime<Utc>>,
}

impl IncomingTransaction {
    /// Создает новую входящую транзакцию
    pub fn new(
        wallet_id: i64,
        tx_hash: String,
        from_address: String,
        to_address: String,
        amount: Decimal,
    ) -> Self {
        Self {
            id: None,
            wallet_id,
            tx_hash,
            block_number: None,
            from_address,
            to_address,
            amount,
            status: TransactionStatus::Pending,
            error_message: None,
            detected_at: Utc::now(),
            confirmed_at: None,
        }
    }
}

/// Исходящий трансфер от пользовательского кошелька к мастер-кошельку
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingTransfer {
    /// Уникальный идентификатор в БД
    pub id: Option<i64>,

    /// ID кошелька отправителя
    pub from_wallet_id: i64,

    /// Адрес получателя (обычно мастер-кошелек)
    pub to_address: String,

    /// Сумма для перевода в USDT
    pub amount: Decimal,

    /// Статус трансфера
    pub status: TransactionStatus,

    /// Хэш транзакции (устанавливается после успешной отправки)
    pub tx_hash: Option<String>,

    /// Референс для связи с заказом в e-commerce системе
    pub reference_id: Option<String>,

    /// Сообщение об ошибке
    pub error_message: Option<String>,

    /// Время создания запроса на трансфер
    pub created_at: DateTime<Utc>,

    /// Время завершения трансфера
    pub completed_at: Option<DateTime<Utc>>,
}

impl OutgoingTransfer {
    /// Создает новый исходящий трансфер
    pub fn new(
        from_wallet_id: i64,
        to_address: String,
        amount: Decimal,
        reference_id: Option<String>,
    ) -> Self {
        Self {
            id: None,
            from_wallet_id,
            to_address,
            amount,
            status: TransactionStatus::Pending,
            tx_hash: None,
            reference_id,
            error_message: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Помечает трансфер как завершенный с указанием хэша транзакции
    pub fn complete_with_hash(&mut self, tx_hash: String) {
        self.status = TransactionStatus::Completed;
        self.tx_hash = Some(tx_hash);
        self.completed_at = Some(Utc::now());
    }

    /// Помечает трансфер как неудачный с указанием ошибки
    pub fn fail_with_error(&mut self, error: String) {
        self.status = TransactionStatus::Failed;
        self.error_message = Some(error);
        self.completed_at = Some(Utc::now());
    }
}
