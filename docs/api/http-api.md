# 📡 HTTP API Documentation

## 🎯 Обзор

Multi-Chain Payment Gateway предоставляет RESTful HTTP API для всех операций с кошельками, трансферами и multi-chain функциональностью.

**Base URL**: `http://localhost:8080` (или ваш домен)  
**Version**: 3.0  
**Content-Type**: `application/json`

## 🔐 Аутентификация

_В текущей версии аутентификация не требуется. В production версии будет добавлена JWT/API key аутентификация._

## 📋 Общие принципы

### 📊 Формат ответов

Все ответы возвращаются в JSON формате:

```json
{
  "data": { ... },
  "status": "success",
  "timestamp": "2024-12-20T10:30:00Z"
}
```

### ❌ Обработка ошибок

```json
{
  "error": {
    "code": "WALLET_NOT_FOUND",
    "message": "Wallet with ID 123 not found",
    "details": { ... }
  },
  "status": "error",
  "timestamp": "2024-12-20T10:30:00Z"
}
```

### 💰 Денежные суммы

Все денежные суммы передаются как строки с фиксированной точностью:

- **USDT/USDC**: `"100.000000"` (6 знаков после запятой)
- **TRX**: `"15.000000"` (6 знаков после запятой)
- **ETH**: `"0.050000000000000000"` (18 знаков после запятой)

## 🏠 Health Check

### `GET /health`

Проверка состояния сервиса.

**Response**:

```json
{
  "status": "OK",
  "version": "3.0",
  "timestamp": "2024-12-20T10:30:00Z",
  "components": {
    "database": "healthy",
    "tron_network": "healthy",
    "cache": "healthy"
  }
}
```

## 💳 Управление кошельками

### `POST /api/wallets`

Создание нового кошелька.

**Request**:

```json
{
  "owner_id": "user_12345" // Опционально
}
```

**Response**:

```json
{
  "id": 1,
  "address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "hex_address": "41234567890abcdef...",
  "owner_id": "user_12345",
  "is_activated": true,
  "created_at": "2024-12-20T10:30:00Z"
}
```

### `GET /api/wallets/{wallet_id}`

Получение информации о кошельке.

**Response**:

```json
{
  "id": 1,
  "address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "owner_id": "user_12345",
  "is_activated": true,
  "created_at": "2024-12-20T10:30:00Z",
  "balance": {
    "USDT": "100.000000",
    "TRX": "15.000000"
  }
}
```

### `GET /api/wallets/{wallet_id}/balance`

Получение баланса кошелька.

**Response**:

```json
{
  "wallet_id": 1,
  "balances": {
    "USDT": {
      "balance": "100.000000",
      "balance_usd": "100.00",
      "contract_address": "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t"
    },
    "TRX": {
      "balance": "15.000000",
      "balance_usd": "2.40"
    }
  },
  "total_usd_value": "102.40",
  "last_updated": "2024-12-20T10:30:00Z"
}
```

### `POST /api/wallets/{wallet_id}/activate`

Активация кошелька (отправка TRX для активации).

**Response**:

```json
{
  "wallet_id": 1,
  "address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "activation_status": "success",
  "tx_hash": "abc123...",
  "amount_sent": "1.000000"
}
```

## 💸 Управление трансферами

### `POST /api/transfers/preview`

Превью трансфера с расчетом всех комиссий.

**Request**:

```json
{
  "from_wallet_id": 1,
  "order_amount": "100.000000",
  "reference_id": "order_12345" // Опционально
}
```

**Response**:

```json
{
  "order_amount": "100.000000",
  "commission": "5.000000",
  "commission_percentage": "5.00",
  "gas_cost_in_usdt": "2.500000",
  "total_amount": "107.500000",
  "master_wallet_receives": "107.500000",
  "breakdown": "100 USDT заказ + 5 USDT комиссия + 2.5 USDT газ = 107.5 USDT",
  "trx_to_usdt_rate": "0.16",
  "estimated_time": "1-3 minutes",
  "from_wallet_id": 1,
  "reference_id": "order_12345"
}
```

