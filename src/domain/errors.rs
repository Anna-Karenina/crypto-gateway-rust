use thiserror::Error;

/// Основные ошибки доменного слоя
#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Кошелек с ID {id} не найден")]
    WalletNotFound { id: i64 },

    #[error("Кошелек с адресом {address} не найден")]
    WalletNotFoundByAddress { address: String },

    #[error("Недостаточно средств: требуется {required}, доступно {available}")]
    InsufficientBalance { 
        required: rust_decimal::Decimal, 
        available: rust_decimal::Decimal 
    },

    #[error("Неверный адрес TRON: {address}")]
    InvalidTronAddress { address: String },

    #[error("Неверная сумма: {amount}")]
    InvalidAmount { amount: rust_decimal::Decimal },

    #[error("Трансфер с ID {id} не найден")]
    TransferNotFound { id: i64 },

    #[error("Транзакция с хэшем {hash} уже существует")]
    TransactionAlreadyExists { hash: String },

    #[error("Ошибка криптографии: {message}")]
    CryptoError { message: String },

    #[error("Ошибка конфигурации: {message}")]
    ConfigurationError { message: String },
}

/// Результат операций доменного слоя
pub type DomainResult<T> = Result<T, DomainError>;
