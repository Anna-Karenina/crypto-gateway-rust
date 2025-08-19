//! # gRPC сервер
//!
//! Основной gRPC сервер для TRON Gateway

use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

use crate::application::state::AppState;
use crate::config::GrpcConfig;

use super::generated::{
    transfer::transfer_service_server::TransferServiceServer,
    wallet::wallet_service_server::WalletServiceServer,
};
use super::services::{GrpcTransferService, GrpcWalletService};

/// gRPC сервер
pub struct GrpcServer {
    config: GrpcConfig,
    app_state: Arc<AppState>,
}

impl GrpcServer {
    /// Создает новый gRPC сервер
    pub fn new(config: GrpcConfig, app_state: Arc<AppState>) -> Self {
        Self { config, app_state }
    }

    /// Запускает gRPC сервер
    pub async fn serve(self) -> anyhow::Result<()> {
        if !self.config.enabled {
            info!("gRPC сервер отключен в конфигурации");
            return Ok(());
        }

        let addr = format!("{}:{}", self.config.host, self.config.port).parse()?;

        // Создаем сервисы
        let wallet_service = GrpcWalletService::new(self.app_state.clone());
        let transfer_service = GrpcTransferService::new(self.app_state.clone());

        info!("🚀 gRPC сервер запускается на {}", addr);

        // Запускаем сервер с нашими сервисами
        Server::builder()
            .add_service(WalletServiceServer::new(wallet_service))
            .add_service(TransferServiceServer::new(transfer_service))
            .serve(addr)
            .await?;

        Ok(())
    }

    /// Адрес сервера
    pub fn address(&self) -> String {
        format!("{}:{}", self.config.host, self.config.port)
    }
}