### `POST /api/transfers`

Создание трансфера.

**Request**:

```json
{
  "from_wallet_id": 1,
  "order_amount": "100.000000",
  "reference_id": "order_12345", // Опционально
  "preview_only": false // По умолчанию false
}
```

**Response**:

```json
{
  "id": 1,
  "from_wallet_id": 1,
  "to_address": "TYourMasterWalletAddress...",
  "amount": "107.500000",
  "status": "pending",
  "reference_id": "order_12345",
  "created_at": "2024-12-20T10:30:00Z",
  "estimated_completion": "2024-12-20T10:33:00Z"
}
```

### `GET /api/transfers/{transfer_id}`

Получение информации о трансфере.

**Response**:

```json
{
  "id": 1,
  "from_wallet_id": 1,
  "to_address": "TYourMasterWalletAddress...",
  "amount": "107.500000",
  "status": "completed",
  "tx_hash": "abc123def456...",
  "reference_id": "order_12345",
  "error_message": null,
  "created_at": "2024-12-20T10:30:00Z",
  "completed_at": "2024-12-20T10:32:30Z"
}
```

### `GET /api/transfers/reference/{reference_id}`

Получение трансфера по reference_id.

**Response**: Аналогично `GET /api/transfers/{transfer_id}`

### `GET /api/wallets/{wallet_id}/transfers`

Получение всех трансферов кошелька.

**Query params**:

- `limit` (default: 50, max: 100)
- `offset` (default: 0)
- `status` (pending, processing, completed, failed)

**Response**:

```json
{
  "wallet_id": 1,
  "transfers": [
    {
      "id": 1,
      "amount": "107.500000",
      "status": "completed",
      "tx_hash": "abc123...",
      "created_at": "2024-12-20T10:30:00Z"
    }
  ],
  "pagination": {
    "total": 25,
    "limit": 50,
    "offset": 0,
    "has_more": false
  }
}
```

## 🪙 Multi-Token API

### `GET /api/tokens`

Получение списка поддерживаемых токенов.

**Response**:

```json
{
  "tokens": [
    {
      "symbol": "USDT",
      "name": "Tether USD",
      "contract_address": "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t",
      "decimals": 6,
      "enabled": true,
      "network": "tron"
    },
    {
      "symbol": "USDC",
      "name": "USD Coin",
      "contract_address": "TEkxiTehnzSmSe2XqrBj4w32RUN966rdz8",
      "decimals": 6,
      "enabled": false,
      "network": "tron"
    }
  ]
}
```

### `GET /api/tokens/balance`

Получение балансов по всем токенам.

**Query params**:

- `wallet_address` (required)
- `tokens` (comma-separated, optional)

**Example**: `/api/tokens/balance?wallet_address=TH3Q...&tokens=USDT,USDC`

**Response**:

```json
{
  "wallet_address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "balances": {
    "USDT": {
      "balance": "100.000000",
      "balance_usd": "100.00",
      "balance_wei": "100000000"
    },
    "USDC": {
      "balance": "50.000000",
      "balance_usd": "50.00",
      "balance_wei": "50000000"
    }
  },
  "total_usd_value": "150.00",
  "last_updated": "2024-12-20T10:30:00Z"
}
```

### `POST /api/tokens/transfer`

Трансфер конкретного токена.

**Request**:

```json
{
  "from_wallet_id": 1,
  "token_symbol": "USDT",
  "amount": "100.000000",
  "reference_id": "token_transfer_123"
}
```

**Response**: Аналогично обычному трансферу

### `POST /api/tokens/{token_symbol}/toggle`

Включение/отключение токена.

**Request**:

```json
{
  "enabled": true
}
```

**Response**:

```json
{
  "token_symbol": "USDC",
  "enabled": true,
  "message": "Token USDC has been enabled"
}
```

### `GET /api/tokens/cache/stats`

Статистика кэша токенов.

**Response**:

```json
{
  "hits": 1250,
  "misses": 50,
  "total_requests": 1300,
  "hit_rate": 96.15,
  "cache_size": 100,
  "last_cleanup": "2024-12-20T10:25:00Z"
}
```

