//! # TRON Gateway
//!
//! Rust реализация TRC-20 платежного шлюза для TRON сети.
//!
//! ## Архитектура
//!
//! - **Domain**: Бизнес-логика и доменные сущности
//! - **Application**: Сервисы приложения и DTO
//! - **Infrastructure**: Внешние зависимости (БД, TRON API, HTTP)
//! - **Utils**: Вспомогательные утилиты

pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod utils;

// Реэкспорт основных типов для удобства
pub use application::state::AppState;
pub use config::Settings;
pub use domain::{DomainError, TransactionStatus};

/// Версия приложения
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Название приложения
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
