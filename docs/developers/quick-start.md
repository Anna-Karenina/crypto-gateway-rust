# 🚀 Быстрый старт для разработчиков

## 📋 Предварительные требования

### 🖥️ Системные требования

- **Rust**: 1.70+ (для сборки из исходников)
- **Docker**: 20.10+ (рекомендуемый способ)
- **PostgreSQL**: 13+ (база данных)
- **8GB RAM** и **4 CPU cores** (минимум)

### 🔑 Необходимые данные

- **TRON API Key** от [TronGrid](https://www.trongrid.io/)
- **Ethereum RPC URL** (Alchemy, Infura или собственная нода)
- **Master Wallet** с небольшим количеством TRX для активации кошельков

## ⚡ Быстрый запуск (5 минут)

### 1️⃣ Клонирование и настройка

```bash
# Клонируем репозиторий
git clone https://github.com/your-org/tron-gateway-rust.git
cd tron-gateway-rust

# Копируем пример конфигурации
cp env_example.txt .env

# Редактируем основные настройки
nano .env
```

### 2️⃣ Минимальная конфигурация

Отредактируйте `.env` файл:

```bash
# Основные настройки
SERVER__HOST=0.0.0.0
SERVER__PORT=8080
DATABASE__URL=postgresql://postgres:password@localhost:5432/tron_gateway

# TRON настройки
TRON__API_KEY=your_trongrid_api_key_here
TRON__BASE_URL=https://api.shasta.trongrid.io
TRON__MASTER_WALLET_ADDRESS=your_master_wallet_address
TRON__MASTER_WALLET_PRIVATE_KEY=your_master_wallet_private_key

# Сети (только TRON для начала)
NETWORKS__TRON__ENABLED=true
NETWORKS__ETHEREUM__ENABLED=false
```

### 3️⃣ Запуск через Docker

```bash
# Запускаем PostgreSQL
docker run -d \
  --name tron-gateway-db \
  -e POSTGRES_DB=tron_gateway \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  postgres:13

# Собираем и запускаем приложение
docker build -t tron-gateway .
docker run -d \
  --name tron-gateway-app \
  --env-file .env \
  -p 8080:8080 \
  -p 50051:50051 \
  tron-gateway
```

### 4️⃣ Проверка работы

```bash
# Проверяем health check
curl http://localhost:8080/health

# Получаем список поддерживаемых сетей
curl http://localhost:8080/api/networks

# Создаем тестовый кошелек
curl -X POST http://localhost:8080/api/wallets \
  -H "Content-Type: application/json" \
  -d '{"owner_id": "test_user_123"}'
```

## 🧪 Первые API вызовы

### 💳 Создание кошелька

```bash
curl -X POST http://localhost:8080/api/wallets \
  -H "Content-Type: application/json" \
  -d '{
    "owner_id": "user_12345"
  }'
```

**Ответ**:

```json
{
  "id": 1,
  "address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
  "owner_id": "user_12345",
  "is_activated": true,
  "created_at": "2024-12-20T10:30:00Z"
}
```

### 💰 Проверка баланса

```bash
curl http://localhost:8080/api/wallets/1/balance
```

**Ответ**:

```json
{
  "wallet_id": 1,
  "balances": {
    "USDT": {
      "balance": "0.000000",
      "balance_usd": "0.00"
    },
    "TRX": {
      "balance": "1.000000",
      "balance_usd": "0.16"
    }
  },
  "total_usd_value": "0.16"
}
```

### 📊 Превью трансфера

```bash
curl -X POST http://localhost:8080/api/transfers/preview \
  -H "Content-Type: application/json" \
  -d '{
    "from_wallet_id": 1,
    "order_amount": "100.000000",
    "reference_id": "order_12345"
  }'
```

**Ответ**:

```json
{
  "order_amount": "100.000000",
  "commission": "5.000000",
  "gas_cost_in_usdt": "2.500000",
  "total_amount": "107.500000",
  "breakdown": "100 USDT + 5 USDT комиссия + 2.5 USDT газ = 107.5 USDT",
  "estimated_time": "1-3 minutes"
}
```

### 💸 Создание трансфера

```bash
curl -X POST http://localhost:8080/api/transfers \
  -H "Content-Type: application/json" \
  -d '{
    "from_wallet_id": 1,
    "order_amount": "100.000000",
    "reference_id": "order_12345"
  }'
```

**Ответ**:

```json
{
  "id": 1,
  "from_wallet_id": 1,
  "amount": "107.500000",
  "status": "pending",
  "reference_id": "order_12345",
  "created_at": "2024-12-20T10:35:00Z"
}
```

## 🌐 Multi-Chain возможности

### 🔗 Smart Router

```bash
# Получаем оптимальные маршруты для трансфера
curl -X POST http://localhost:8080/smart/routes \
  -H "Content-Type: application/json" \
  -d '{
    "from_address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
    "to_address": "master_wallet_address",
    "amount": "100.000000",
    "token_symbol": "USDT",
    "preference": "balanced"
  }'
```

**Ответ**:

```json
{
  "routes": [
    {
      "network": "tron",
      "estimated_cost_usd": "2.50",
      "estimated_time_sec": "90",
      "reliability_score": "0.98",
      "recommended": true
    },
    {
      "network": "ethereum",
      "estimated_cost_usd": "25.00",
      "estimated_time_sec": "300",
      "reliability_score": "0.99",
      "recommended": false
    }
  ]
}
```

### 🎯 Умный трансфер

```bash
# Выполняем трансфер с автовыбором сети
curl -X POST http://localhost:8080/smart/transfer \
  -H "Content-Type: application/json" \
  -d '{
    "from_address": "TH3QBLNLsimQbNwq2DxTGhoDYeeCZYTvK3",
    "to_address": "master_wallet_address",
    "amount": "100.000000",
    "token_symbol": "USDT",
    "preference": "cheapest"
  }'
```

## ⚡ gRPC API

### 📝 gRPC клиент (Node.js)

```javascript
const grpc = require("@grpc/grpc-js");
const protoLoader = require("@grpc/proto-loader");

// Загружаем proto файлы
const packageDefinition = protoLoader.loadSync("proto/wallet.proto");
const walletProto = grpc.loadPackageDefinition(packageDefinition);

// Создаем клиент
const client = new walletProto.tron_gateway.wallet.v1.WalletService(
  "localhost:50051",
  grpc.credentials.createInsecure()
);

// Создаем кошелек
client.CreateWallet(
  {
    owner_id: "user_12345",
  },
  (error, response) => {
    if (error) {
      console.error("Error:", error);
    } else {
      console.log("Wallet created:", response);
    }
  }
);
```

### 🐍 gRPC клиент (Python)

```python
import grpc
import wallet_pb2
import wallet_pb2_grpc

# Создаем соединение
channel = grpc.insecure_channel('localhost:50051')
stub = wallet_pb2_grpc.WalletServiceStub(channel)

# Создаем кошелек
request = wallet_pb2.CreateWalletRequest(owner_id='user_12345')
response = stub.CreateWallet(request)
print(f"Wallet created: {response}")
```

## 🔧 Локальная разработка

### 🛠️ Сборка из исходников

```bash
# Устанавливаем Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Устанавливаем зависимости
cargo install diesel_cli --no-default-features --features postgres

# Настраиваем базу данных
diesel setup
diesel migration run

# Запускаем в режиме разработки
source .env && cargo run
```

### 🧪 Запуск тестов

```bash
# Unit тесты
cargo test

# Integration тесты
cargo test --test integration_*

# Все тесты с выводом
cargo test -- --nocapture
```

### 🔍 Логирование и отладка

```bash
# Включаем детальное логирование
export LOGGING__LEVEL=debug
export RUST_LOG=debug

# Запускаем с логами
cargo run
```

## 📈 Мониторинг и метрики

### 🏥 Health checks

```bash
# Основной health check
curl http://localhost:8080/health

# Проверка статуса сетей
curl http://localhost:8080/networks

# Статистика кэша
curl http://localhost:8080/api/tokens/cache/stats
```

### 📊 Prometheus метрики

```bash
# Метрики приложения (если включены)
curl http://localhost:8080/metrics
```

## 🚨 Troubleshooting

### ❌ Частые проблемы

#### 1. "Database connection failed"

```bash
# Проверьте, что PostgreSQL запущен
docker ps | grep postgres

# Проверьте строку подключения
echo $DATABASE__URL
```

#### 2. "TRON API key invalid"

```bash
# Проверьте API ключ
curl -H "TRON-PRO-API-KEY: $TRON__API_KEY" \
  https://api.shasta.trongrid.io/wallet/validateaddress
```

#### 3. "Insufficient TRX for activation"

```bash
# Проверьте баланс master wallet
curl "https://api.shasta.trongrid.io/v1/accounts/$TRON__MASTER_WALLET_ADDRESS"
```

### 🔍 Логи и диагностика

```bash
# Просмотр логов Docker контейнера
docker logs tron-gateway-app -f

# Проверка состояния процесса
ps aux | grep tron-gateway

# Проверка портов
netstat -tlnp | grep -E '8080|50051'
```

## 📚 Следующие шаги

### 🎯 Для интеграции

1. 📖 Изучите [полную API документацию](../api/http-api.md)
2. 🔧 Настройте [конфигурацию под ваши нужды](configuration.md)
3. 📝 Посмотрите [примеры интеграции](../examples/)

### 🏗️ Для разработки

1. 🏛️ Ознакомьтесь с [архитектурой системы](architecture.md)
2. 🧪 Настройте [тестирование](testing.md)
3. 🚀 Изучите [руководство по развертыванию](../deployment/)

### 💡 Примеры использования

1. 🛒 [E-commerce интеграция](../examples/ecommerce.md)
2. 🎮 [Gaming платформа](../examples/gaming.md)
3. 💱 [Криптообменник](../examples/exchange.md)

---

**Нужна помощь?**

- 📧 Email: dev-support@your-company.com
- 💬 Telegram: @tron_gateway_support
- 📖 Документация: [полная версия](../README.md)


