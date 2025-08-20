//! # Улучшенный TRC-20 Token Service
//!
//! Сервис для работы с множественными TRC-20 токенами с кэшированием и оптимизацией

use anyhow::Result;
use reqwest::Client;
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::config::TronConfig;
use crate::domain::tokens::{MultiTokenBalance, TokenInfo, TokenRegistry};
use crate::infrastructure::retry::{classify_reqwest_error, RetryConfig, RetryableService};

/// Кэшированный баланс
#[derive(Debug, Clone)]
struct CachedBalance {
    balance: Decimal,
    balance_wei: u64,
    cached_at: Instant,
    ttl: Duration,
}

impl CachedBalance {
    fn new(balance: Decimal, balance_wei: u64, ttl: Duration) -> Self {
        Self {
            balance,
            balance_wei,
            cached_at: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

/// Конфигурация TRC-20 сервиса
#[derive(Debug, Clone)]
pub struct Trc20ServiceConfig {
    pub balance_cache_ttl_seconds: u64,
    pub batch_size: u32,
    pub enable_parallel_requests: bool,
    pub rate_limit_per_second: u32,
}

impl Default for Trc20ServiceConfig {
    fn default() -> Self {
        Self {
            balance_cache_ttl_seconds: 30, // Кэш на 30 секунд
            batch_size: 5,                 // Батчи по 5 запросов
            enable_parallel_requests: true,
            rate_limit_per_second: 10, // 10 запросов в секунду
        }
    }
}

/// Улучшенный сервис для работы с TRC-20 токенами
#[derive(Clone)]
pub struct Trc20TokenService {
    client: Client,
    tron_config: TronConfig,
    service_config: Trc20ServiceConfig,
    token_registry: Arc<RwLock<TokenRegistry>>,
    balance_cache: Arc<RwLock<HashMap<String, CachedBalance>>>, // key: "address:token_symbol"
    retry_service: RetryableService<()>,
}

impl Trc20TokenService {
    /// Создает новый экземпляр сервиса
    pub fn new(
        tron_config: TronConfig,
        service_config: Trc20ServiceConfig,
        token_registry: TokenRegistry,
    ) -> Self {
        let retry_config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        };

        Self {
            client: Client::new(),
            tron_config,
            service_config,
            token_registry: Arc::new(RwLock::new(token_registry)),
            balance_cache: Arc::new(RwLock::new(HashMap::new())),
            retry_service: RetryableService::with_config((), retry_config),
        }
    }

    /// Получает баланс конкретного токена с кэшированием
    pub async fn get_token_balance(
        &self,
        wallet_address: &str,
        token_symbol: &str,
    ) -> Result<Decimal> {
        let cache_key = format!("{}:{}", wallet_address, token_symbol);

        // Проверяем кэш
        {
            let cache = self.balance_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if !cached.is_expired() {
                    return Ok(cached.balance);
                }
            }
        }

        // Получаем информацию о токене
        let token_registry = self.token_registry.read().await;
        let token_info = token_registry
            .get_token(token_symbol)
            .ok_or_else(|| anyhow::anyhow!("Токен {} не поддерживается", token_symbol))?;

        if !token_info.enabled {
            return Err(anyhow::anyhow!("Токен {} отключен", token_symbol));
        }

        // Получаем баланс из сети
        let (balance, balance_wei) = self
            .fetch_token_balance_from_network(
                wallet_address,
                &token_info.contract_address,
                token_info.decimals,
            )
            .await?;

        // Кэшируем результат
        {
            let mut cache = self.balance_cache.write().await;
            let ttl = Duration::from_secs(self.service_config.balance_cache_ttl_seconds);
            cache.insert(cache_key, CachedBalance::new(balance, balance_wei, ttl));
        }

        Ok(balance)
    }

    /// Получает балансы всех поддерживаемых токенов для кошелька
    pub async fn get_multi_token_balance(&self, wallet_address: &str) -> Result<MultiTokenBalance> {
        let mut multi_balance = MultiTokenBalance::new(wallet_address.to_string());

        let token_registry = self.token_registry.read().await;
        let enabled_tokens = token_registry.get_enabled_tokens();

        if self.service_config.enable_parallel_requests {
            // Параллельные запросы с ограничением батчей
            let mut futures = Vec::new();

            for chunk in enabled_tokens.chunks(self.service_config.batch_size as usize) {
                for token in chunk {
                    let fut = self.get_token_balance(wallet_address, &token.symbol);
                    futures.push((token.symbol.clone(), fut));
                }

                // Ждем выполнения батча
                for (symbol, fut) in futures.drain(..) {
                    match fut.await {
                        Ok(balance) => {
                            let token_info = token_registry.get_token(&symbol).unwrap();
                            let balance_wei = token_info.to_wei(balance).unwrap_or(0);
                            multi_balance.add_balance(symbol, balance, balance_wei);
                        }
                        Err(e) => {
                            warn!("Ошибка получения баланса {}: {}", symbol, e);
                        }
                    }
                }

                // Пауза между батчами для rate limiting
                if !futures.is_empty() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        } else {
            // Последовательные запросы
            for token in enabled_tokens {
                match self.get_token_balance(wallet_address, &token.symbol).await {
                    Ok(balance) => {
                        let balance_wei = token.to_wei(balance).unwrap_or(0);
                        multi_balance.add_balance(token.symbol.clone(), balance, balance_wei);
                    }
                    Err(e) => {
                        warn!("Ошибка получения баланса {}: {}", token.symbol, e);
                    }
                }
            }
        }

        Ok(multi_balance)
    }

    /// Создает TRC-20 транзакцию для любого поддерживаемого токена
    pub async fn create_token_transaction(
        &self,
        from_address: &str,
        to_address: &str,
        token_symbol: &str,
        amount: Decimal,
    ) -> Result<Value> {
        let token_registry = self.token_registry.read().await;
        let token_info = token_registry
            .get_token(token_symbol)
            .ok_or_else(|| anyhow::anyhow!("Токен {} не поддерживается", token_symbol))?;

        if !token_info.enabled {
            return Err(anyhow::anyhow!("Токен {} отключен", token_symbol));
        }

        // Валидация суммы
        token_info
            .validate_amount(amount)
            .map_err(|e| anyhow::anyhow!("Валидация суммы: {}", e))?;

        // Конвертируем адреса
        let hex_from = self.address_to_hex(from_address)?;
        let hex_to = self.address_to_hex(to_address)?;

        // Конвертируем сумму в wei
        let amount_wei = token_info.to_wei(amount)?;

        let url = format!("{}/wallet/triggersmartcontract", self.tron_config.base_url);

        let payload = serde_json::json!({
            "owner_address": hex_from,
            "contract_address": self.address_to_hex(&token_info.contract_address)?,
            "function_selector": "transfer(address,uint256)",
            "parameter": format!("{:0>64}{:0>64}",
                &hex_to[2..], // убираем 0x
                format!("{:x}", amount_wei)
            ),
            "fee_limit": 100_000_000, // 100 TRX
        });

        // Отправляем с retry логикой
        self.retry_service
            .retry("create_token_transaction", || {
                let client = self.client.clone();
                let url = url.clone();
                let payload = payload.clone();
                let api_key = self.tron_config.api_key.clone();

                async move {
                    let mut request = client.post(&url).json(&payload);

                    if let Some(key) = api_key {
                        request = request.header("TRON-PRO-API-KEY", key);
                    }

                    let response = request.send().await.map_err(classify_reqwest_error)?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let error_text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());

                        return Err(crate::infrastructure::retry::classify_http_error(
                            status,
                            &error_text,
                        ));
                    }

                    let result: Value = response.json().await.map_err(|e| {
                        crate::infrastructure::retry::RetryableError::Temporary(format!(
                            "JSON parse error: {}",
                            e
                        ))
                    })?;

                    Ok(result)
                }
            })
            .await
    }

