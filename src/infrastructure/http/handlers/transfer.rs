//! # Обработчики для переводов
//!
//! HTTP handlers для операций с переводами USDT

use actix_web::{web, HttpResponse, Result};
use serde_json::json;

use crate::application::{dto::*, state::AppState};

/// Превью трансфера с расчетом комиссий
pub async fn preview_transfer(
    app_state: web::Data<AppState>,
    request: web::Json<TransferRequest>,
) -> Result<HttpResponse> {
    match app_state
        .transfer_service
        .preview_transfer(request.into_inner())
        .await
    {
        Ok(preview) => Ok(HttpResponse::Ok().json(preview)),
        Err(err) => {
            tracing::error!("Ошибка создания превью трансфера: {}", err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Не удалось создать превью трансфера",
                "details": err.to_string()
            })))
        }
    }
}

/// Создание USDT трансфера
pub async fn create_transfer(
    app_state: web::Data<AppState>,
    body: web::Json<CreateTransferRequest>,
) -> Result<HttpResponse> {
    let request = body.into_inner();

    match app_state.transfer_service.create_transfer(request).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(err) => {
            tracing::error!("Ошибка создания трансфера: {}", err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Не удалось создать трансфер",
                "details": err.to_string()
            })))
        }
    }
}

/// Получение трансфера по ID
pub async fn get_transfer(
    app_state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let transfer_id = path.into_inner();

    match app_state.transfer_service.get_transfer(transfer_id).await {
        Ok(Some(transfer)) => Ok(HttpResponse::Ok().json(transfer)),
        Ok(None) => Ok(HttpResponse::NotFound().json(json!({
            "error": "Transfer not found",
            "transfer_id": transfer_id
        }))),
        Err(err) => {
            tracing::error!("Ошибка получения трансфера {}: {}", transfer_id, err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get transfer",
                "details": err.to_string()
            })))
        }
    }
}

/// Получение трансфера по reference_id
pub async fn get_transfer_by_reference(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let reference_id = path.into_inner();

    match app_state
        .transfer_service
        .get_transfer_by_reference(&reference_id)
        .await
    {
        Ok(Some(transfer)) => Ok(HttpResponse::Ok().json(transfer)),
        Ok(None) => Ok(HttpResponse::NotFound().json(json!({
            "error": "Transfer not found",
            "reference_id": reference_id
        }))),
        Err(err) => {
            tracing::error!(
                "Ошибка поиска трансфера по reference_id {}: {}",
                reference_id,
                err
            );
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get transfer by reference",
                "details": err.to_string()
            })))
        }
    }
}

/// Получение всех трансферов кошелька
pub async fn get_wallet_transfers(
    app_state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let wallet_id = path.into_inner();

    match app_state
        .transfer_service
        .get_wallet_transfers(wallet_id)
        .await
    {
        Ok(transfers) => Ok(HttpResponse::Ok().json(json!({
            "wallet_id": wallet_id,
            "transfers": transfers,
            "count": transfers.len()
        }))),
        Err(err) => {
            tracing::error!(
                "Ошибка получения трансферов кошелька {}: {}",
                wallet_id,
                err
            );
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get wallet transfers",
                "wallet_id": wallet_id,
                "details": err.to_string()
            })))
        }
    }
}

/// Ручной запуск обработки pending трансферов
/// Этот endpoint можно использовать для тестирования или ручной обработки
/// В продакшене трансферы обрабатываются автоматически по расписанию
pub async fn process_pending_transfers(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    tracing::info!("Ручной запуск обработки pending трансферов");

    match app_state.transfer_service.process_pending_transfers().await {
        Ok(_) => Ok(HttpResponse::Ok().json(json!({
            "message": "Обработка pending трансферов запущена"
        }))),
        Err(err) => {
            tracing::error!("Ошибка обработки pending трансферов: {}", err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Ошибка обработки трансферов",
                "details": err.to_string()
            })))
        }
    }
}

/// Получение транзакции по хэшу
pub async fn get_transaction(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let tx_hash = path.into_inner();

    // Ищем транзакцию в исходящих трансферах
    match app_state
        .transfer_service
        .get_transfer_by_tx_hash(&tx_hash)
        .await
    {
        Ok(Some(transfer)) => Ok(HttpResponse::Ok().json(json!({
            "tx_hash": tx_hash,
            "transaction_type": "outgoing_transfer",
            "transfer": transfer
        }))),
        Ok(None) => {
            // Можно было бы искать еще в incoming_transactions, но пока просто 404
            Ok(HttpResponse::NotFound().json(json!({
                "error": "Transaction not found",
                "tx_hash": tx_hash
            })))
        }
        Err(err) => {
            tracing::error!("Ошибка поиска транзакции {}: {}", tx_hash, err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get transaction",
                "details": err.to_string()
            })))
        }
    }
}
