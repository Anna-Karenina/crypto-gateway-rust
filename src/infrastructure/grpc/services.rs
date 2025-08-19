//! # gRPC сервисы
//!
//! Реализация gRPC сервисов для TRON Gateway

use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::application::dto;
use crate::application::state::AppState;

use super::generated::{transfer::*, wallet::*};

/// gRPC сервис для кошельков
pub struct GrpcWalletService {
    app_state: Arc<AppState>,
}

impl GrpcWalletService {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }
}

#[tonic::async_trait]
impl wallet_service_server::WalletService for GrpcWalletService {
    /// Создание нового кошелька
    async fn create_wallet(
        &self,
        request: Request<CreateWalletRequest>,
    ) -> Result<Response<WalletResponse>, Status> {
        let req = request.into_inner();

        match self
            .app_state
            .wallet_service
            .create_wallet(req.owner_id)
            .await
        {
            Ok(wallet) => {
                let response = WalletResponse {
                    id: wallet.id,
                    address: wallet.address,
                    owner_id: wallet.owner_id,
                    created_at: wallet.created_at.to_rfc3339(),
                    balance: wallet.balance.map(|b| b.to_string()),
                };
                Ok(Response::new(response))
            }
            Err(err) => {
                tracing::error!("gRPC: Ошибка создания кошелька: {}", err);
                Err(Status::internal("Failed to create wallet"))
            }
        }
    }

    /// Получение кошелька по ID
    async fn get_wallet(
        &self,
        request: Request<GetWalletRequest>,
    ) -> Result<Response<WalletResponse>, Status> {
        let req = request.into_inner();

        match self
            .app_state
            .wallet_service
            .get_wallet(req.wallet_id)
            .await
        {
            Ok(Some(wallet)) => {
                let response = WalletResponse {
                    id: wallet.id,
                    address: wallet.address,
                    owner_id: wallet.owner_id,
                    created_at: wallet.created_at.to_rfc3339(),
                    balance: wallet.balance.map(|b| b.to_string()),
                };
                Ok(Response::new(response))
            }
            Ok(None) => Err(Status::not_found("Wallet not found")),
            Err(err) => {
                tracing::error!("gRPC: Ошибка получения кошелька: {}", err);
                Err(Status::internal("Failed to get wallet"))
            }
        }
    }

    /// Получение баланса кошелька
    async fn get_wallet_balance(
        &self,
        request: Request<GetWalletBalanceRequest>,
    ) -> Result<Response<WalletBalanceResponse>, Status> {
        let req = request.into_inner();

        match self
            .app_state
            .wallet_service
            .get_wallet_balance(req.wallet_id)
            .await
        {
            Ok((usdt_balance, trx_balance)) => {
                let response = WalletBalanceResponse {
                    wallet_id: req.wallet_id,
                    usdt_balance: usdt_balance.to_string(),
                    trx_balance: trx_balance.to_string(),
                };
                Ok(Response::new(response))
            }
            Err(err) => {
                tracing::error!("gRPC: Ошибка получения баланса: {}", err);
                Err(Status::internal("Failed to get wallet balance"))
            }
        }
    }

