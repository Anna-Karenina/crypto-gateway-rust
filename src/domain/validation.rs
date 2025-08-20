//! # Валидация данных
//!
//! Модуль для валидации адресов, сумм и других входных данных

use base58::{FromBase58, ToBase58};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

use crate::domain::errors::{DomainError, DomainResult};

/// Сервис валидации TRON адресов и данных
pub struct TronValidator;

impl TronValidator {
    /// Валидирует TRON адрес (base58 формат)
    pub fn validate_address(address: &str) -> DomainResult<()> {
        // Проверяем базовый формат
        if address.is_empty() {
            return Err(DomainError::InvalidTronAddress {
                address: address.to_string(),
            });
        }

        // TRON адреса должны начинаться с 'T'
        if !address.starts_with('T') {
            return Err(DomainError::InvalidTronAddress {
                address: address.to_string(),
            });
        }

        // Проверяем длину (обычно 34 символа)
        if address.len() < 30 || address.len() > 40 {
            return Err(DomainError::InvalidTronAddress {
                address: address.to_string(),
            });
        }

        // Декодируем base58
        let decoded = address
            .from_base58()
            .map_err(|_| DomainError::InvalidTronAddress {
                address: address.to_string(),
            })?;

        // TRON адрес должен быть 25 байт (21 байт адреса + 4 байта checksum)
        if decoded.len() != 25 {
            return Err(DomainError::InvalidTronAddress {
                address: address.to_string(),
            });
        }

        // Проверяем checksum
        let (address_bytes, checksum) = decoded.split_at(21);
        let calculated_checksum = Self::calculate_checksum(address_bytes);

        if checksum != calculated_checksum {
            return Err(DomainError::InvalidTronAddress {
                address: address.to_string(),
            });
        }

        // Проверяем, что первый байт правильный (0x41 для mainnet)
        if address_bytes[0] != 0x41 {
            return Err(DomainError::InvalidTronAddress {
                address: address.to_string(),
            });
        }

        Ok(())
    }

    /// Валидирует сумму трансфера
    pub fn validate_amount(amount: Decimal) -> DomainResult<()> {
        if amount <= Decimal::ZERO {
            return Err(DomainError::InvalidAmount { amount });
        }

        // Максимальная сумма (для защиты от переполнения)
        let max_amount = Decimal::new(1_000_000_000, 6); // 1 миллиард USDT
        if amount > max_amount {
            return Err(DomainError::InvalidAmount { amount });
        }

        // Проверяем количество знаков после запятой (USDT имеет 6 decimals)
        let scale = amount.scale();
        if scale > 6 {
            return Err(DomainError::InvalidAmount { amount });
        }

        Ok(())
    }

    /// Валидирует reference_id
    pub fn validate_reference_id(reference_id: &str) -> DomainResult<()> {
        if reference_id.is_empty() {
            return Err(DomainError::ConfigurationError {
                message: "Reference ID не может быть пустым".to_string(),
            });
        }

        if reference_id.len() > 255 {
            return Err(DomainError::ConfigurationError {
                message: "Reference ID слишком длинный (максимум 255 символов)".to_string(),
            });
        }

        // Разрешаем только безопасные символы
        let is_valid = reference_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.');

        if !is_valid {
            return Err(DomainError::ConfigurationError {
                message: "Reference ID содержит недопустимые символы".to_string(),
            });
        }

        Ok(())
    }

    /// Валидирует приватный ключ TRON (hex формат)
    pub fn validate_private_key(private_key: &str) -> DomainResult<()> {
        if private_key.is_empty() {
            return Err(DomainError::CryptoError {
                message: "Приватный ключ не может быть пустым".to_string(),
            });
        }

        // TRON приватный ключ должен быть 64 символа (32 байта в hex)
        if private_key.len() != 64 {
            return Err(DomainError::CryptoError {
                message: "Приватный ключ должен содержать 64 символа".to_string(),
            });
        }

        // Проверяем, что это валидный hex
        if !private_key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(DomainError::CryptoError {
                message: "Приватный ключ должен содержать только hex символы".to_string(),
            });
        }

