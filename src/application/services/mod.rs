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

mod activation_service;
mod fee_service;
mod gas_service;
mod transfer_service;
mod wallet_service;

pub use activation_service::WalletActivationService;
pub use fee_service::FeeCalculationService;
pub use gas_service::SponsorGasService;
pub use transfer_service::{TransferService, TrxTransferService};
pub use wallet_service::WalletService;