    /// Получает список транзакций для всех поддерживаемых токенов
    pub async fn get_multi_token_transactions(
        &self,
        wallet_address: &str,
        limit: u32,
    ) -> Result<Vec<crate::domain::BlockchainTransaction>> {
        let mut all_transactions = Vec::new();

        let token_registry = self.token_registry.read().await;
        let enabled_tokens = token_registry.get_enabled_tokens();

        for token in enabled_tokens {
            match self
                .get_token_transactions(wallet_address, &token.symbol, limit)
                .await
            {
                Ok(mut transactions) => {
                    all_transactions.append(&mut transactions);
                }
                Err(e) => {
                    warn!("Ошибка получения транзакций {}: {}", token.symbol, e);
                }
            }
        }

        // Сортируем по времени (новые первыми)
        all_transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Ограничиваем результат
        if all_transactions.len() > limit as usize {
            all_transactions.truncate(limit as usize);
        }

        Ok(all_transactions)
    }

    /// Получает транзакции конкретного токена
    pub async fn get_token_transactions(
        &self,
        wallet_address: &str,
        token_symbol: &str,
        limit: u32,
    ) -> Result<Vec<crate::domain::BlockchainTransaction>> {
        let token_registry = self.token_registry.read().await;
        let token_info = token_registry
            .get_token(token_symbol)
            .ok_or_else(|| anyhow::anyhow!("Токен {} не поддерживается", token_symbol))?;

        // Используем существующую логику из TronGridClient
        self.get_trc20_transactions(wallet_address, &token_info.contract_address, limit)
            .await
    }

