//! # TRON Grid API клиент
//!
//! HTTP клиент для взаимодействия с TRON сетью через TronGrid API

use anyhow::Result;
use base58::FromBase58;
use reqwest::Client;
use rust_decimal::prelude::*;
use serde_json::Value;

use crate::config::TronConfig;

/// Клиент для взаимодействия с TronGrid API
#[derive(Clone)]
pub struct TronGridClient {
    client: Client,
    config: TronConfig,
}

impl TronGridClient {
    /// Создает новый экземпляр клиента
    pub fn new(config: TronConfig) -> Self {
        Self {
            client: Client::new(),
            config,
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
}
