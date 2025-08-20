//! # Сервис webhook уведомлений
//!
//! Отправляет уведомления о транзакциях на внешние endpoints

use anyhow::Result;
use hmac::Mac;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, warn};

use crate::domain::TransactionStatus;
use crate::infrastructure::retry::{classify_reqwest_error, RetryConfig, RetryableService};

/// Конфигурация webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub enabled: bool,
    pub url: String,
    pub timeout_seconds: u64,
    pub secret_key: Option<String>, // Для подписи payload
}

/// Типы webhook событий
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    IncomingTransaction,
    OutgoingTransfer,
    TransferCompleted,
    TransferFailed,
    WalletCreated,
    WalletActivated,
}

/// Данные webhook события
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event_type: WebhookEventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: WebhookData,
}

/// Различные типы данных для webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WebhookData {
    IncomingTransaction {
        wallet_id: i64,
        wallet_address: String,
        tx_hash: String,
        from_address: String,
        amount: String, // Decimal as string
        status: TransactionStatus,
    },
    OutgoingTransfer {
        transfer_id: i64,
        wallet_id: i64,
        wallet_address: String,
        to_address: String,
        amount: String,
        reference_id: Option<String>,
        status: TransactionStatus,
        tx_hash: Option<String>,
    },
    WalletCreated {
        wallet_id: i64,
        wallet_address: String,
        owner_id: Option<String>,
    },
    WalletActivated {
        wallet_address: String,
        activation_amount: String,
        activation_tx_hash: String,
    },
}

/// Сервис webhook уведомлений
#[derive(Clone)]
pub struct WebhookService {
    config: WebhookConfig,
    client: Client,
    retry_service: RetryableService<()>,
}

impl WebhookService {
    /// Создает новый экземпляр сервиса
    pub fn new(config: WebhookConfig) -> Self {
        // Настройка retry для webhook
        let retry_config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        };

