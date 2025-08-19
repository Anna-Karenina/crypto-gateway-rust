//! # Сервис расчета комиссий
//!
//! Рассчитывает газовые и сервисные комиссии для USDT трансферов

use anyhow::Result;
use rust_decimal::Decimal;

use crate::infrastructure::tron::TronGridClient;

/// Сервис для расчета комиссий и газа (FeeCalculationService)
#[derive(Clone)]
pub struct FeeCalculationService {
    tron_client: TronGridClient,
    trx_per_transaction: Decimal,
    trx_to_usdt_rate: Decimal,
    commission_percentage: Decimal,
    min_commission_usdt: Decimal,
    max_commission_usdt: Decimal,
    master_wallet_address: String,
}

impl FeeCalculationService {
    /// Создает новый экземпляр сервиса
    pub fn new(
        tron_client: TronGridClient,
        trx_per_transaction: Decimal,
        trx_to_usdt_rate: Decimal,
        commission_percentage: Decimal,
        min_commission_usdt: Decimal,
        max_commission_usdt: Decimal,
        master_wallet_address: String,
    ) -> Self {
        Self {
            tron_client,
            trx_per_transaction,
            trx_to_usdt_rate,
            commission_percentage,
            min_commission_usdt,
            max_commission_usdt,
            master_wallet_address,
        }
    }

    /// Расчет комиссии за газ в USDT
    pub async fn calculate_gas_fee(
        &self,
        from: &str,
        _to: &str,
        amount: Decimal,
    ) -> Result<Decimal> {
        // Пытаемся получить реальную стоимость газа из TRON сети
        match self.get_real_gas_cost(from, amount).await {
            Ok(gas_cost) => Ok(gas_cost),
            Err(e) => {
                tracing::warn!(
                    "Не удалось получить реальную стоимость газа, используем конфигурированное значение: {}",
                    e
                );
                // Используем конфигурированное базовое значение
                let base_cost = self.trx_per_transaction * self.trx_to_usdt_rate;
                Ok(base_cost)
            }
        }
    }

    /// Получение реальной стоимости газа из TRON сети
    async fn get_real_gas_cost(&self, from_address: &str, amount: Decimal) -> Result<Decimal> {
        tracing::debug!(
            "Оцениваем реальную стоимость газа для {} USDT от {}",
            amount,
            from_address
        );

        // Получаем реальную оценку энергии из TRON сети
        let estimated_energy = self
            .tron_client
            .estimate_energy(from_address, &self.master_wallet_address, amount)
            .await?;

        // Конвертируем энергию в TRX (используем базовое значение как минимум)
        let real_trx_cost = Decimal::from(estimated_energy) / Decimal::from(1000); // Упрощенная формула
        let real_trx_cost = real_trx_cost.max(self.trx_per_transaction);

        tracing::debug!("Реальная стоимость газа из сети: {} TRX", real_trx_cost);

        // Конвертируем TRX в USDT
        let gas_cost_usdt = real_trx_cost * self.trx_to_usdt_rate;

        tracing::info!(
            "Реальная стоимость газа: {} TRX = {} USDT (энергия: {})",
            real_trx_cost,
            gas_cost_usdt,
            estimated_energy
        );

        Ok(gas_cost_usdt)
    }

    /// Расчет сервисной комиссии
    pub fn calculate_service_fee(&self, order_amount: Decimal) -> Decimal {
        // Процентная комиссия
        let percentage_fee = order_amount * (self.commission_percentage / Decimal::from(100));

        // Применяем минимальные и максимальные лимиты
        percentage_fee
            .max(self.min_commission_usdt)
            .min(self.max_commission_usdt)
    }

    /// Полный расчет суммы (газ + процентная комиссия)
    pub async fn calculate_total_amount(
        &self,
        order_amount: Decimal,
        from_address: &str,
    ) -> Result<(Decimal, Decimal, Decimal, Decimal)> {
        // 1. Получаем реальную стоимость газа
        let gas_cost_usdt = self
            .get_real_gas_cost(from_address, order_amount)
            .await
            .unwrap_or_else(|_| self.trx_per_transaction * self.trx_to_usdt_rate);

        // 2. Вычисляем процентную комиссию
        let percentage_commission =
            order_amount * (self.commission_percentage / Decimal::from(100));

        // 3. Берем максимум между газом и процентной комиссией
        let commission = gas_cost_usdt.max(percentage_commission);

        // 4. Применяем мин/макс лимиты
        let final_commission = commission
            .max(self.min_commission_usdt)
            .min(self.max_commission_usdt);

        // 5. Итоговая сумма
        let total_amount = order_amount + final_commission;

        Ok((
            gas_cost_usdt,
            percentage_commission,
            final_commission,
            total_amount,
        ))
    }

    /// Получение курса TRX/USDT
    pub fn get_trx_to_usdt_rate(&self) -> Decimal {
        self.trx_to_usdt_rate
    }
}
