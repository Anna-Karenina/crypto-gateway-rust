-- Откат миграции - удаление таблицы кошельков
DROP INDEX IF EXISTS idx_wallets_created_at;
DROP INDEX IF EXISTS idx_wallets_address; 
DROP INDEX IF EXISTS idx_wallets_owner_id;
DROP TABLE IF EXISTS wallets;