        Self {
            config,
            client: Client::new(),
            retry_service: RetryableService::with_config((), retry_config),
        }
    }

    /// Отправляет webhook уведомление о входящей транзакции
    pub async fn notify_incoming_transaction(
        &self,
        wallet_id: i64,
        wallet_address: String,
        tx_hash: String,
        from_address: String,
        amount: Decimal,
        status: TransactionStatus,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let payload = WebhookPayload {
            event_type: WebhookEventType::IncomingTransaction,
            timestamp: chrono::Utc::now(),
            data: WebhookData::IncomingTransaction {
                wallet_id,
                wallet_address,
                tx_hash,
                from_address,
                amount: amount.to_string(),
                status,
            },
        };

        self.send_webhook(payload).await
    }

    /// Отправляет webhook уведомление об исходящем трансфере
    pub async fn notify_outgoing_transfer(
        &self,
        transfer_id: i64,
        wallet_id: i64,
        wallet_address: String,
        to_address: String,
        amount: Decimal,
        reference_id: Option<String>,
        status: TransactionStatus,
        tx_hash: Option<String>,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let payload = WebhookPayload {
            event_type: WebhookEventType::OutgoingTransfer,
            timestamp: chrono::Utc::now(),
            data: WebhookData::OutgoingTransfer {
                transfer_id,
                wallet_id,
                wallet_address,
                to_address,
                amount: amount.to_string(),
                reference_id,
                status,
                tx_hash,
            },
        };

        self.send_webhook(payload).await
    }

    /// Отправляет webhook уведомление о создании кошелька
    pub async fn notify_wallet_created(
        &self,
        wallet_id: i64,
        wallet_address: String,
        owner_id: Option<String>,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let payload = WebhookPayload {
            event_type: WebhookEventType::WalletCreated,
            timestamp: chrono::Utc::now(),
            data: WebhookData::WalletCreated {
                wallet_id,
                wallet_address,
                owner_id,
            },
        };

        self.send_webhook(payload).await
    }

    /// Отправляет webhook уведомление об активации кошелька
    pub async fn notify_wallet_activated(
        &self,
        wallet_address: String,
        activation_amount: Decimal,
        activation_tx_hash: String,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let payload = WebhookPayload {
            event_type: WebhookEventType::WalletActivated,
            timestamp: chrono::Utc::now(),
            data: WebhookData::WalletActivated {
                wallet_address,
                activation_amount: activation_amount.to_string(),
                activation_tx_hash,
            },
        };

        self.send_webhook(payload).await
    }

    /// Внутренний метод для отправки webhook с retry логикой
    async fn send_webhook(&self, payload: WebhookPayload) -> Result<()> {
        let config = self.config.clone();
        let client = self.client.clone();
        let payload_json = serde_json::to_string(&payload)?;

        self.retry_service
            .retry("send_webhook", || {
                let config = config.clone();
                let client = client.clone();
                let payload_json = payload_json.clone();

                async move {
                    let mut request = client
                        .post(&config.url)
                        .header("Content-Type", "application/json")
                        .header("User-Agent", "TRON-Gateway-Webhook/1.0");

                    // Добавляем подпись если есть secret key
                    if let Some(secret_key) = &config.secret_key {
                        let signature = format!(
                            "sha256={}",
                            hex::encode(
                                hmac::Hmac::<sha2::Sha256>::new_from_slice(secret_key.as_bytes())
                                    .expect("HMAC can take key of any size")
                                    .chain_update(payload_json.as_bytes())
                                    .finalize()
                                    .into_bytes()
                            )
                        );
                        request = request.header("X-Webhook-Signature", signature);
                    }

                    let request = request.body(payload_json);

                    let request_timeout = Duration::from_secs(config.timeout_seconds);

                    let response = timeout(request_timeout, request.send())
                        .await
                        .map_err(|_| {
                            crate::infrastructure::retry::RetryableError::Network(
                                "Webhook timeout".to_string(),
                            )
                        })?
                        .map_err(classify_reqwest_error)?;

                    if response.status().is_success() {
                        info!("✅ Webhook отправлен успешно на {}", config.url);
                        Ok(())
                    } else {
                        let status = response.status();
                        let error_text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());

                        Err(crate::infrastructure::retry::classify_http_error(
                            status,
                            &error_text,
                        ))
                    }
                }
            })
            .await
            .map_err(|e| {
                error!("❌ Webhook не удалось отправить: {}", e);
                e
            })
    }

    /// Вычисляет HMAC подпись для webhook payload
    fn calculate_signature(&self, payload: &str, secret_key: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());

        let result = mac.finalize();
        format!("sha256={}", hex::encode(result.into_bytes()))
    }

    /// Проверяет работоспособность webhook endpoint
    pub async fn health_check(&self) -> Result<bool> {
        if !self.config.enabled {
            return Ok(true);
        }

        let test_payload = WebhookPayload {
            event_type: WebhookEventType::IncomingTransaction,
            timestamp: chrono::Utc::now(),
            data: WebhookData::IncomingTransaction {
                wallet_id: 0,
                wallet_address: "TEST".to_string(),
                tx_hash: "TEST".to_string(),
                from_address: "TEST".to_string(),
                amount: "0".to_string(),
                status: TransactionStatus::Pending,
            },
        };

        // Добавляем заголовок для обозначения тестового запроса
        let request = self
            .client
            .post(&self.config.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Test", "true")
            .json(&test_payload);

        let request_timeout = Duration::from_secs(self.config.timeout_seconds);

        match timeout(request_timeout, request.send()).await {
            Ok(Ok(response)) => {
                let success = response.status().is_success();
                if success {
                    info!("✅ Webhook health check прошел успешно");
                } else {
                    warn!("⚠️  Webhook health check неуспешен: {}", response.status());
                }
                Ok(success)
            }
            Ok(Err(e)) => {
                warn!("⚠️  Webhook health check ошибка: {}", e);
                Ok(false)
            }
            Err(_) => {
                warn!("⚠️  Webhook health check timeout");
                Ok(false)
            }
        }
    }
}
