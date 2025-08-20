//! # Единый сервис расчета комиссий
//!
//! Объединяет статические и динамические комиссии для USDT трансферов

use anyhow::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::{info, warn};

use crate::infrastructure::tron::TronGridClient;

/// Конфигурация расчета комиссий
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeConfig {
    // Базовые настройки комиссий
    pub base_trx_per_transaction: Decimal,
    pub trx_to_usdt_rate: Decimal,
    pub commission_percentage: Decimal,
    pub min_commission_usdt: Decimal,
    pub max_commission_usdt: Decimal,

    // Динамические комиссии
    pub dynamic_fees_enabled: bool,
    pub dynamic_min_fee: Decimal,
    pub dynamic_max_fee: Decimal,
    pub network_congestion_multiplier: Decimal,
    pub update_interval_minutes: u64,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            // Базовые настройки
            base_trx_per_transaction: Decimal::new(15, 0), // 15 TRX
            trx_to_usdt_rate: Decimal::new(10, 2),         // 0.10 USDT за TRX
            commission_percentage: Decimal::new(5, 1),     // 0.5%
            min_commission_usdt: Decimal::new(1, 0),       // 1 USDT
            max_commission_usdt: Decimal::new(10, 0),      // 10 USDT

            // Динамические настройки
            dynamic_fees_enabled: true,
            dynamic_min_fee: Decimal::new(10, 0), // 10 TRX минимум
            dynamic_max_fee: Decimal::new(50, 0), // 50 TRX максимум
            network_congestion_multiplier: Decimal::new(15, 1), // 1.5x при загрузке
            update_interval_minutes: 5,           // Обновление каждые 5 минут
        }
    }
}

/// Состояние сети TRON
#[derive(Debug, Clone, Serialize)]
pub struct NetworkState {
    pub timestamp: u64,
    pub energy_price: Decimal,
    pub bandwidth_price: Decimal,
    pub congestion_level: CongestionLevel,
    pub recommended_fee_trx: Decimal,
}

/// Уровень загрузки сети
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CongestionLevel {
    Low,    // Низкая загрузка
    Medium, // Средняя загрузка
    High,   // Высокая загрузка
}

/// Результат расчета комиссий
#[derive(Debug, Clone, Serialize)]
pub struct FeeCalculationResult {
    pub gas_cost_usdt: Decimal,
    pub percentage_commission: Decimal,
    pub final_commission: Decimal,
    pub total_amount: Decimal,
    pub gas_fee_source: FeeSource,
}

/// Источник расчета комиссии за газ
#[derive(Debug, Clone, Serialize)]
pub enum FeeSource {
    Static,   // Статический расчет
    Dynamic,  // Динамический на основе сети
    Fallback, // Резервный (при ошибке API)
}

/// Единый сервис расчета комиссий
#[derive(Clone)]
pub struct UnifiedFeeService {
    config: FeeConfig,
    tron_client: TronGridClient,
    master_wallet_address: String,
    network_state: Option<NetworkState>,
}

impl UnifiedFeeService {
    /// Создает новый экземпляр сервиса
    pub fn new(
        config: FeeConfig,
        tron_client: TronGridClient,
        master_wallet_address: String,
    ) -> Self {
        Self {
            config,
            tron_client,
            master_wallet_address,
            network_state: None,
        }
    }

    /// Создает из старой конфигурации (для обратной совместимости)
    pub fn from_legacy_config(
        tron_client: TronGridClient,
        trx_per_transaction: Decimal,
        trx_to_usdt_rate: Decimal,
        commission_percentage: Decimal,
        min_commission_usdt: Decimal,
        max_commission_usdt: Decimal,
        master_wallet_address: String,
    ) -> Self {
        let config = FeeConfig {
            base_trx_per_transaction: trx_per_transaction,
            trx_to_usdt_rate,
            commission_percentage,
            min_commission_usdt,
            max_commission_usdt,
            ..Default::default()
        };

        Self::new(config, tron_client, master_wallet_address)
    }