### `DELETE /api/tokens/cache/invalidate/{wallet_address}`

Инвалидация кэша для кошелька.

**Response**:

```json
{
  "wallet_address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "invalidated": true,
  "message": "Cache invalidated for wallet"
}
```

## 🌐 Smart Router API

### `POST /smart/routes`

Получение оптимальных маршрутов для трансфера.

**Request**:

```json
{
  "from_address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "to_address": "TYourMasterWallet...",
  "amount": "100.000000",
  "token_symbol": "USDT",
  "preference": "balanced" // cheapest, fastest, most_reliable, balanced
}
```

**Response**:

```json
{
  "routes": [
    {
      "id": "tron-route-1",
      "network": "tron",
      "estimated_cost_usd": "2.50",
      "estimated_time_sec": "90",
      "reliability_score": "0.98",
      "total_score": "0.88",
      "recommended": true,
      "details": {
        "gas_fee_native": "15.000000",
        "gas_fee_usd": "2.50",
        "network_congestion": "low",
        "success_rate": "98%"
      }
    },
    {
      "id": "ethereum-route-1",
      "network": "ethereum",
      "estimated_cost_usd": "25.00",
      "estimated_time_sec": "300",
      "reliability_score": "0.99",
      "total_score": "0.65",
      "recommended": false,
      "details": {
        "gas_fee_native": "0.015000000000000000",
        "gas_fee_usd": "25.00",
        "network_congestion": "medium",
        "success_rate": "99%"
      }
    }
  ],
  "analysis_time_ms": "150",
  "recommended_route_id": "tron-route-1"
}
```

### `POST /smart/transfer`

Умный трансфер с автовыбором сети.

**Request**:

```json
{
  "from_address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "to_address": "TYourMasterWallet...",
  "amount": "100.000000",
  "token_symbol": "USDT",
  "preference": "cheapest",
  "route_id": "tron-route-1" // Опционально, принудительный выбор
}
```

**Response**:

```json
{
  "transaction_hash": "abc123def456...",
  "network_used": "tron",
  "actual_cost_usd": "2.45",
  "execution_time_ms": "2500",
  "selected_route": {
    "id": "tron-route-1",
    "network": "tron",
    "estimated_cost_usd": "2.50"
  },
  "status": "success"
}
```

### `POST /smart/balance`

Multi-chain баланс кошелька.

**Request**:

```json
{
  "wallet_address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "networks": ["tron", "ethereum"], // Опционально
  "tokens": ["USDT", "USDC"] // Опционально
}
```

**Response**:

```json
{
  "wallet_address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "balances": {
    "tron": {
      "native_balance": "15.000000",
      "tokens": {
        "USDT": {
          "balance": "100.000000",
          "balance_usd": "100.00"
        }
      }
    },
    "ethereum": {
      "native_balance": "0.050000000000000000",
      "tokens": {
        "USDT": {
          "balance": "50.000000",
          "balance_usd": "50.00"
        }
      }
    }
  },
  "total_usd_value": "185.50"
}
```

### `GET /smart/networks`

Статус всех поддерживаемых сетей.

**Response**:

```json
{
  "networks": {
    "tron": {
      "enabled": true,
      "health": "healthy",
      "rpc_status": "online",
      "block_height": "65432100",
      "response_time_ms": "150"
    },
    "ethereum": {
      "enabled": true,
      "health": "healthy",
      "rpc_status": "online",
      "block_height": "18750000",
      "response_time_ms": "300"
    }
  },
  "last_updated": "2024-12-20T10:30:00Z"
}
```

### `POST /smart/quote`

Быстрая оценка стоимости без кэша.

**Request**:

```json
{
  "from_address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "to_address": "TYourMasterWallet...",
  "amount": "100.000000",
  "token_symbol": "USDT",
  "networks": ["tron", "ethereum"]
}
```

**Response**:

