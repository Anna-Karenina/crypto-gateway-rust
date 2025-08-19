//! # TRON интеграция
//!
//! Модули для работы с TRON блокчейном:
//! - `client` - TronGrid API клиент
//! - `crypto` - криптографические операции

pub mod client;
pub mod crypto;

// Реэкспорт основных типов
pub use client::TronGridClient;
pub use crypto::{TronTransactionSigner, TronWalletGenerator};
