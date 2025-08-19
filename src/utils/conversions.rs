//! # Конвертеры типов
//! 
//! Утилиты для конвертации между различными типами данных

use bigdecimal::BigDecimal;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Конвертирует rust_decimal::Decimal в bigdecimal::BigDecimal для Diesel
pub fn decimal_to_bigdecimal(decimal: Decimal) -> BigDecimal {
    BigDecimal::from_str(&decimal.to_string()).unwrap_or_default()
}

/// Конвертирует bigdecimal::BigDecimal в rust_decimal::Decimal
pub fn bigdecimal_to_decimal(bigdecimal: BigDecimal) -> Decimal {
    Decimal::from_str(&bigdecimal.to_string()).unwrap_or_default()
}
