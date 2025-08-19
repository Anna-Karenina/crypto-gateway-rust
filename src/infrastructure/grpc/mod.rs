//! # gRPC сервер
//!
//! Высокопроизводительный gRPC API для TRON Gateway

pub mod server;
pub mod services;

// Включаем сгенерированный код
pub mod generated {
    pub mod common {
        tonic::include_proto!("tron_gateway.common.v1");
    }

    pub mod wallet {
        tonic::include_proto!("tron_gateway.wallet.v1");
    }

    pub mod transfer {
        tonic::include_proto!("tron_gateway.transfer.v1");
    }
}

// Реэкспорт для удобства
pub use server::GrpcServer;
pub use services::{GrpcTransferService, GrpcWalletService};
