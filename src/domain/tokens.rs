//! # TRC-20 Token Domain
//!
//! Доменная модель для работы с различными TRC-20 токенами

use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Информация о TRC-20 токене
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub symbol: String,                       // USDT, USDC, BTT, etc.
    pub name: String,                         // Tether USD, USD Coin, etc.
    pub contract_address: String,             // TRC-20 контракт адрес
    pub decimals: u8,                         // Количество знаков после запятой
    pub is_stable: bool,                      // Является ли стейблкоином
    pub min_transfer_amount: Decimal,         // Минимальная сумма для трансфера
    pub max_transfer_amount: Option<Decimal>, // Максимальная сумма (если есть)
    pub enabled: bool,                        // Включен ли токен для приема
    pub icon_url: Option<String>,             // URL иконки токена
    pub coingecko_id: Option<String>,         // ID для получения курса
}

impl TokenInfo {
    /// Конвертирует сумму в минимальные единицы (wei)
    pub fn to_wei(&self, amount: Decimal) -> anyhow::Result<u64> {
        let multiplier = Decimal::from(10u64.pow(self.decimals as u32));
        let amount_wei = amount * multiplier;
        amount_wei
            .to_u64()
            .ok_or_else(|| anyhow::anyhow!("Сумма {} слишком большая для {}", amount, self.symbol))
    }

    /// Конвертирует из минимальных единиц (wei) в нормальную сумму
    pub fn from_wei(&self, amount_wei: u64) -> Decimal {
        let divisor = Decimal::from(10u64.pow(self.decimals as u32));
        Decimal::from(amount_wei) / divisor
    }

    /// Проверяет валидность суммы для трансфера
    pub fn validate_amount(&self, amount: Decimal) -> Result<(), String> {
        if amount < self.min_transfer_amount {
            return Err(format!(
                "Сумма {} {} меньше минимальной ({})",
                amount, self.symbol, self.min_transfer_amount
            ));
        }

        if let Some(max_amount) = self.max_transfer_amount {
            if amount > max_amount {
                return Err(format!(
                    "Сумма {} {} превышает максимальную ({})",
                    amount, self.symbol, max_amount
                ));
            }
        }

        Ok(())
    }
}

/// Реестр поддерживаемых токенов
#[derive(Debug, Clone)]
pub struct TokenRegistry {
    tokens: HashMap<String, TokenInfo>,
    primary_token: String, // Основной токен (обычно USDT)
}

impl TokenRegistry {
    /// Создает новый реестр с базовыми токенами
    pub fn new() -> Self {
        let mut tokens = HashMap::new();

        // USDT (основной)
        tokens.insert(
            "USDT".to_string(),
            TokenInfo {
                symbol: "USDT".to_string(),
                name: "Tether USD".to_string(),
                contract_address: "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t".to_string(), // mainnet
                decimals: 6,
                is_stable: true,
                min_transfer_amount: Decimal::new(1, 0), // 1 USDT
                max_transfer_amount: Some(Decimal::new(1_000_000, 0)), // 1M USDT
                enabled: true,
                icon_url: Some(
                    "https://assets.coingecko.com/coins/images/325/small/Tether.png".to_string(),
                ),
                coingecko_id: Some("tether".to_string()),
            },
        );

        // USDC
        tokens.insert(
            "USDC".to_string(),
            TokenInfo {
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                contract_address: "TEkxiTehnzSmSe2XqrBj4w32RUN966rdz8".to_string(), // mainnet
                decimals: 6,
                is_stable: true,
                min_transfer_amount: Decimal::new(1, 0),
                max_transfer_amount: Some(Decimal::new(1_000_000, 0)),
                enabled: false, // По умолчанию отключен
                icon_url: Some(
                    "https://assets.coingecko.com/coins/images/6319/small/USD_Coin_icon.png"
                        .to_string(),
                ),
                coingecko_id: Some("usd-coin".to_string()),
            },
        );

        // BTT (BitTorrent Token)
        tokens.insert(
            "BTT".to_string(),
            TokenInfo {
                symbol: "BTT".to_string(),
                name: "BitTorrent".to_string(),
                contract_address: "TAFjULxiVgT4qWk6UZwjqwZXTSaGaqnVp4".to_string(),
                decimals: 18,
                is_stable: false,
                min_transfer_amount: Decimal::new(1000, 0), // 1000 BTT
                max_transfer_amount: None,
                enabled: false,
                icon_url: Some(
                    "https://assets.coingecko.com/coins/images/22457/small/btt_logo.png"
                        .to_string(),
                ),
                coingecko_id: Some("bittorrent".to_string()),
            },
        );

        Self {
            tokens,
            primary_token: "USDT".to_string(),
        }
    }