    /// Активация кошелька
    async fn activate_wallet(
        &self,
        request: Request<ActivateWalletRequest>,
    ) -> Result<Response<ActivateWalletResponse>, Status> {
        let req = request.into_inner();

        match self
            .app_state
            .wallet_service
            .activate_wallet_by_address(&req.wallet_address)
            .await
        {
            Ok(true) => {
                let response = ActivateWalletResponse {
                    address: req.wallet_address,
                    activation_status: "success".to_string(),
                    message: "Кошелек успешно активирован".to_string(),
                };
                Ok(Response::new(response))
            }
            Ok(false) => {
                let response = ActivateWalletResponse {
                    address: req.wallet_address,
                    activation_status: "skipped".to_string(),
                    message: "Кошелек уже был активирован".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(err) => {
                tracing::error!("gRPC: Ошибка активации кошелька: {}", err);
                Err(Status::internal("Failed to activate wallet"))
            }
        }
    }

    /// Получение транзакций кошелька
    async fn get_wallet_transactions(
        &self,
        request: Request<GetWalletTransactionsRequest>,
    ) -> Result<Response<WalletTransactionsResponse>, Status> {
        let req = request.into_inner();

        match self
            .app_state
            .transfer_service
            .get_wallet_transfers(req.wallet_id)
            .await
        {
            Ok(transfers) => {
                let transactions: Vec<Transaction> = transfers
                    .into_iter()
                    .map(|t| Transaction {
                        id: t.id,
                        tx_hash: t.tx_hash.unwrap_or_default(),
                        status: format!("{:?}", t.status),
                        amount: t.amount.to_string(),
                        created_at: t.created_at.to_rfc3339(),
                    })
                    .collect();

                let response = WalletTransactionsResponse {
                    wallet_id: req.wallet_id,
                    transactions,
                    total_count: 0, // TODO: реализовать подсчет
                };
                Ok(Response::new(response))
            }
            Err(err) => {
                tracing::error!("gRPC: Ошибка получения транзакций: {}", err);
                Err(Status::internal("Failed to get wallet transactions"))
            }
        }
    }
}

/// gRPC сервис для трансферов
pub struct GrpcTransferService {
    app_state: Arc<AppState>,
}

impl GrpcTransferService {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }
}

#[tonic::async_trait]
impl transfer_service_server::TransferService for GrpcTransferService {
    /// Превью трансфера с расчетом комиссий
    async fn preview_transfer(
        &self,
        request: Request<PreviewTransferRequest>,
    ) -> Result<Response<TransferPreviewResponse>, Status> {
        let req = request.into_inner();

        // Конвертируем из gRPC в наш DTO
        let transfer_request = dto::TransferRequest {
            from_wallet_id: req.from_wallet_id,
            order_amount: req
                .order_amount
                .parse()
                .map_err(|_| Status::invalid_argument("Invalid order amount"))?,
            reference_id: req.reference_id,
        };

        match self
            .app_state
            .transfer_service
            .preview_transfer(transfer_request)
            .await
        {
            Ok(preview) => {
                let response = TransferPreviewResponse {
                    order_amount: preview.order_amount.to_string(),
                    commission: preview.commission.to_string(),
                    gas_cost_in_usdt: preview.gas_cost_in_usdt.to_string(),
                    percentage_commission: preview.percentage_commission.to_string(),
                    total_amount: preview.total_amount.to_string(),
                    master_wallet_receives: preview.master_wallet_receives.to_string(),
                    breakdown: preview.breakdown,
                    trx_to_usdt_rate: preview.trx_to_usdt_rate.to_string(),
                    from_wallet_id: preview.from_wallet_id,
                    reference_id: preview.reference_id,
                };
                Ok(Response::new(response))
            }
            Err(err) => {
                tracing::error!("gRPC: Ошибка превью трансфера: {}", err);
                Err(Status::internal("Failed to preview transfer"))
            }
        }
    }

    /// Создание USDT трансфера
    async fn create_transfer(
        &self,
        request: Request<CreateTransferRequest>,
    ) -> Result<Response<TransferResponse>, Status> {
        let req = request.into_inner();

        // Конвертируем из gRPC в наш DTO
        let transfer_request = dto::CreateTransferRequest {
            from_wallet_id: req.from_wallet_id,
            order_amount: req
                .order_amount
                .parse()
                .map_err(|_| Status::invalid_argument("Invalid order amount"))?,
            reference_id: req.reference_id,
            preview_only: req.preview_only,
        };

        match self
            .app_state
            .transfer_service
            .create_transfer(transfer_request)
            .await
        {
            Ok(transfer) => {
                let response = TransferResponse {
                    id: transfer.id,
                    from_wallet_id: transfer.from_wallet_id,
                    to_address: transfer.to_address,
                    amount: transfer.amount.to_string(),
                    status: format!("{:?}", transfer.status),
                    tx_hash: transfer.tx_hash,
                    reference_id: transfer.reference_id,
                    error_message: transfer.error_message,
                    created_at: transfer.created_at.to_rfc3339(),
                    completed_at: transfer.completed_at.map(|dt| dt.to_rfc3339()),
                };
                Ok(Response::new(response))
            }
            Err(err) => {
                tracing::error!("gRPC: Ошибка создания трансфера: {}", err);
                Err(Status::internal("Failed to create transfer"))
            }
        }
    }