    /// Запускает фоновое обновление состояния сети (если включены динамические комиссии)
    pub async fn start_background_updates(&mut self) -> Result<()> {
        if !self.config.dynamic_fees_enabled {
            info!("💰 Динамические комиссии отключены, используется статический расчет");
            return Ok(());
        }

        info!(
            "💰 Запуск динамических комиссий (интервал: {} мин)",
            self.config.update_interval_minutes
        );

        let mut update_interval = interval(Duration::from_secs(
            self.config.update_interval_minutes * 60,
        ));

        loop {
            update_interval.tick().await;

            if let Err(e) = self.update_network_state().await {
                warn!("⚠️  Ошибка обновления состояния сети: {}", e);
                // Продолжаем работу с статическими комиссиями
            }
        }
    }

    /// Основной метод расчета комиссии за газ в USDT
    pub async fn calculate_gas_fee(
        &mut self,
        from: &str,
        _to: &str,
        amount: Decimal,
    ) -> Result<Decimal> {
        // Пытаемся использовать динамический расчет
        if self.config.dynamic_fees_enabled {
            match self.get_dynamic_gas_fee().await {
                Ok(fee) => {
                    tracing::info!("💰 Динамическая комиссия за газ: {} USDT", fee);
                    return Ok(fee);
                }
                Err(e) => {
                    warn!(
                        "⚠️  Динамический расчет неудачен, используем статический: {}",
                        e
                    );
                }
            }
        }

        // Статический расчет (fallback)
        self.calculate_static_gas_fee(from, amount).await
    }

    /// Динамический расчет комиссии за газ
    async fn get_dynamic_gas_fee(&mut self) -> Result<Decimal> {
        // Обновляем состояние сети если нужно
        if self.network_state.is_none() {
            self.update_network_state().await?;
        }

        // Проверяем свежесть данных (не старше 10 минут)
        if let Some(state) = &self.network_state {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            if now - state.timestamp > 600 {
                // 10 минут
                self.update_network_state().await?;
            }
        }

        let fee_trx = self
            .network_state
            .as_ref()
            .map(|s| s.recommended_fee_trx)
            .unwrap_or(self.config.base_trx_per_transaction);

        // Конвертируем TRX в USDT
        Ok(fee_trx * self.config.trx_to_usdt_rate)
    }

    /// Статический расчет комиссии за газ
    async fn calculate_static_gas_fee(&self, from: &str, amount: Decimal) -> Result<Decimal> {
        // Пытаемся получить реальную оценку энергии
        match self
            .tron_client
            .estimate_energy(from, &self.master_wallet_address, amount)
            .await
        {
            Ok(energy_cost) => {
                tracing::info!("💰 Реальная оценка энергии: {} TRX", energy_cost);
                let gas_cost_usdt = Decimal::from(energy_cost) * self.config.trx_to_usdt_rate;
                Ok(gas_cost_usdt)
            }
            Err(e) => {
                tracing::warn!(
                    "Не удалось получить реальную стоимость газа, используем конфигурированное значение: {}",
                    e
                );
                // Используем конфигурированное базовое значение
                let base_cost = self.config.base_trx_per_transaction * self.config.trx_to_usdt_rate;
                Ok(base_cost)
            }
        }
    }

    /// Расчет процентной комиссии
    pub fn calculate_percentage_commission(&self, amount: Decimal) -> Decimal {
        let commission = amount * self.config.commission_percentage / Decimal::new(100, 0);

        // Применяем ограничения
        commission
            .max(self.config.min_commission_usdt)
            .min(self.config.max_commission_usdt)
    }

    /// Полный расчет всех комиссий и итоговой суммы
    pub async fn calculate_total_amount(
        &mut self,
        order_amount: Decimal,
        from_wallet_address: &str,
    ) -> Result<(Decimal, Decimal, Decimal, Decimal)> {
        // 1. Газовая комиссия (клонируем мастер адрес)
        let master_wallet_address = self.master_wallet_address.clone();
        let gas_cost_usdt = self
            .calculate_gas_fee(from_wallet_address, &master_wallet_address, order_amount)
            .await?;

        // 2. Процентная комиссия
        let percentage_commission = self.calculate_percentage_commission(order_amount);

        // 3. Итоговая комиссия (пока равна процентной, можно добавить другие)
        let final_commission = percentage_commission;

        // 4. Общая сумма для списания
        let total_amount = order_amount + gas_cost_usdt + final_commission;

        Ok((
            gas_cost_usdt,
            percentage_commission,
            final_commission,
            total_amount,
        ))
    }

