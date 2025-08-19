-- Создание таблицы входящих транзакций
CREATE TABLE incoming_transactions (
    id BIGSERIAL PRIMARY KEY,
    wallet_id BIGINT NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    tx_hash VARCHAR(128) NOT NULL UNIQUE,
    block_number BIGINT,
    from_address VARCHAR(64) NOT NULL,
    to_address VARCHAR(64) NOT NULL,
    amount DECIMAL(30,18) NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'PENDING',
    error_message TEXT,
    detected_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMP WITH TIME ZONE
);

-- Индексы для оптимизации поиска
CREATE INDEX idx_incoming_transactions_wallet_id ON incoming_transactions(wallet_id);
CREATE INDEX idx_incoming_transactions_tx_hash ON incoming_transactions(tx_hash);
CREATE INDEX idx_incoming_transactions_status ON incoming_transactions(status);
CREATE INDEX idx_incoming_transactions_detected_at ON incoming_transactions(detected_at);