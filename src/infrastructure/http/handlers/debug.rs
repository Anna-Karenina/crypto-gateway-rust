//! # Отладочные обработчики
//!
//! HTTP handlers для отладки и мониторинга

use actix_web::{web, HttpResponse, Result};
use serde_json::json;

use crate::{application::state::AppState, VERSION};

/// Корневой маршрут
pub async fn root_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "service": "TRON Gateway",
        "version": VERSION,
        "status": "running",
        "message": "Добро пожаловать в TRON Gateway API!"
    })))
}

/// Health check эндпоинт
pub async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "healthy",
        "version": VERSION,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Получение баланса мастер-кошелька
pub async fn get_master_wallet_balance(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    // Получаем адрес мастер-кошелька из конфига через TransferService
    let master_address = &app_state.transfer_service.master_wallet_address;

    // Получаем баланс USDT и TRX
    match app_state
        .wallet_service
        .get_wallet_balance_by_address(master_address)
        .await
    {
        Ok((usdt_balance, trx_balance)) => Ok(HttpResponse::Ok().json(json!({
            "master_wallet": {
                "address": master_address,
                "balance_usdt": usdt_balance.to_string(),
                "balance_trx": trx_balance.to_string()
            }
        }))),
        Err(err) => {
            tracing::error!("Ошибка получения баланса мастер-кошелька: {}", err);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to get master wallet balance",
                "details": err.to_string()
            })))
        }
    }
}
