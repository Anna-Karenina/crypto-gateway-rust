# Многоэтапная сборка для оптимизации размера образа
FROM rust:1.75 as builder

WORKDIR /app

# Копируем файлы зависимостей для кеширования
COPY Cargo.toml Cargo.lock ./

# Создаем dummy main.rs для кеширования зависимостей
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Копируем исходный код
COPY src ./src

# Пересобираем с реальным кодом
RUN touch src/main.rs && cargo build --release

# Финальный образ
FROM debian:bookworm-slim

# Устанавливаем необходимые системные зависимости
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Создаем пользователя для безопасности
RUN useradd --create-home --shell /bin/bash --uid 1000 appuser

# Копируем скомпилированный бинарник
COPY --from=builder /app/target/release/tron-gateway-rust /usr/local/bin/tron-gateway-rust

# Создаем директорию для конфигурации
RUN mkdir -p /app/config && chown appuser:appuser /app/config

# Переключаемся на непривилегированного пользователя
USER appuser
WORKDIR /app

# Копируем конфигурацию (опционально)
COPY --chown=appuser:appuser config.toml ./config.toml

# Открываем порт
EXPOSE 8080

# Команда запуска
CMD ["tron-gateway-rust"]
