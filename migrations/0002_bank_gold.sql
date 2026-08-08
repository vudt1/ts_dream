-- Add the explicit bank balance to databases created before ticket #15.
-- MySQL has no portable IF NOT EXISTS form for ADD COLUMN, so deployments
-- should run this migration once through SQLx before serving old databases.
ALTER TABLE players ADD COLUMN BankGold BIGINT DEFAULT 0 AFTER Gold;
