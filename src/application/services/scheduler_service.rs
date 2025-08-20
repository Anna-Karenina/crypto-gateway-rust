//! # Сервис планировщика задач
//!
//! Выполняет периодические задачи (monitoring, cleanup, etc.)

use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use super::{TransactionMonitoringService, TransferService, WebhookService};

/// Конфигурация планировщика
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub monitoring_interval_seconds: u64,
    pub transfer_processing_interval_seconds: u64,
    pub cleanup_interval_hours: u64,
    pub health_check_interval_minutes: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            monitoring_interval_seconds: 30,          // Мониторинг каждые 30 сек
            transfer_processing_interval_seconds: 60, // Обработка pending каждую минуту
            cleanup_interval_hours: 24,               // Очистка каждые 24 часа
            health_check_interval_minutes: 5,         // Health check каждые 5 минут
        }
    }
}

/// Планировщик задач
pub struct TaskScheduler {
    config: SchedulerConfig,
    monitoring_service: Arc<TransactionMonitoringService>,
    transfer_service: Arc<TransferService>,
    webhook_service: Option<Arc<WebhookService>>,
}

impl TaskScheduler {
    /// Создает новый планировщик
    pub fn new(
        config: SchedulerConfig,
        monitoring_service: Arc<TransactionMonitoringService>,
        transfer_service: Arc<TransferService>,
        webhook_service: Option<Arc<WebhookService>>,
    ) -> Self {
        Self {
            config,
            monitoring_service,
            transfer_service,
            webhook_service,
        }
    }

    /// Запускает все фоновые задачи
    pub async fn start(&self) -> Result<()> {
        info!("🕒 Запуск планировщика задач...");

        // Запускаем все задачи параллельно
        tokio::try_join!(
            self.start_monitoring_task(),
            self.start_transfer_processing_task(),
            self.start_cleanup_task(),
            self.start_health_check_task()
        )?;

        Ok(())
    }

    /// Задача мониторинга входящих транзакций
    async fn start_monitoring_task(&self) -> Result<()> {
        info!(
            "📡 Запуск мониторинга транзакций (интервал: {} сек)",
            self.config.monitoring_interval_seconds
        );

        let mut interval = interval(Duration::from_secs(self.config.monitoring_interval_seconds));
        let monitoring_service = self.monitoring_service.clone();

        loop {
            interval.tick().await;

            if let Err(e) = monitoring_service.scan_for_incoming_transactions().await {
                error!("❌ Ошибка мониторинга транзакций: {}", e);
                // Продолжаем работу
            }
        }
    }

    /// Задача обработки pending трансферов
    async fn start_transfer_processing_task(&self) -> Result<()> {
        info!(
            "⚙️  Запуск обработки pending трансферов (интервал: {} сек)",
            self.config.transfer_processing_interval_seconds
        );

        let mut interval = interval(Duration::from_secs(
            self.config.transfer_processing_interval_seconds,
        ));
        let transfer_service = self.transfer_service.clone();

        loop {
            interval.tick().await;

            if let Err(e) = transfer_service.process_pending_transfers().await {
                error!("❌ Ошибка обработки pending трансферов: {}", e);
                // Продолжаем работу
            }
        }
    }

    /// Задача очистки старых данных
    async fn start_cleanup_task(&self) -> Result<()> {
        info!(
            "🧹 Запуск задачи очистки (интервал: {} часов)",
            self.config.cleanup_interval_hours
        );

        let mut interval = interval(Duration::from_secs(
            self.config.cleanup_interval_hours * 3600,
        ));

        loop {
            interval.tick().await;

            if let Err(e) = self.perform_cleanup().await {
                error!("❌ Ошибка очистки: {}", e);
            }
        }
    }

    /// Задача health check для внешних сервисов
    async fn start_health_check_task(&self) -> Result<()> {
        info!(
            "🏥 Запуск health check (интервал: {} минут)",
            self.config.health_check_interval_minutes
        );

        let mut interval = interval(Duration::from_secs(
            self.config.health_check_interval_minutes * 60,
        ));

        loop {
            interval.tick().await;

            if let Err(e) = self.perform_health_checks().await {
                warn!("⚠️  Health check warnings: {}", e);
            }
        }
    }

