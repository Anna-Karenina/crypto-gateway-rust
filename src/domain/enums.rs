use serde::{Deserialize, Serialize};

/// Статус транзакции или трансфера
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TransactionStatus {
    /// Транзакция создана и ожидает обработки
    Pending,
    /// Транзакция обрабатывается (подписывается/отправляется)
    Processing,
    /// Транзакция успешно выполнена
    Completed,
    /// Транзакция отклонена или не удалась
    Failed,
    /// Транзакция отменена
    Cancelled,
}

impl Default for TransactionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Processing => write!(f, "PROCESSING"),
            Self::Completed => write!(f, "COMPLETED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Cancelled => write!(f, "CANCELLED"),
        }
    }
}