    /// Получение трансфера по ID
    async fn get_transfer(
        &self,
        request: Request<GetTransferRequest>,
    ) -> Result<Response<TransferResponse>, Status> {
        let req = request.into_inner();

        match self
            .app_state
            .transfer_service
            .get_transfer(req.transfer_id)
            .await
        {
            Ok(Some(transfer)) => {
                let response = TransferResponse {
                    id: transfer.id,
                    from_wallet_id: transfer.from_wallet_id,
                    to_address: transfer.to_address,
                    amount: transfer.amount.to_string(),
                    status: format!("{:?}", transfer.status),
                    tx_hash: transfer.tx_hash,
                    reference_id: transfer.reference_id,
                    error_message: transfer.error_message,
                    created_at: transfer.created_at.to_rfc3339(),
                    completed_at: transfer.completed_at.map(|dt| dt.to_rfc3339()),
                };
                Ok(Response::new(response))
            }
            Ok(None) => Err(Status::not_found("Transfer not found")),
            Err(err) => {
                tracing::error!("gRPC: Ошибка получения трансфера: {}", err);
                Err(Status::internal("Failed to get transfer"))
            }
        }
    }

    /// Получение трансфера по reference_id
    async fn get_transfer_by_reference(
        &self,
        request: Request<GetTransferByReferenceRequest>,
    ) -> Result<Response<TransferResponse>, Status> {
        let req = request.into_inner();

        match self
            .app_state
            .transfer_service
            .get_transfer_by_reference(&req.reference_id)
            .await
        {
            Ok(Some(transfer)) => {
                let response = TransferResponse {
                    id: transfer.id,
                    from_wallet_id: transfer.from_wallet_id,
                    to_address: transfer.to_address,
                    amount: transfer.amount.to_string(),
                    status: format!("{:?}", transfer.status),
                    tx_hash: transfer.tx_hash,
                    reference_id: transfer.reference_id,
                    error_message: transfer.error_message,
                    created_at: transfer.created_at.to_rfc3339(),
                    completed_at: transfer.completed_at.map(|dt| dt.to_rfc3339()),
                };
                Ok(Response::new(response))
            }
            Ok(None) => Err(Status::not_found("Transfer not found")),
            Err(err) => {
                tracing::error!("gRPC: Ошибка поиска трансфера: {}", err);
                Err(Status::internal("Failed to get transfer by reference"))
            }
        }
    }

    /// Получение трансферов кошелька
    async fn get_wallet_transfers(
        &self,
        request: Request<GetWalletTransfersRequest>,
    ) -> Result<Response<WalletTransfersResponse>, Status> {
        let req = request.into_inner();

        match self
            .app_state
            .transfer_service
            .get_wallet_transfers(req.wallet_id)
            .await
        {
            Ok(transfers) => {
                let transfer_responses: Vec<TransferResponse> = transfers
                    .into_iter()
                    .map(|transfer| TransferResponse {
                        id: transfer.id,
                        from_wallet_id: transfer.from_wallet_id,
                        to_address: transfer.to_address,
                        amount: transfer.amount.to_string(),
                        status: format!("{:?}", transfer.status),
                        tx_hash: transfer.tx_hash,
                        reference_id: transfer.reference_id,
                        error_message: transfer.error_message,
                        created_at: transfer.created_at.to_rfc3339(),
                        completed_at: transfer.completed_at.map(|dt| dt.to_rfc3339()),
                    })
                    .collect();

                let response = WalletTransfersResponse {
                    wallet_id: req.wallet_id,
                    transfers: transfer_responses.clone(),
                    count: transfer_responses.len() as i32,
                };
                Ok(Response::new(response))
            }
            Err(err) => {
                tracing::error!("gRPC: Ошибка получения трансферов кошелька: {}", err);
                Err(Status::internal("Failed to get wallet transfers"))
            }
        }
    }

    /// Обработка pending трансферов
    async fn process_pending_transfers(
        &self,
        _request: Request<ProcessPendingTransfersRequest>,
    ) -> Result<Response<ProcessPendingTransfersResponse>, Status> {
        match self
            .app_state
            .transfer_service
            .process_pending_transfers()
            .await
        {
            Ok(_) => {
                let response = ProcessPendingTransfersResponse {
                    message: "Обработка pending трансферов запущена".to_string(),
                    processed_count: 0, // TODO: возвращать реальное количество
                };
                Ok(Response::new(response))
            }
            Err(err) => {
                tracing::error!("gRPC: Ошибка обработки pending трансферов: {}", err);
                Err(Status::internal("Failed to process pending transfers"))
            }
        }
    }
}
