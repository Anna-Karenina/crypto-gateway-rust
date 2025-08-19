-- Создание таблицы кошельков
CREATE TABLE wallets (
    id BIGSERIAL PRIMARY KEY,
    address VARCHAR(64) NOT NULL UNIQUE,
    hex_address VARCHAR(64) NOT NULL UNIQUE,
    private_key VARCHAR(128) NOT NULL,
    owner_id VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Индексы для оптимизации поиска
CREATE INDEX idx_wallets_owner_id ON wallets(owner_id);
CREATE INDEX idx_wallets_address ON wallets(address);
CREATE INDEX idx_wallets_created_at ON wallets(created_at);