    /// Получает информацию о токене по символу
    pub fn get_token(&self, symbol: &str) -> Option<&TokenInfo> {
        self.tokens.get(symbol)
    }

    /// Получает информацию о токене по контракт адресу
    pub fn get_token_by_contract(&self, contract_address: &str) -> Option<&TokenInfo> {
        self.tokens.values().find(|token| {
            token
                .contract_address
                .eq_ignore_ascii_case(contract_address)
        })
    }

    /// Получает основной токен (USDT)
    pub fn get_primary_token(&self) -> &TokenInfo {
        self.tokens
            .get(&self.primary_token)
            .expect("Primary token must exist")
    }

    /// Возвращает все активные токены
    pub fn get_enabled_tokens(&self) -> Vec<&TokenInfo> {
        self.tokens.values().filter(|token| token.enabled).collect()
    }

    /// Возвращает все токены
    pub fn get_all_tokens(&self) -> Vec<&TokenInfo> {
        self.tokens.values().collect()
    }

    /// Добавляет новый токен
    pub fn add_token(&mut self, token: TokenInfo) {
        self.tokens.insert(token.symbol.clone(), token);
    }

    /// Включает/отключает токен
    pub fn set_token_enabled(&mut self, symbol: &str, enabled: bool) -> Result<(), String> {
        match self.tokens.get_mut(symbol) {
            Some(token) => {
                token.enabled = enabled;
                Ok(())
            }
            None => Err(format!("Токен {} не найден", symbol)),
        }
    }

    /// Обновляет конфигурацию токена из внешнего источника
    pub fn update_from_config(&mut self, config_tokens: Vec<TokenInfo>) {
        for token in config_tokens {
            self.tokens.insert(token.symbol.clone(), token);
        }
    }
}

impl Default for TokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Баланс токена для кошелька
#[derive(Debug, Clone, Serialize)]
pub struct TokenBalance {
    pub token_symbol: String,
    pub balance: Decimal,
    pub balance_wei: u64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Множественные балансы токенов
#[derive(Debug, Clone, Serialize)]
pub struct MultiTokenBalance {
    pub wallet_address: String,
    pub balances: HashMap<String, TokenBalance>,
    pub total_usd_value: Option<Decimal>, // Общая стоимость в USD (если есть курсы)
}

impl MultiTokenBalance {
    /// Создает новый мультибаланс
    pub fn new(wallet_address: String) -> Self {
        Self {
            wallet_address,
            balances: HashMap::new(),
            total_usd_value: None,
        }
    }

    /// Добавляет баланс токена
    pub fn add_balance(&mut self, token_symbol: String, balance: Decimal, balance_wei: u64) {
        let token_balance = TokenBalance {
            token_symbol: token_symbol.clone(),
            balance,
            balance_wei,
            last_updated: chrono::Utc::now(),
        };
        self.balances.insert(token_symbol, token_balance);
    }

    /// Получает баланс токена
    pub fn get_balance(&self, token_symbol: &str) -> Option<&TokenBalance> {
        self.balances.get(token_symbol)
    }

    /// Получает список всех токенов с балансом > 0
    pub fn get_non_zero_balances(&self) -> Vec<&TokenBalance> {
        self.balances
            .values()
            .filter(|balance| balance.balance > Decimal::ZERO)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_registry() {
        let registry = TokenRegistry::new();

        // Проверяем основной токен
        let usdt = registry.get_primary_token();
        assert_eq!(usdt.symbol, "USDT");
        assert_eq!(usdt.decimals, 6);
        assert!(usdt.enabled);

        // Проверяем поиск по контракту
        let token_by_contract =
            registry.get_token_by_contract("TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t");
        assert!(token_by_contract.is_some());
        assert_eq!(token_by_contract.unwrap().symbol, "USDT");
    }

    #[test]
    fn test_token_amount_conversion() {
        let registry = TokenRegistry::new();
        let usdt = registry.get_token("USDT").unwrap();

        // Тест конвертации в wei
        let amount = Decimal::new(12345, 2); // 123.45 USDT
        let wei = usdt.to_wei(amount).unwrap();
        assert_eq!(wei, 123_450_000); // 123.45 * 10^6

        // Тест обратной конвертации
        let back_amount = usdt.from_wei(wei);
        assert_eq!(back_amount, amount);
    }

    #[test]
    fn test_amount_validation() {
        let registry = TokenRegistry::new();
        let usdt = registry.get_token("USDT").unwrap();

        // Проверяем минимальную сумму
        assert!(usdt.validate_amount(Decimal::new(5, 1)).is_err()); // 0.5 USDT
        assert!(usdt.validate_amount(Decimal::new(10, 0)).is_ok()); // 10 USDT
    }
}