    /// Подробный расчет комиссий с указанием источника
    pub async fn calculate_detailed_fees(
        &mut self,
        order_amount: Decimal,
        from_wallet_address: &str,
    ) -> Result<FeeCalculationResult> {
        let (gas_cost_usdt, percentage_commission, final_commission, total_amount) = self
            .calculate_total_amount(order_amount, from_wallet_address)
            .await?;

        let fee_source = if self.config.dynamic_fees_enabled && self.network_state.is_some() {
            FeeSource::Dynamic
        } else {
            FeeSource::Static
        };

        Ok(FeeCalculationResult {
            gas_cost_usdt,
            percentage_commission,
            final_commission,
            total_amount,
            gas_fee_source: fee_source,
        })
    }

    /// Обновляет состояние сети
    async fn update_network_state(&mut self) -> Result<()> {
        info!("📊 Обновление состояния TRON сети...");

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        // Получаем метрики сети
        let (energy_price, bandwidth_price, congestion_level) =
            self.fetch_network_metrics().await?;

        // Рассчитываем рекомендуемую комиссию
        let recommended_fee_trx = self.calculate_dynamic_fee(&congestion_level, energy_price);

        let new_state = NetworkState {
            timestamp,
            energy_price,
            bandwidth_price,
            congestion_level,
            recommended_fee_trx,
        };

        info!(
            "📊 Состояние сети: {:?}, рекомендуемая комиссия: {} TRX",
            new_state.congestion_level, new_state.recommended_fee_trx
        );

        self.network_state = Some(new_state);
        Ok(())
    }

    /// Получает метрики сети (упрощенная реализация)
    async fn fetch_network_metrics(&self) -> Result<(Decimal, Decimal, CongestionLevel)> {
        // Симуляция: генерируем данные на основе времени
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let cycle = (now % 300) as f64 / 300.0; // 5-минутный цикл

        let energy_price = Decimal::new(420, 0) + // базовая цена 420 sun
            Decimal::new((cycle * 200.0) as i64, 0); // +0-200 sun вариация

        let bandwidth_price = Decimal::new(1000, 0); // 1000 sun за bandwidth

        let congestion_level = match cycle {
            x if x < 0.3 => CongestionLevel::Low,
            x if x < 0.7 => CongestionLevel::Medium,
            _ => CongestionLevel::High,
        };

        Ok((energy_price, bandwidth_price, congestion_level))
    }

    /// Рассчитывает динамическую комиссию
    fn calculate_dynamic_fee(
        &self,
        congestion_level: &CongestionLevel,
        energy_price: Decimal,
    ) -> Decimal {
        let base_fee = self.config.base_trx_per_transaction;

        // Корректировка на основе загрузки сети
        let congestion_multiplier = match congestion_level {
            CongestionLevel::Low => Decimal::new(8, 1),     // 0.8x
            CongestionLevel::Medium => Decimal::new(10, 1), // 1.0x
            CongestionLevel::High => self.config.network_congestion_multiplier, // 1.5x
        };

        // Корректировка на основе цены энергии
        let energy_multiplier = if energy_price > Decimal::new(500, 0) {
            Decimal::new(12, 1) // 1.2x если энергия дорогая
        } else {
            Decimal::new(10, 1) // 1.0x
        };

        let calculated_fee = base_fee * congestion_multiplier * energy_multiplier;

        // Применяем ограничения
        calculated_fee
            .max(self.config.dynamic_min_fee)
            .min(self.config.dynamic_max_fee)
    }

    /// Получает конфигурацию
    pub fn get_config(&self) -> &FeeConfig {
        &self.config
    }

    /// Получает текущее состояние сети
    pub fn get_network_state(&self) -> Option<&NetworkState> {
        self.network_state.as_ref()
    }

    /// Получает статистику по комиссиям
    pub fn get_fee_stats(&self) -> FeeStats {
        FeeStats {
            config: self.config.clone(),
            network_state: self.network_state.clone(),
            dynamic_fees_active: self.config.dynamic_fees_enabled && self.network_state.is_some(),
        }
    }
}

/// Статистика комиссий
#[derive(Debug, Clone, Serialize)]
pub struct FeeStats {
    pub config: FeeConfig,
    pub network_state: Option<NetworkState>,
    pub dynamic_fees_active: bool,
}
