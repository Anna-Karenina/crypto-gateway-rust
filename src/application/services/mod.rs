//! # Сервисы приложения
//!
//! Бизнес-логика разбита по отдельным сервисам:
//!
//! - `WalletService` - управление кошельками
//! - `TransferService` - обработка переводов
//! - `FeeCalculationService` - расчет комиссий
//! - `WalletActivationService` - активация кошельков
//! - `SponsorGasService` - спонсорство газа
//! - `TrxTransferService` - TRX переводы
//! - `TransactionMonitoringService` - мониторинг входящих транзакций

mod activation_service;
mod fee_service;
mod gas_service;
mod monitoring_service;
mod scheduler_service;
mod transfer_service;
mod wallet_service;
mod webhook_service;

pub use activation_service::WalletActivationService;
pub use fee_service::{
    CongestionLevel, FeeCalculationResult, FeeConfig, FeeSource, FeeStats, NetworkState,
    UnifiedFeeService,
};
pub use gas_service::SponsorGasService;
pub use monitoring_service::{MonitoringStats, TransactionMonitoringService};
pub use scheduler_service::{SchedulerConfig, SchedulerStats, TaskScheduler};
pub use transfer_service::{TransferService, TrxTransferService};
pub use wallet_service::WalletService;
pub use webhook_service::{
    WebhookConfig, WebhookData, WebhookEventType, WebhookPayload, WebhookService,
};

// Обратная совместимость - alias для старого названия
pub type FeeCalculationService = UnifiedFeeService;
