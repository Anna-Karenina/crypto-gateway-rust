//! # Обработчики для кошельков
//!
//! HTTP handlers для операций с кошельками

use actix_web::{web, HttpResponse, Result};
use serde_json::json;

use crate::application::{dto::*, state::AppState};

/// Создание нового кошелька
pub async fn create_wallet(
    app_state: web::Data<AppState>,
    request: web::Json<CreateWalletRequest>,
) -> Result<HttpResponse> {
    match app_state
        .wallet_service
        .create_wallet(request.owner_id.clone())
        .await
    {
        Ok(wallet) => Ok(HttpResponse::Ok().json(wallet)),
        Err(err) => {
            tracing::error!("Ошибка создания кошелька: {}", err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Не удалось создать кошелек",
                "details": err.to_string()
            })))
        }
    }
}

/// Получение кошелька по ID
pub async fn get_wallet(
    app_state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let wallet_id = path.into_inner();

    match app_state.wallet_service.get_wallet(wallet_id).await {
        Ok(Some(wallet)) => Ok(HttpResponse::Ok().json(wallet)),
        Ok(None) => Ok(HttpResponse::NotFound().json(json!({
            "error": "Кошелек не найден",
            "wallet_id": wallet_id
        }))),
        Err(err) => {
            tracing::error!("Ошибка получения кошелька {}: {}", wallet_id, err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Не удалось получить кошелек",
                "details": err.to_string()
            })))
        }
    }
}

/// Получение баланса кошелька
pub async fn get_wallet_balance(
    app_state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let wallet_id = path.into_inner();

    match app_state.wallet_service.get_wallet_balance(wallet_id).await {
        Ok((usdt_balance, trx_balance)) => Ok(HttpResponse::Ok().json(json!({
            "wallet_id": wallet_id,
            "usdt_balance": usdt_balance.to_string(),
            "trx_balance": trx_balance.to_string()
        }))),
        Err(err) => {
            tracing::error!("Ошибка получения баланса кошелька {}: {}", wallet_id, err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Не удалось получить баланс",
                "details": err.to_string()
            })))
        }
    }
}

/// Получение истории транзакций кошелька (входящие + исходящие)
pub async fn get_wallet_transactions(
    app_state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let wallet_id = path.into_inner();

    // Для полной реализации нужен TransactionService для incoming_transactions
    // Пока возвращаем только исходящие трансферы
    match app_state.transfer_service.get_wallet_transfers(wallet_id).await {
        Ok(transfers) => {
            Ok(HttpResponse::Ok().json(json!({
                "wallet_id": wallet_id,
                "transactions": transfers,
                "note": "В данный момент показываются только исходящие трансферы. Для полной реализации нужен TransactionService."
            })))
        }
        Err(err) => {
            tracing::error!("Ошибка получения транзакций кошелька {}: {}", wallet_id, err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get wallet transactions",
                "wallet_id": wallet_id,
                "details": err.to_string()
            })))
        }
    }
}

/// Активация кошелька отправкой TRX
pub async fn activate_wallet(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let wallet_address = path.into_inner();

    match app_state
        .wallet_service
        .activate_wallet_by_address(&wallet_address)
        .await
    {
        Ok(true) => Ok(HttpResponse::Ok().json(json!({
            "address": wallet_address,
            "activation_status": "success",
            "message": "Кошелек успешно активирован"
        }))),
        Ok(false) => Ok(HttpResponse::Ok().json(json!({
            "address": wallet_address,
            "activation_status": "skipped",
            "message": "Кошелек уже был активирован"
        }))),
        Err(err) => {
            tracing::error!("Ошибка активации кошелька {}: {}", wallet_address, err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to activate wallet",
                "address": wallet_address,
                "details": err.to_string()
            })))
        }
    }
}
