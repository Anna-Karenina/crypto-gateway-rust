//! # TRON интеграция
//!
//! Модули для работы с TRON блокчейном:
//! - `client` - TronGrid API клиент
//! - `crypto` - криптографические операции

pub mod client;
pub mod crypto;
pub mod token_service;

// Реэкспорт основных типов
pub use client::TronGridClient;
pub use crypto::{TronTransactionSigner, TronWalletGenerator};
pub use token_service::{Trc20TokenService, Trc20ServiceConfig};
