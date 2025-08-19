-- Откат миграции - удаление таблицы исходящих трансферов
DROP INDEX IF EXISTS idx_outgoing_transfers_created_at;
DROP INDEX IF EXISTS idx_outgoing_transfers_reference_id;
DROP INDEX IF EXISTS idx_outgoing_transfers_status;
DROP INDEX IF EXISTS idx_outgoing_transfers_tx_hash;
DROP INDEX IF EXISTS idx_outgoing_transfers_from_wallet_id;
DROP TABLE IF EXISTS outgoing_transfers;