//! # Обработчики мультитокенных операций
//!
//! HTTP endpoints для работы с множественными TRC-20 токенами

use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::application::state::AppState;
use crate::domain::tokens::TokenInfo;

/// Запрос на получение балансов всех токенов
#[derive(Deserialize)]
pub struct MultiBalanceRequest {
    pub wallet_address: String,
}

/// Ответ с балансами всех токенов
#[derive(Serialize)]
pub struct MultiBalanceResponse {
    pub wallet_address: String,
    pub balances: HashMap<String, TokenBalanceInfo>,
    pub total_usd_value: Option<String>,
    pub last_updated: String,
}

#[derive(Serialize)]
pub struct TokenBalanceInfo {
    pub symbol: String,
    pub balance: String,
    pub balance_wei: String,
    pub last_updated: String,
}

/// Запрос на получение списка поддерживаемых токенов
#[derive(Serialize)]
pub struct SupportedTokensResponse {
    pub tokens: Vec<TokenInfoResponse>,
    pub total_count: usize,
}

#[derive(Serialize)]
pub struct TokenInfoResponse {
    pub symbol: String,
    pub name: String,
    pub contract_address: String,
    pub decimals: u8,
    pub is_stable: bool,
    pub min_transfer_amount: String,
    pub max_transfer_amount: Option<String>,
    pub enabled: bool,
    pub icon_url: Option<String>,
}

impl From<&TokenInfo> for TokenInfoResponse {
    fn from(token: &TokenInfo) -> Self {
        Self {
            symbol: token.symbol.clone(),
            name: token.name.clone(),
            contract_address: token.contract_address.clone(),
            decimals: token.decimals,
            is_stable: token.is_stable,
            min_transfer_amount: token.min_transfer_amount.to_string(),
            max_transfer_amount: token.max_transfer_amount.map(|a| a.to_string()),
            enabled: token.enabled,
            icon_url: token.icon_url.clone(),
        }
    }
}

/// Запрос на создание мультитокенного трансфера
#[derive(Deserialize)]
pub struct MultiTokenTransferRequest {
    pub from_wallet_id: i64,
    pub to_address: String,
    pub token_symbol: String,
    pub amount: String, // Decimal as string
    pub reference_id: Option<String>,
}

/// Ответ на создание трансфера
#[derive(Serialize)]
pub struct MultiTokenTransferResponse {
    pub transfer_id: i64,
    pub token_symbol: String,
    pub from_wallet_id: i64,
    pub to_address: String,
    pub amount: String,
    pub status: String,
    pub estimated_fees: FeeBreakdown,
    pub reference_id: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct FeeBreakdown {
    pub gas_fee_usdt: String,
    pub service_commission: String,
    pub total_fees: String,
    pub total_amount_to_deduct: String,
}

/// Статистика кэша токенов
#[derive(Serialize)]
pub struct CacheStatsResponse {
    pub cache_stats: HashMap<String, usize>,
    pub cleanup_performed: bool,
}

/// Получает балансы всех поддерживаемых токенов для кошелька
pub async fn get_multi_token_balance(
    data: web::Data<AppState>,
    query: web::Query<MultiBalanceRequest>,
) -> Result<HttpResponse> {
    tracing::info!(
        "Запрос мультитокенных балансов для кошелька: {}",
        query.wallet_address
    );

    match data
        .trc20_service
        .get_multi_token_balance(&query.wallet_address)
        .await
    {
        Ok(multi_balance) => {
            let mut balances = HashMap::new();

            for (symbol, balance) in multi_balance.balances {
                balances.insert(
                    symbol.clone(),
                    TokenBalanceInfo {
                        symbol: symbol.clone(),
                        balance: balance.balance.to_string(),
                        balance_wei: balance.balance_wei.to_string(),
                        last_updated: balance.last_updated.to_rfc3339(),
                    },
                );
            }

            let response = MultiBalanceResponse {
                wallet_address: multi_balance.wallet_address,
                balances,
                total_usd_value: multi_balance.total_usd_value.map(|v| v.to_string()),
                last_updated: chrono::Utc::now().to_rfc3339(),
            };

            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            tracing::error!("Ошибка получения мультитокенных балансов: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "internal_error",
                "message": format!("Ошибка получения балансов: {}", e)
            })))
        }
    }
}

