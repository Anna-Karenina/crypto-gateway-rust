//! # TRON Grid API клиент
//!
//! HTTP клиент для взаимодействия с TRON сетью через TronGrid API

use anyhow::Result;
use base58::{FromBase58, ToBase58};
use chrono::{TimeZone, Utc};
use reqwest::Client;
use rust_decimal::prelude::*;
use serde_json::Value;
use sha2::Digest;
use std::time::Duration;

use crate::config::TronConfig;
use crate::domain::BlockchainTransaction;
use crate::infrastructure::retry::{RetryConfig, RetryableService};

/// Клиент для взаимодействия с TronGrid API
#[derive(Clone)]
pub struct TronGridClient {
    client: Client,
    config: TronConfig,
    retry_service: RetryableService<()>,
}

impl TronGridClient {
    /// Создает новый экземпляр клиента
    pub fn new(config: TronConfig) -> Self {
        // Настройка retry для TRON API
        let retry_config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        };

        Self {
            client: Client::new(),
            config,
            retry_service: RetryableService::with_config((), retry_config),
        }
    }

    /// Получение баланса USDT по адресу
    pub async fn get_usdt_balance(&self, address: &str) -> Result<rust_decimal::Decimal> {
        let url = format!(
            "{}/v1/accounts/{}/transactions/trc20",
            self.config.base_url, address
        );

        let mut request = self.client.get(&url).query(&[
            ("limit", "200"),
            ("contract_address", &self.config.usdt_contract),
        ]);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("TRON-PRO-API-KEY", api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            tracing::warn!("TronGrid API error for USDT balance: {}", response.status());
            return Ok(rust_decimal::Decimal::ZERO);
        }

        let data: serde_json::Value = response.json().await?;

        // Вычисляем баланс из транзакций (упрощенная логика)
        // В реальности нужно делать запрос к /walletsolidity/gettrc20balance
        if let Some(transactions) = data.get("data").and_then(|d| d.as_array()) {
            let mut balance = rust_decimal::Decimal::ZERO;

            for tx in transactions {
                if let Some(token_info) = tx.get("token_info") {
                    if let (Some(address_to), Some(value_str)) = (
                        token_info.get("address").and_then(|a| a.as_str()),
                        tx.get("value").and_then(|v| v.as_str()),
                    ) {
                        if address_to.eq_ignore_ascii_case(address) {
                            if let Ok(amount) = rust_decimal::Decimal::from_str(value_str) {
                                let normalized =
                                    amount / rust_decimal::Decimal::new(10_i64.pow(6), 0);
                                balance += normalized;
                            }
                        }
                    }
                }
            }

            Ok(balance)
        } else {
            Ok(rust_decimal::Decimal::ZERO)
        }
    }

    /// Получение баланса TRX по адресу
    pub async fn get_trx_balance(&self, address: &str) -> Result<rust_decimal::Decimal> {
        let url = format!("{}/v1/accounts/{}", self.config.base_url, address);

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("TRON-PRO-API-KEY", api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            tracing::warn!("TronGrid API error for TRX balance: {}", response.status());
            return Ok(rust_decimal::Decimal::ZERO);
        }

        let data: Value = response.json().await?;

        // Парсим баланс из ответа
        if let Some(accounts) = data.get("data").and_then(|d| d.as_array()) {
            if let Some(account) = accounts.first() {
                if let Some(balance_num) = account.get("balance").and_then(|b| b.as_u64()) {
                    // Конвертируем из sun в TRX (1 TRX = 1,000,000 sun)
                    let balance_decimal = rust_decimal::Decimal::from(balance_num)
                        / rust_decimal::Decimal::new(1_000_000, 0);
                    return Ok(balance_decimal);
                }
            }
        }

        Ok(rust_decimal::Decimal::ZERO)
    }

    /// Оценка энергии для TRC20 транзакции
    pub async fn estimate_energy(
        &self,
        from: &str,
        to: &str,
        amount: rust_decimal::Decimal,
    ) -> Result<u64> {
        tracing::debug!(
            "Оценка энергии для трансфера {} USDT с {} на {}",
            amount,
            from,
            to
        );

        // Для упрощения возвращаем фиксированную оценку
        // В реальности можно делать запрос к /wallet/triggerconstantcontract
        let estimated_energy = 31895_u64; // Примерная энергия для TRC20 трансфера

        tracing::debug!("Оценка энергии: {} единиц", estimated_energy);
        Ok(estimated_energy)
    }

    /// Создание TRC20 транзакции (USDT)
    pub async fn create_trc20_transaction(
        &self,
        from: &str,
        to: &str,
        amount: rust_decimal::Decimal,
    ) -> Result<Value> {
        let hex_from = self.address_to_hex(from)?;
        let hex_to = self.address_to_hex(to)?;

        // Конвертируем USDT в минимальные единицы (с 6 знаками после запятой)
        let amount_units = amount * rust_decimal::Decimal::new(10_i64.pow(6), 0);
        let amount_u64 = amount_units
            .to_u64()
            .ok_or_else(|| anyhow::anyhow!("Недопустимая сумма"))?;

        let url = format!("{}/wallet/triggersmartcontract", self.config.base_url);

        let payload = serde_json::json!({
            "owner_address": hex_from,
            "contract_address": self.address_to_hex(&self.config.usdt_contract)?,
            "function_selector": "transfer(address,uint256)",
            "parameter": format!("{:0>64}{:0>64}",
                &hex_to[2..], // убираем 0x
                format!("{:x}", amount_u64)
            ),
            "fee_limit": 100_000_000, // 100 TRX
        });

        let mut request = self.client.post(&url).json(&payload);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("TRON-PRO-API-KEY", api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Ошибка создания TRC20 транзакции: {}",
                error_text
            ));
        }

        let result: Value = response.json().await?;

        tracing::debug!("TRC20 транзакция создана: {:?}", result);
        Ok(result)
    }

    /// Создание TRX транзакции
    pub async fn create_trx_transaction(
        &self,
        from: &str,
        to: &str,
        amount: rust_decimal::Decimal,
    ) -> Result<Value> {
        let hex_from = self.address_to_hex(from)?;
        let hex_to = self.address_to_hex(to)?;

        // Конвертируем TRX в sun (1 TRX = 1,000,000 sun)
        let amount_sun = amount * rust_decimal::Decimal::new(1_000_000, 0);
        let amount_u64 = amount_sun
            .to_u64()
            .ok_or_else(|| anyhow::anyhow!("Недопустимая сумма TRX"))?;

        let url = format!("{}/wallet/createtransaction", self.config.base_url);

        let payload = serde_json::json!({
            "to_address": hex_to,
            "owner_address": hex_from,
            "amount": amount_u64
        });

        let mut request = self.client.post(&url).json(&payload);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("TRON-PRO-API-KEY", api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Ошибка создания TRX транзакции: {}",
                error_text
            ));
        }

        let result: Value = response.json().await?;

        tracing::debug!("TRX транзакция создана: {:?}", result);
        Ok(result)
    }

    /// Отправка подписанной транзакции
    pub async fn broadcast_transaction(&self, signed_transaction: &Value) -> Result<String> {
        let url = format!("{}/wallet/broadcasttransaction", self.config.base_url);

        let mut request = self.client.post(&url).json(signed_transaction);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("TRON-PRO-API-KEY", api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Ошибка отправки транзакции: {}",
                error_text
            ));
        }

        let result: Value = response.json().await?;

        if let Some(result_bool) = result.get("result").and_then(|r| r.as_bool()) {
            if result_bool {
                if let Some(txid) = result.get("txid").and_then(|t| t.as_str()) {
                    tracing::info!("Транзакция успешно отправлена. TX Hash: {}", txid);
                    return Ok(txid.to_string());
                }
            }
        }

        // Если есть сообщение об ошибке
        if let Some(message) = result.get("message") {
            if let Some(msg_str) = message.as_str() {
                return Err(anyhow::anyhow!("Broadcast failed: {}", msg_str));
            } else if let Some(msg_bytes) = message.as_array() {
                let hex_message: String = msg_bytes
                    .iter()
                    .filter_map(|b| b.as_u64())
                    .map(|b| format!("{:02x}", b as u8))
                    .collect();
                return Err(anyhow::anyhow!("Broadcast failed: {}", hex_message));
            }
        }

        Err(anyhow::anyhow!("Неизвестная ошибка broadcast"))
    }

    /// Конвертация base58 адреса в hex
    fn address_to_hex(&self, address: &str) -> Result<String> {
        if address.starts_with("0x") {
            return Ok(address.to_string());
        }

        // Декодируем base58 адрес
        let decoded = address
            .from_base58()
            .map_err(|_| anyhow::anyhow!("Неверный TRON адрес: {}", address))?;

        if decoded.len() < 21 {
            return Err(anyhow::anyhow!("Слишком короткий TRON адрес"));
        }

        // Берем первые 21 байт (без checksum)
        let hex_address = format!("0x{}", hex::encode(&decoded[..21]));
        Ok(hex_address)
    }

    /// Получает TRC20 транзакции для адреса
    pub async fn get_trc20_transactions(
        &self,
        address: &str,
        contract_address: &str,
        limit: u32,
    ) -> Result<Vec<BlockchainTransaction>> {
        let url = format!(
            "{}/v1/accounts/{}/transactions/trc20",
            self.config.base_url, address
        );

        let mut request = self.client.get(&url).query(&[
            ("limit", limit.to_string().as_str()),
            ("contract_address", contract_address),
            ("only_confirmed", "true"), // Только подтвержденные
        ]);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("TRON-PRO-API-KEY", api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("TronGrid API error: {}", response.status()));
        }

        let result: Value = response.json().await?;
        let mut transactions = Vec::new();

        if let Some(data) = result.get("data").and_then(|d| d.as_array()) {
            for tx_data in data {
                if let Ok(tx) = self.parse_trc20_transaction(tx_data) {
                    transactions.push(tx);
                }
            }
        }

        Ok(transactions)
    }

    /// Парсит TRC20 транзакцию из JSON
    fn parse_trc20_transaction(&self, tx_data: &Value) -> Result<BlockchainTransaction> {
        let tx_hash = tx_data
            .get("transaction_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Отсутствует transaction_id"))?
            .to_string();

        let block_number = tx_data.get("block_timestamp").and_then(|v| v.as_i64());

        let from_address = tx_data
            .get("from")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Отсутствует from"))?
            .to_string();

        let to_address = tx_data
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Отсутствует to"))?
            .to_string();

        let amount_str = tx_data
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Отсутствует value"))?;

        // Конвертируем из wei (6 decimals для USDT)
        let amount_wei = amount_str.parse::<u64>()?;
        let amount =
            Decimal::from(amount_wei) / Decimal::from(10u64.pow(self.config.usdt_decimals as u32));

        let timestamp_ms = tx_data
            .get("block_timestamp")
            .and_then(|v| v.as_i64())
            .unwrap_or(chrono::Utc::now().timestamp_millis());

        let timestamp = Utc
            .timestamp_millis_opt(timestamp_ms)
            .single()
            .unwrap_or_else(Utc::now);

        // В TRON сети после 19 подтверждений транзакция считается финальной
        let confirmations = if tx_data
            .get("confirmed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            19u32 // Считаем подтвержденным
        } else {
            0u32 // Неподтвержденная
        };

        Ok(BlockchainTransaction {
            tx_hash,
            block_number,
            from_address,
            to_address,
            amount,
            timestamp,
            confirmations,
        })
    }

    /// Получает информацию о транзакции по хэшу
    pub async fn get_transaction_info(
        &self,
        tx_hash: &str,
    ) -> Result<Option<BlockchainTransaction>> {
        // Шаг 1: Получаем информацию о транзакции (статус, блок)
        let info_url = format!("{}/wallet/gettransactioninfobyid", self.config.base_url);
        let info_body = serde_json::json!({ "value": tx_hash });

        let mut info_request = self.client.post(&info_url).json(&info_body);
        if let Some(api_key) = &self.config.api_key {
            info_request = info_request.header("TRON-PRO-API-KEY", api_key);
        }

        let info_response = info_request.send().await?;
        if !info_response.status().is_success() {
            return Ok(None);
        }

        let info_result: Value = info_response.json().await?;
        if info_result.is_null() || info_result.get("id").is_none() {
            return Ok(None);
        }

        // Шаг 2: Получаем детали транзакции
        let tx_url = format!("{}/wallet/gettransactionbyid", self.config.base_url);
        let tx_body = serde_json::json!({ "value": tx_hash });

        let mut tx_request = self.client.post(&tx_url).json(&tx_body);
        if let Some(api_key) = &self.config.api_key {
            tx_request = tx_request.header("TRON-PRO-API-KEY", api_key);
        }

        let tx_response = tx_request.send().await?;
        if !tx_response.status().is_success() {
            return Ok(None);
        }

        let tx_result: Value = tx_response.json().await?;
        if tx_result.is_null() {
            return Ok(None);
        }

        // Парсим данные транзакции
        let block_number = info_result.get("blockNumber").and_then(|v| v.as_i64());
        let block_timestamp = info_result
            .get("blockTimeStamp")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| Utc::now().timestamp_millis());

        let timestamp = Utc
            .timestamp_millis_opt(block_timestamp)
            .single()
            .unwrap_or_else(Utc::now);

        // Определяем подтверждения
        let confirmations = if block_number.is_some() {
            19u32 // Подтверждена (в блоке)
        } else {
            0u32 // В мемпуле
        };

        // Парсим адреса и сумму из contract данных
        let mut from_address = String::new();
        let mut to_address = String::new();
        let mut amount = Decimal::ZERO;

        // Ищем TRC20 трансфер в контрактах
        if let Some(raw_data) = tx_result
            .get("raw_data")
            .and_then(|rd| rd.get("contract"))
            .and_then(|contracts| contracts.as_array())
            .and_then(|arr| arr.first())
        {
            if let Some(parameter) = raw_data.get("parameter").and_then(|p| p.get("value")) {
                // Для TriggerSmartContract (TRC20)
                if let Some(owner_address) = parameter.get("owner_address").and_then(|v| v.as_str())
                {
                    from_address = self.hex_to_base58_safe(owner_address);
                }

                if let Some(contract_address) =
                    parameter.get("contract_address").and_then(|v| v.as_str())
                {
                    // Проверяем, что это USDT контракт
                    if contract_address.eq_ignore_ascii_case(
                        &self
                            .address_to_hex(&self.config.usdt_contract)
                            .unwrap_or_default(),
                    ) {
                        // Парсим data для получения to_address и amount
                        if let Some(data) = parameter.get("data").and_then(|v| v.as_str()) {
                            if let Ok((parsed_to, parsed_amount)) =
                                self.parse_trc20_transfer_data(data)
                            {
                                to_address = parsed_to;
                                amount = parsed_amount;
                            }
                        }
                    }
                }
            }
        }

        Ok(Some(BlockchainTransaction {
            tx_hash: tx_hash.to_string(),
            block_number,
            from_address,
            to_address,
            amount,
            timestamp,
            confirmations,
        }))
    }

    /// Конвертирует hex адрес в base58 с обработкой ошибок
    fn hex_to_base58_safe(&self, hex_address: &str) -> String {
        match self.hex_to_base58_address(hex_address) {
            Ok(addr) => addr,
            Err(_) => hex_address.to_string(), // Возвращаем hex если конвертация не удалась
        }
    }

    /// Конвертирует hex адрес в base58 (внутренний метод)
    fn hex_to_base58_address(&self, hex_address: &str) -> Result<String> {
        let hex_clean = hex_address.strip_prefix("0x").unwrap_or(hex_address);
        let hex_clean = if hex_clean.len() == 40 {
            format!("41{}", hex_clean) // Добавляем префикс mainnet
        } else {
            hex_clean.to_string()
        };

        let address_bytes =
            hex::decode(&hex_clean).map_err(|_| anyhow::anyhow!("Invalid hex address"))?;

        if address_bytes.len() != 21 {
            return Err(anyhow::anyhow!("Invalid address length"));
        }

        // Вычисляем checksum
        let hash1 = sha2::Sha256::digest(&address_bytes);
        let hash2 = sha2::Sha256::digest(&hash1);
        let checksum = &hash2[..4];

        let mut full_address = address_bytes;
        full_address.extend_from_slice(checksum);

        Ok(full_address.to_base58())
    }

    /// Парсит данные TRC20 transfer из hex
    fn parse_trc20_transfer_data(&self, data: &str) -> Result<(String, Decimal)> {
        if data.len() < 136 {
            // 4 bytes method + 32 bytes to + 32 bytes amount
            return Err(anyhow::anyhow!("Invalid TRC20 data length"));
        }

        // Пропускаем method signature (первые 8 символов)
        let data_hex = &data[8..];

        // Извлекаем to_address (следующие 64 символа, последние 40 - адрес)
        let to_hex = &data_hex[24..64]; // Пропускаем padding
        let to_address = self.hex_to_base58_safe(&format!("41{}", to_hex));

        // Извлекаем amount (следующие 64 символа)
        let amount_hex = &data_hex[64..128];
        let amount_wei = u128::from_str_radix(amount_hex, 16).unwrap_or(0);
        let amount =
            Decimal::from(amount_wei) / Decimal::from(10u64.pow(self.config.usdt_decimals as u32));

        Ok((to_address, amount))
    }
}