```json
{
  "quotes": {
    "tron": {
      "network": "tron",
      "estimated_cost_usd": "2.50",
      "estimated_time_sec": "90",
      "gas_fee": "15.000000",
      "available": true
    },
    "ethereum": {
      "network": "ethereum",
      "estimated_cost_usd": "25.00",
      "estimated_time_sec": "300",
      "gas_fee": "0.015000000000000000",
      "available": true
    }
  },
  "fastest_network": "tron",
  "cheapest_network": "tron"
}
```

## 🔧 Network Management API

### `GET /networks`

Получение информации о всех сетях.

**Response**:

```json
{
  "networks": [
    {
      "name": "tron",
      "chain_id": "728126428",
      "native_currency": "TRX",
      "enabled": true,
      "rpc_url": "https://api.shasta.trongrid.io",
      "confirmation_blocks": 3
    },
    {
      "name": "ethereum",
      "chain_id": "5",
      "native_currency": "ETH",
      "enabled": false,
      "rpc_url": "https://eth-goerli.g.alchemy.com/v2/YOUR_API_KEY",
      "confirmation_blocks": 12
    }
  ]
}
```

### `POST /networks/{network}/enable`

Включение сети.

**Response**:

```json
{
  "network": "ethereum",
  "enabled": true,
  "message": "Network ethereum has been enabled"
}
```

### `POST /networks/{network}/disable`

Отключение сети.

**Response**:

```json
{
  "network": "ethereum",
  "enabled": false,
  "message": "Network ethereum has been disabled"
}
```

## 📊 Статусы и коды ошибок

### 📈 Статусы трансферов

| Status       | Description         |
| ------------ | ------------------- |
| `pending`    | Ожидает обработки   |
| `processing` | В процессе отправки |
| `completed`  | Успешно завершен    |
| `failed`     | Ошибка выполнения   |

### ❌ HTTP коды ошибок

| Code  | Description               |
| ----- | ------------------------- |
| `200` | Успешно                   |
| `400` | Неверный запрос           |
| `404` | Ресурс не найден          |
| `422` | Ошибка валидации          |
| `500` | Внутренняя ошибка сервера |
| `503` | Сервис недоступен         |

### 🚨 Коды ошибок приложения

| Code                   | Description          |
| ---------------------- | -------------------- |
| `WALLET_NOT_FOUND`     | Кошелек не найден    |
| `INSUFFICIENT_BALANCE` | Недостаточно средств |
| `INVALID_ADDRESS`      | Некорректный адрес   |
| `NETWORK_ERROR`        | Ошибка сети блокчейн |
| `INVALID_AMOUNT`       | Некорректная сумма   |
| `TRANSFER_FAILED`      | Ошибка трансфера     |

## 🔄 Rate Limits

| Endpoint              | Limit       |
| --------------------- | ----------- |
| `/api/wallets`        | 100 req/min |
| `/api/transfers`      | 50 req/min  |
| `/api/tokens/balance` | 200 req/min |
| `/smart/*`            | 100 req/min |

## 📝 Примеры интеграции

### 🛒 E-commerce checkout

```javascript
// 1. Создаем кошелек для пользователя
const wallet = await fetch("/api/wallets", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ owner_id: `user_${userId}` }),
}).then((r) => r.json());

// 2. Показываем адрес для пополнения
showDepositAddress(wallet.address);

// 3. Проверяем баланс
const balance = await fetch(`/api/wallets/${wallet.id}/balance`).then((r) =>
  r.json()
);

// 4. Создаем трансфер после пополнения
const transfer = await fetch("/api/transfers", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    from_wallet_id: wallet.id,
    order_amount: orderAmount,
    reference_id: orderId,
  }),
}).then((r) => r.json());

// 5. Отслеживаем статус
const checkStatus = setInterval(async () => {
  const status = await fetch(`/api/transfers/${transfer.id}`).then((r) =>
    r.json()
  );

  if (status.status === "completed") {
    clearInterval(checkStatus);
    completeOrder(orderId);
  }
}, 5000);
```

---

**Следующие шаги**:

- 📖 [gRPC API документация](grpc-api.md)
- 🔗 [Multi-Chain API примеры](multi-chain-api.md)
- 📝 [Код примеры](examples.md)