/// Получает список всех поддерживаемых токенов
pub async fn get_supported_tokens(data: web::Data<AppState>) -> Result<HttpResponse> {
    tracing::info!("Запрос списка поддерживаемых токенов");

    let tokens = data.trc20_service.get_supported_tokens().await;
    let token_responses: Vec<TokenInfoResponse> = tokens.iter().map(|t| t.into()).collect();

    let response = SupportedTokensResponse {
        total_count: token_responses.len(),
        tokens: token_responses,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Создает трансфер любого поддерживаемого токена
pub async fn create_multi_token_transfer(
    data: web::Data<AppState>,
    request: web::Json<MultiTokenTransferRequest>,
) -> Result<HttpResponse> {
    tracing::info!(
        "Создание мультитокенного трансфера: {} {} от кошелька {} на {}",
        request.amount,
        request.token_symbol,
        request.from_wallet_id,
        request.to_address
    );

    // Парсим сумму
    let amount = match request.amount.parse::<rust_decimal::Decimal>() {
        Ok(amount) => amount,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "invalid_amount",
                "message": "Неверный формат суммы"
            })));
        }
    };

    // Получаем информацию о кошельке
    // Здесь нужно интегрироваться с существующим WalletService
    // Для примера - заглушка

    match data
        .trc20_service
        .create_token_transaction(
            "mock_from_address", // В реальности получаем из БД по wallet_id
            &request.to_address,
            &request.token_symbol,
            amount,
        )
        .await
    {
        Ok(_tx_result) => {
            // Создаем запись в БД (интеграция с TransferService)

            let response = MultiTokenTransferResponse {
                transfer_id: 12345, // Mock ID
                token_symbol: request.token_symbol.clone(),
                from_wallet_id: request.from_wallet_id,
                to_address: request.to_address.clone(),
                amount: request.amount.clone(),
                status: "PENDING".to_string(),
                estimated_fees: FeeBreakdown {
                    gas_fee_usdt: "1.5".to_string(),
                    service_commission: "0.5".to_string(),
                    total_fees: "2.0".to_string(),
                    total_amount_to_deduct: (amount + rust_decimal::Decimal::new(2, 0)).to_string(),
                },
                reference_id: request.reference_id.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };

            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            tracing::error!("Ошибка создания мультитокенного трансфера: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "transfer_failed",
                "message": format!("Ошибка создания трансфера: {}", e)
            })))
        }
    }
}

/// Включает или отключает токен
pub async fn toggle_token_status(
    data: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let token_symbol = path.into_inner();
    let enabled = query
        .get("enabled")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    tracing::info!(
        "Изменение статуса токена {}: {}",
        token_symbol,
        if enabled {
            "включен"
        } else {
            "отключен"
        }
    );

    match data.trc20_service.set_token_enabled(&token_symbol, enabled).await {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": format!("Токен {} {}", token_symbol, if enabled { "включен" } else { "отключен" }),
                "token_symbol": token_symbol,
                "enabled": enabled
            })))
        }
        Err(e) => {
            tracing::error!("Ошибка изменения статуса токена: {}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "token_not_found",
                "message": format!("Ошибка: {}", e)
            })))
        }
    }
}

/// Очищает кэш балансов и возвращает статистику
pub async fn get_cache_stats_and_cleanup(data: web::Data<AppState>) -> Result<HttpResponse> {
    tracing::info!("Запрос статистики кэша токенов");

    let stats_before = data.trc20_service.get_cache_stats().await;

    // Выполняем очистку
    data.trc20_service.cleanup_cache().await;

    let stats_after = data.trc20_service.get_cache_stats().await;

    let response = CacheStatsResponse {
        cache_stats: stats_after,
        cleanup_performed: true,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Инвалидирует кэш для конкретного кошелька
pub async fn invalidate_wallet_cache(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let wallet_address = path.into_inner();

    tracing::info!("Инвалидация кэша для кошелька: {}", wallet_address);

    data.trc20_service.invalidate_cache(&wallet_address).await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": format!("Кэш для кошелька {} очищен", wallet_address),
        "wallet_address": wallet_address
    })))
}
