//! # Retry логика
//!
//! Обеспечивает надежность при вызовах внешних API

use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, warn};

/// Конфигурация retry логики
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

/// Типы ошибок для определения стратегии retry
#[derive(Debug)]
pub enum RetryableError {
    /// Временная ошибка - можно повторить
    Temporary(String),
    /// Ошибка сети - можно повторить
    Network(String),
    /// Ошибка API лимитов - можно повторить с задержкой
    RateLimit(String),
    /// Постоянная ошибка - не повторять
    Permanent(String),
}

impl RetryableError {
    /// Определяет можно ли повторить операцию
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Temporary(_) | Self::Network(_) | Self::RateLimit(_)
        )
    }

    /// Возвращает дополнительную задержку для rate limit
    pub fn additional_delay(&self) -> Duration {
        match self {
            Self::RateLimit(_) => Duration::from_secs(5), // Дополнительная задержка для rate limit
            _ => Duration::ZERO,
        }
    }
}

impl std::fmt::Display for RetryableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Temporary(msg) => write!(f, "Временная ошибка: {}", msg),
            Self::Network(msg) => write!(f, "Сетевая ошибка: {}", msg),
            Self::RateLimit(msg) => write!(f, "Превышен лимит API: {}", msg),
            Self::Permanent(msg) => write!(f, "Постоянная ошибка: {}", msg),
        }
    }
}

impl std::error::Error for RetryableError {}

/// Wrapper для retry логики
#[derive(Clone)]
pub struct RetryableService<T> {
    inner: T,
    config: RetryConfig,
}

impl<T> RetryableService<T> {
    /// Создает новый wrapper с конфигурацией по умолчанию
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            config: RetryConfig::default(),
        }
    }

    /// Создает новый wrapper с кастомной конфигурацией
    pub fn with_config(inner: T, config: RetryConfig) -> Self {
        Self { inner, config }
    }

    /// Получает ссылку на внутренний сервис
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Выполняет операцию с retry логикой
    pub async fn retry<F, Fut, R, E>(&self, operation_name: &str, operation: F) -> Result<R>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<R, E>>,
        E: Into<RetryableError> + std::fmt::Display,
    {
        let mut attempt = 0;
        let mut delay = self.config.initial_delay;

        loop {
            attempt += 1;

            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        tracing::info!(
                            "✅ {} выполнена успешно с попытки {}",
                            operation_name,
                            attempt
                        );
                    }
                    return Ok(result);
                }
                Err(error) => {
                    let retry_error: RetryableError = error.into();

                    if !retry_error.is_retryable() || attempt >= self.config.max_attempts {
                        error!(
                            "❌ {} не удалась после {} попыток: {}",
                            operation_name, attempt, retry_error
                        );
                        return Err(anyhow::anyhow!("{}", retry_error));
                    }

                    let total_delay = delay + retry_error.additional_delay();

                    warn!(
                        "⚠️  {} не удалась (попытка {}/{}): {}. Повтор через {:?}",
                        operation_name, attempt, self.config.max_attempts, retry_error, total_delay
                    );

                    sleep(total_delay).await;

                    // Exponential backoff
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * self.config.backoff_multiplier) as u64,
                        ),
                        self.config.max_delay,
                    );
                }
            }
        }
    }
}

/// Помощник для классификации HTTP ошибок
pub fn classify_http_error(status: reqwest::StatusCode, message: &str) -> RetryableError {
    match status.as_u16() {
        // 4xx клиентские ошибки (обычно не повторяем)
        400..=499 => match status.as_u16() {
            408 | 429 => RetryableError::RateLimit(format!("HTTP {}: {}", status, message)),
            _ => RetryableError::Permanent(format!("HTTP {}: {}", status, message)),
        },
        // 5xx серверные ошибки (повторяем)
        500..=599 => RetryableError::Temporary(format!("HTTP {}: {}", status, message)),
        // Другие коды
        _ => RetryableError::Network(format!("HTTP {}: {}", status, message)),
    }
}

/// Помощник для классификации ошибок reqwest
pub fn classify_reqwest_error(error: reqwest::Error) -> RetryableError {
    if error.is_timeout() {
        RetryableError::Network("Timeout".to_string())
    } else if error.is_connect() {
        RetryableError::Network("Connection failed".to_string())
    } else if let Some(status) = error.status() {
        classify_http_error(status, &error.to_string())
    } else {
        RetryableError::Temporary(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
    }

    #[test]
    fn test_retryable_error_classification() {
        assert!(RetryableError::Temporary("test".to_string()).is_retryable());
        assert!(RetryableError::Network("test".to_string()).is_retryable());
        assert!(RetryableError::RateLimit("test".to_string()).is_retryable());
        assert!(!RetryableError::Permanent("test".to_string()).is_retryable());
    }

    #[test]
    fn test_http_error_classification() {
        let error_400 = classify_http_error(reqwest::StatusCode::BAD_REQUEST, "bad request");
        assert!(!error_400.is_retryable());

        let error_429 = classify_http_error(reqwest::StatusCode::TOO_MANY_REQUESTS, "rate limit");
        assert!(error_429.is_retryable());

        let error_500 =
            classify_http_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "server error");
        assert!(error_500.is_retryable());
    }
}
