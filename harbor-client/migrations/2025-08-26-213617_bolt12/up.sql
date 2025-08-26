-- Your SQL goes here
-- Add Bolt12 support by allowing nullable bolt11/payment_hash and adding bolt12_offer columns
-- We need to recreate the lightning_payments and lightning_receives tables because SQLite
-- cannot drop NOT NULL constraints directly.

-- 1) Drop triggers so we can replace the tables
DROP TRIGGER IF EXISTS update_timestamp_lightning_payments;
DROP TRIGGER IF EXISTS update_timestamp_lightning_receives;

-- 2) Create new tables with the updated schema
CREATE TABLE lightning_payments_new
(
    operation_id   TEXT PRIMARY KEY NOT NULL,
    fedimint_id    TEXT REFERENCES fedimint (id),
    cashu_mint_url TEXT REFERENCES cashu_mint (mint_url),
    payment_hash   TEXT,              -- now nullable to support bolt12
    bolt11         TEXT,              -- now nullable to support bolt12
    bolt12_offer   TEXT,              -- new column for bolt12 offers ("lno...")
    amount_msats   BIGINT           NOT NULL,
    fee_msats      BIGINT           NOT NULL,
    preimage       TEXT,
    status         INTEGER          NOT NULL,
    created_at     TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE lightning_receives_new
(
    operation_id   TEXT PRIMARY KEY NOT NULL,
    fedimint_id    TEXT REFERENCES fedimint (id),
    cashu_mint_url TEXT REFERENCES cashu_mint (mint_url),
    payment_hash   TEXT,              -- now nullable to support bolt12
    bolt11         TEXT,              -- now nullable to support bolt12
    bolt12_offer   TEXT,              -- new column for bolt12 offers ("lno...")
    amount_msats   BIGINT           NOT NULL,
    fee_msats      BIGINT           NOT NULL,
    status         INTEGER          NOT NULL,
    created_at     TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 3) Copy over existing data
INSERT INTO lightning_payments_new (
    operation_id, fedimint_id, cashu_mint_url, payment_hash, bolt11, amount_msats, fee_msats, preimage, status, created_at, updated_at
)
SELECT operation_id, fedimint_id, cashu_mint_url, payment_hash, bolt11, amount_msats, fee_msats, preimage, status, created_at, updated_at
FROM lightning_payments;

INSERT INTO lightning_receives_new (
    operation_id, fedimint_id, cashu_mint_url, payment_hash, bolt11, amount_msats, fee_msats, status, created_at, updated_at
)
SELECT operation_id, fedimint_id, cashu_mint_url, payment_hash, bolt11, amount_msats, fee_msats, status, created_at, updated_at
FROM lightning_receives;

-- 4) Replace old tables
DROP TABLE lightning_payments;
ALTER TABLE lightning_payments_new RENAME TO lightning_payments;

DROP TABLE lightning_receives;
ALTER TABLE lightning_receives_new RENAME TO lightning_receives;

-- 5) Recreate triggers
CREATE TRIGGER update_timestamp_lightning_payments
    AFTER UPDATE ON lightning_payments
    FOR EACH ROW
BEGIN
    UPDATE lightning_payments
    SET updated_at = CURRENT_TIMESTAMP
    WHERE operation_id = OLD.operation_id;
END;

CREATE TRIGGER update_timestamp_lightning_receives
    AFTER UPDATE ON lightning_receives
    FOR EACH ROW
BEGIN
    UPDATE lightning_receives
    SET updated_at = CURRENT_TIMESTAMP
    WHERE operation_id = OLD.operation_id;
END;
