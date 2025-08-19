-- Откат миграции - удаление таблицы входящих транзакций
DROP INDEX IF EXISTS idx_incoming_transactions_detected_at;
DROP INDEX IF EXISTS idx_incoming_transactions_status;
DROP INDEX IF EXISTS idx_incoming_transactions_tx_hash;
DROP INDEX IF EXISTS idx_incoming_transactions_wallet_id;
DROP TABLE IF EXISTS incoming_transactions;