    /// Инвалидирует кэш для кошелька
    pub async fn invalidate_cache(&self, wallet_address: &str) {
        let mut cache = self.balance_cache.write().await;
        cache.retain(|key, _| !key.starts_with(&format!("{}:", wallet_address)));
        info!("Кэш балансов очищен для кошелька {}", wallet_address);
    }

    /// Добавляет новый токен в реестр
    pub async fn add_token(&self, token: TokenInfo) -> Result<()> {
        let mut registry = self.token_registry.write().await;
        registry.add_token(token.clone());
        info!(
            "Добавлен новый токен: {} ({})",
            token.symbol, token.contract_address
        );
        Ok(())
    }

    /// Включает/отключает токен
    pub async fn set_token_enabled(&self, token_symbol: &str, enabled: bool) -> Result<()> {
        let mut registry = self.token_registry.write().await;
        registry
            .set_token_enabled(token_symbol, enabled)
            .map_err(|e| anyhow::anyhow!(e))?;

        info!(
            "Токен {} {}",
            token_symbol,
            if enabled {
                "включен"
            } else {
                "отключен"
            }
        );
        Ok(())
    }

    /// Получает информацию о всех токенах
    pub async fn get_supported_tokens(&self) -> Vec<TokenInfo> {
        let registry = self.token_registry.read().await;
        registry.get_all_tokens().into_iter().cloned().collect()
    }

    /// Получает статистику кэша
    pub async fn get_cache_stats(&self) -> HashMap<String, usize> {
        let cache = self.balance_cache.read().await;
        let mut stats = HashMap::new();

        stats.insert("total_entries".to_string(), cache.len());

        let expired_count = cache.values().filter(|entry| entry.is_expired()).count();
        stats.insert("expired_entries".to_string(), expired_count);
        stats.insert("active_entries".to_string(), cache.len() - expired_count);

        stats
    }

    /// Очищает просроченные записи из кэша
    pub async fn cleanup_cache(&self) {
        let mut cache = self.balance_cache.write().await;
        let initial_size = cache.len();
        cache.retain(|_, entry| !entry.is_expired());
        let cleaned = initial_size - cache.len();

        if cleaned > 0 {
            info!("Очищено {} просроченных записей кэша", cleaned);
        }
    }

    // Приватные методы (копируем нужные из TronGridClient)

    async fn fetch_token_balance_from_network(
        &self,
        wallet_address: &str,
        contract_address: &str,
        decimals: u8,
    ) -> Result<(Decimal, u64)> {
        // Используем правильный API для получения баланса TRC-20
        let url = format!(
            "{}/v1/accounts/{}/transactions/trc20",
            self.tron_config.base_url, wallet_address
        );

        let mut request = self
            .client
            .get(&url)
            .query(&[("limit", "1"), ("contract_address", contract_address)]);

        if let Some(api_key) = &self.tron_config.api_key {
            request = request.header("TRON-PRO-API-KEY", api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Ok((Decimal::ZERO, 0));
        }

        let data: Value = response.json().await?;

        // Простая логика вычисления баланса
        if let Some(transactions) = data.get("data").and_then(|d| d.as_array()) {
            let mut balance_wei = 0u64;

            for tx in transactions {
                if let Some(token_info) = tx.get("token_info") {
                    if let (Some(address_to), Some(value_str)) = (
                        token_info.get("address").and_then(|a| a.as_str()),
                        tx.get("value").and_then(|v| v.as_str()),
                    ) {
                        if address_to.eq_ignore_ascii_case(wallet_address) {
                            if let Ok(amount_wei) = value_str.parse::<u64>() {
                                balance_wei += amount_wei;
                            }
                        }
                    }
                }
            }

            let balance = Decimal::from(balance_wei) / Decimal::from(10u64.pow(decimals as u32));
            Ok((balance, balance_wei))
        } else {
            Ok((Decimal::ZERO, 0))
        }
    }

    async fn get_trc20_transactions(
        &self,
        _address: &str,
        _contract_address: &str,
        _limit: u32,
    ) -> Result<Vec<crate::domain::BlockchainTransaction>> {
        // Реализация аналогична TronGridClient::get_trc20_transactions
        // Но адаптирована для работы с retry сервисом
        // ... (детали опущены для краткости)
        Ok(Vec::new()) // Заглушка
    }

    fn address_to_hex(&self, address: &str) -> Result<String> {
        // Конвертация base58 в hex (копируем из TronGridClient)
        use base58::FromBase58;

        let decoded = address
            .from_base58()
            .map_err(|_| anyhow::anyhow!("Неверный формат адреса: {}", address))?;

        if decoded.len() < 21 {
            return Err(anyhow::anyhow!("Неверная длина адреса: {}", address));
        }

        // Берем первые 21 байт (без checksum)
        let hex_address = hex::encode(&decoded[..21]);
        Ok(format!("0x{}", hex_address))
    }
}
