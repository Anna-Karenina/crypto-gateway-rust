//! # TRON криптографические операции
//!
//! Генерация кошельков, подписание транзакций и другие криптографические операции для TRON

use anyhow::Result;
use base58::{FromBase58, ToBase58};
use rand::rngs::OsRng;
use rust_decimal::prelude::*;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sha3::Keccak256;

use crate::domain::DomainError;

/// Генератор TRON кошельков
pub struct TronWalletGenerator {
    secp: Secp256k1<secp256k1::All>,
}

impl TronWalletGenerator {
    /// Создает новый экземпляр генератора
    pub fn new() -> Self {
        Self {
            secp: Secp256k1::new(),
        }
    }

    /// Генерация нового TRON кошелька
    /// Возвращает (address, hex_address, private_key)
    pub fn generate_wallet(&self) -> Result<(String, String, String)> {
        // 1. Генерируем случайный приватный ключ
        let private_key = SecretKey::new(&mut OsRng);
        let private_key_hex = hex::encode(private_key.secret_bytes());

        // 2. Получаем публичный ключ (uncompressed, 65 байт)
        let public_key = PublicKey::from_secret_key(&self.secp, &private_key);
        let public_key_bytes = public_key.serialize_uncompressed();

        // 3. Вычисляем адрес
        // Берем keccak256 от публичного ключа (без первого байта 0x04)
        let mut hasher = Keccak256::new();
        hasher.update(&public_key_bytes[1..]);
        let hash = hasher.finalize();

        // Берем последние 20 байт и добавляем префикс TRON (0x41)
        let mut address_bytes = vec![0x41]; // TRON mainnet prefix
        address_bytes.extend_from_slice(&hash[12..]);

        // 4. Создаем hex адрес
        let hex_address = format!("0x{}", hex::encode(&address_bytes[1..]));

        // 5. Создаем base58 адрес с checksum
        let base58_address = self.encode_base58_with_checksum(&address_bytes)?;

        Ok((base58_address, hex_address, private_key_hex))
    }

    /// Валидация TRON адреса
    pub fn validate_address(&self, address: &str) -> Result<bool, DomainError> {
        if address.len() != 34 {
            return Ok(false);
        }

        if !address.starts_with('T') {
            return Ok(false);
        }

        // Пытаемся декодировать base58
        match address.from_base58() {
            Ok(decoded) => {
                if decoded.len() != 25 {
                    return Ok(false);
                }

                // Проверяем checksum
                let (payload, checksum) = decoded.split_at(21);
                let hash1 = Sha256::digest(payload);
                let hash2 = Sha256::digest(&hash1);

                Ok(&hash2[..4] == checksum)
            }
            Err(_) => Ok(false),
        }
    }

    /// Кодирование в base58 с checksum
    fn encode_base58_with_checksum(&self, data: &[u8]) -> Result<String> {
        // Вычисляем double SHA256 checksum
        let hash1 = Sha256::digest(data);
        let hash2 = Sha256::digest(&hash1);
        let checksum = &hash2[..4];

        // Объединяем данные с checksum
        let mut data_with_checksum = data.to_vec();
        data_with_checksum.extend_from_slice(checksum);

        // Кодируем в base58
        Ok(data_with_checksum.to_base58())
    }
}

/// Подписчик TRON транзакций
#[derive(Clone)]
pub struct TronTransactionSigner {
    secp: Secp256k1<secp256k1::All>,
}

impl TronTransactionSigner {
    /// Создает новый экземпляр подписчика
    pub fn new() -> Self {
        Self {
            secp: Secp256k1::new(),
        }
    }

    /// Подписание транзакции
    pub fn sign_transaction(&self, transaction: &Value, private_key_hex: &str) -> Result<Value> {
        tracing::debug!("Подписание транзакции: {:?}", transaction);

        // 1. Получаем raw_data из транзакции
        let raw_data = transaction
            .get("raw_data")
            .ok_or_else(|| anyhow::anyhow!("Отсутствует raw_data в транзакции"))?;

        // 2. Сериализуем raw_data в JSON без пробелов
        let raw_data_json = serde_json::to_string(raw_data)?;
        let raw_data_bytes = raw_data_json.as_bytes();

        tracing::debug!("Raw data для подписи: {}", raw_data_json);

        // 3. Вычисляем SHA256 хеш (TRON использует SHA256, НЕ SHA3!)
        let mut hasher = Sha256::new();
        hasher.update(raw_data_bytes);
        let hash = hasher.finalize();

        tracing::debug!("SHA256 хеш для подписи: {}", hex::encode(&hash));

        // 4. Подписываем хеш
        let private_key_bytes = hex::decode(private_key_hex)?;
        let private_key = SecretKey::from_slice(&private_key_bytes)?;

        let message = secp256k1::Message::from_digest_slice(&hash)?;
        let signature = self.secp.sign_ecdsa(&message, &private_key);

        // 5. Получаем подпись в формате DER и конвертируем в hex
        let signature_bytes = signature.serialize_compact();
        let signature_hex = hex::encode(signature_bytes);

        tracing::debug!("Подпись: {}", signature_hex);

        // 6. Создаем подписанную транзакцию
        let mut signed_transaction = transaction.clone();
        signed_transaction["signature"] = Value::Array(vec![Value::String(signature_hex)]);

        tracing::debug!("Подписанная транзакция: {:?}", signed_transaction);

        Ok(signed_transaction)
    }

    /// Создание USDT transfer транзакции (альтернативный метод)
    pub fn create_usdt_transfer_transaction(
        &self,
        from_address: &str,
        to_address: &str,
        amount: rust_decimal::Decimal,
        contract_address: &str,
    ) -> Result<Value> {
        // Конвертируем адреса в hex формат
        let from_hex = self.address_to_hex(from_address)?;
        let to_hex = self.address_to_hex(to_address)?;
        let contract_hex = self.address_to_hex(contract_address)?;

        // Конвертируем сумму в минимальные единицы (USDT имеет 6 знаков после запятой)
        let amount_units = amount * rust_decimal::Decimal::new(10_i64.pow(6), 0);
        let amount_u64 = amount_units
            .to_u64()
            .ok_or_else(|| anyhow::anyhow!("Недопустимая сумма"))?;

        // Создаем raw transaction
        let raw_transaction = serde_json::json!({
            "contract": [{
                "parameter": {
                    "value": {
                        "data": format!("a9059cbb{:0>64}{:0>64}",
                            &to_hex[2..], // убираем 0x префикс
                            format!("{:x}", amount_u64)
                        ),
                        "owner_address": from_hex,
                        "contract_address": contract_hex
                    },
                    "type_url": "type.googleapis.com/protocol.TriggerSmartContract"
                },
                "type": "TriggerSmartContract"
            }],
            "ref_block_bytes": "0000",
            "ref_block_hash": "0000000000000000",
            "expiration": (chrono::Utc::now().timestamp_millis() + 60000) as u64,
            "timestamp": chrono::Utc::now().timestamp_millis() as u64
        });

        Ok(serde_json::json!({
            "raw_data": raw_transaction
        }))
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