    /// Выполняет очистку старых данных
    async fn perform_cleanup(&self) -> Result<()> {
        info!("🧹 Выполнение очистки старых данных...");

        // Здесь можно добавить:
        // - Очистка старых логов
        // - Архивирование старых транзакций
        // - Очистка временных файлов
        // - Оптимизация БД

        // Пример: очистка транзакций старше 90 дней
        let cleanup_date = chrono::Utc::now() - chrono::Duration::days(90);

        info!(
            "Очистка данных старше {} (в реальной реализации)",
            cleanup_date.format("%Y-%m-%d")
        );

        // TODO: Реализовать реальную очистку
        // let mut conn = self.monitoring_service.db.get().await?;
        // diesel::delete(...)

        info!("✅ Очистка завершена");
        Ok(())
    }

    /// Выполняет health check внешних сервисов
    async fn perform_health_checks(&self) -> Result<()> {
        info!("🏥 Проверка состояния внешних сервисов...");

        // Health check для webhook
        if let Some(webhook_service) = &self.webhook_service {
            match webhook_service.health_check().await {
                Ok(true) => info!("✅ Webhook сервис работает"),
                Ok(false) => warn!("⚠️  Webhook сервис недоступен"),
                Err(e) => warn!("⚠️  Ошибка проверки webhook: {}", e),
            }
        }

        // Здесь можно добавить проверки:
        // - TRON Grid API доступность
        // - База данных подключение
        // - Внешние интеграции

        // Пример проверки статистики мониторинга
        match self.monitoring_service.get_monitoring_stats().await {
            Ok(stats) => {
                info!(
                    "📊 Статистика мониторинга: {} транзакций, {} pending",
                    stats.total_transactions, stats.pending_count
                );

                // Проверяем на аномалии
                if stats.pending_count > 100 {
                    warn!(
                        "⚠️  Слишком много pending транзакций: {}",
                        stats.pending_count
                    );
                }
            }
            Err(e) => warn!("⚠️  Не удалось получить статистику мониторинга: {}", e),
        }

        Ok(())
    }

    /// Запускает одну итерацию всех задач (для тестирования)
    pub async fn run_once(&self) -> Result<()> {
        info!("🔄 Выполнение одной итерации всех задач...");

        // Мониторинг
        if let Err(e) = self
            .monitoring_service
            .scan_for_incoming_transactions()
            .await
        {
            warn!("Мониторинг: {}", e);
        }

        // Обработка трансферов
        if let Err(e) = self.transfer_service.process_pending_transfers().await {
            warn!("Обработка трансферов: {}", e);
        }

        // Health check
        if let Err(e) = self.perform_health_checks().await {
            warn!("Health check: {}", e);
        }

        info!("✅ Итерация завершена");
        Ok(())
    }

    /// Получает статистику работы планировщика
    pub async fn get_scheduler_stats(&self) -> Result<SchedulerStats> {
        let monitoring_stats = self.monitoring_service.get_monitoring_stats().await?;

        Ok(SchedulerStats {
            monitoring_enabled: monitoring_stats.monitoring_enabled,
            total_transactions: monitoring_stats.total_transactions,
            pending_transactions: monitoring_stats.pending_count,
            config: self.config.clone(),
        })
    }
}

/// Статистика планировщика
#[derive(Debug, Clone, serde::Serialize)]
pub struct SchedulerStats {
    pub monitoring_enabled: bool,
    pub total_transactions: i64,
    pub pending_transactions: i64,
    pub config: SchedulerConfig,
}

// Нужно добавить Serialize для SchedulerConfig
impl serde::Serialize for SchedulerConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SchedulerConfig", 4)?;
        state.serialize_field(
            "monitoring_interval_seconds",
            &self.monitoring_interval_seconds,
        )?;
        state.serialize_field(
            "transfer_processing_interval_seconds",
            &self.transfer_processing_interval_seconds,
        )?;
        state.serialize_field("cleanup_interval_hours", &self.cleanup_interval_hours)?;
        state.serialize_field(
            "health_check_interval_minutes",
            &self.health_check_interval_minutes,
        )?;
        state.end()
    }
}
