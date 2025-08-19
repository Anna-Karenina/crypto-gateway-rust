-- Создание таблицы исходящих трансферов
CREATE TABLE outgoing_transfers (
    id BIGSERIAL PRIMARY KEY,
    from_wallet_id BIGINT NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    to_address VARCHAR(64) NOT NULL,
    amount DECIMAL(30,18) NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'PENDING',
    tx_hash VARCHAR(128) UNIQUE,
    reference_id VARCHAR(128),
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE
);

-- Индексы для оптимизации поиска
CREATE INDEX idx_outgoing_transfers_from_wallet_id ON outgoing_transfers(from_wallet_id);
CREATE INDEX idx_outgoing_transfers_tx_hash ON outgoing_transfers(tx_hash);
CREATE INDEX idx_outgoing_transfers_status ON outgoing_transfers(status);
CREATE INDEX idx_outgoing_transfers_reference_id ON outgoing_transfers(reference_id);
CREATE INDEX idx_outgoing_transfers_created_at ON outgoing_transfers(created_at);