        Ok(())
    }

    /// Валидирует owner_id
    pub fn validate_owner_id(owner_id: &str) -> DomainResult<()> {
        if owner_id.is_empty() {
            return Err(DomainError::ConfigurationError {
                message: "Owner ID не может быть пустым".to_string(),
            });
        }

        if owner_id.len() > 100 {
            return Err(DomainError::ConfigurationError {
                message: "Owner ID слишком длинный (максимум 100 символов)".to_string(),
            });
        }

        Ok(())
    }

    /// Вычисляет checksum для TRON адреса
    fn calculate_checksum(address_bytes: &[u8]) -> Vec<u8> {
        let hash1 = Sha256::digest(address_bytes);
        let hash2 = Sha256::digest(&hash1);
        hash2[..4].to_vec()
    }

    /// Конвертирует hex адрес в base58 (для тестирования)
    pub fn hex_to_base58(hex_address: &str) -> DomainResult<String> {
        // Убираем 0x если есть
        let hex_clean = hex_address.strip_prefix("0x").unwrap_or(hex_address);

        let address_bytes =
            hex::decode(hex_clean).map_err(|_| DomainError::InvalidTronAddress {
                address: hex_address.to_string(),
            })?;

        if address_bytes.len() != 21 {
            return Err(DomainError::InvalidTronAddress {
                address: hex_address.to_string(),
            });
        }

        let checksum = Self::calculate_checksum(&address_bytes);
        let mut full_address = address_bytes;
        full_address.extend_from_slice(&checksum);

        Ok(full_address.to_base58())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_address() {
        // Тестовый TRON адрес
        let address = "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3";
        assert!(TronValidator::validate_address(address).is_ok());
    }

    #[test]
    fn test_validate_invalid_address_format() {
        let invalid_addresses = vec![
            "",                                       // Пустой
            "invalid",                                // Неправильный формат
            "1H3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",     // Не начинается с T
            "T",                                      // Слишком короткий
            "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3xxxx", // Слишком длинный
        ];

        for address in invalid_addresses {
            assert!(TronValidator::validate_address(address).is_err());
        }
    }

    #[test]
    fn test_validate_amount() {
        // Валидные суммы
        assert!(TronValidator::validate_amount(Decimal::new(1, 0)).is_ok());
        assert!(TronValidator::validate_amount(Decimal::new(1000000, 6)).is_ok()); // 1 USDT

        // Невалидные суммы
        assert!(TronValidator::validate_amount(Decimal::ZERO).is_err()); // Ноль
        assert!(TronValidator::validate_amount(Decimal::new(-1, 0)).is_err()); // Отрицательная
        assert!(TronValidator::validate_amount(Decimal::new(1000000000000i64, 0)).is_err());
        // Слишком большая
    }

    #[test]
    fn test_validate_reference_id() {
        // Валидные reference_id
        assert!(TronValidator::validate_reference_id("order_123").is_ok());
        assert!(TronValidator::validate_reference_id("payment-456").is_ok());
        assert!(TronValidator::validate_reference_id("user.transaction.789").is_ok());

        // Невалидные reference_id
        assert!(TronValidator::validate_reference_id("").is_err()); // Пустой
        assert!(TronValidator::validate_reference_id("order#123").is_err()); // Недопустимый символ
        assert!(TronValidator::validate_reference_id(&"x".repeat(256)).is_err());
        // Слишком длинный
    }

    #[test]
    fn test_validate_private_key() {
        // Валидный приватный ключ (пример)
        let valid_key = "df319c4fe709ad6a9f32b07ada986f4055708f4e064e5ff6cab439561a6eae59";
        assert!(TronValidator::validate_private_key(valid_key).is_ok());

        // Невалидные ключи
        assert!(TronValidator::validate_private_key("").is_err()); // Пустой
        assert!(TronValidator::validate_private_key("short").is_err()); // Короткий
        assert!(TronValidator::validate_private_key(
            "invalid_hex_key_with_wrong_characters_xyz123"
        )
        .is_err()); // Неправильные символы
    }
}
