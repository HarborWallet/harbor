-- This file should undo anything in `up.sql`
-- Down migration: revert Bolt12 support changes
-- Recreate the original tables with NOT NULL constraints and without bolt12_offer

-- Drop triggers first
DROP TRIGGER IF EXISTS update_timestamp_lightning_payments;
DROP TRIGGER IF EXISTS update_timestamp_lightning_receives;

-- Create original tables
CREATE TABLE lightning_payments_old
(
    operation_id   TEXT PRIMARY KEY NOT NULL,
    fedimint_id    TEXT REFERENCES fedimint (id),
    cashu_mint_url TEXT REFERENCES cashu_mint (mint_url),
    payment_hash   TEXT             NOT NULL,
    bolt11         TEXT             NOT NULL,
    amount_msats   BIGINT           NOT NULL,
    fee_msats      BIGINT           NOT NULL,
    preimage       TEXT,
    status         INTEGER          NOT NULL,
    created_at     TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE lightning_receives_old
(
    operation_id   TEXT PRIMARY KEY NOT NULL,
    fedimint_id    TEXT REFERENCES fedimint (id),
    cashu_mint_url TEXT REFERENCES cashu_mint (mint_url),
    payment_hash   TEXT             NOT NULL,
    bolt11         TEXT             NOT NULL,
    amount_msats   BIGINT           NOT NULL,
    fee_msats      BIGINT           NOT NULL,
    status         INTEGER          NOT NULL,
    created_at     TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Copy data back from current tables to old schema dropping bolt12 columns
-- Rows that relied on bolt12_offer only will be dropped because NOT NULL columns cannot be populated
INSERT INTO lightning_payments_old (
    operation_id, fedimint_id, cashu_mint_url, payment_hash, bolt11, amount_msats, fee_msats, preimage, status, created_at, updated_at
)
SELECT operation_id, fedimint_id, cashu_mint_url, payment_hash, bolt11, amount_msats, fee_msats, preimage, status, created_at, updated_at
FROM lightning_payments
WHERE payment_hash IS NOT NULL AND bolt11 IS NOT NULL;

INSERT INTO lightning_receives_old (
    operation_id, fedimint_id, cashu_mint_url, payment_hash, bolt11, amount_msats, fee_msats, status, created_at, updated_at
)
SELECT operation_id, fedimint_id, cashu_mint_url, payment_hash, bolt11, amount_msats, fee_msats, status, created_at, updated_at
FROM lightning_receives
WHERE payment_hash IS NOT NULL AND bolt11 IS NOT NULL;

-- Replace tables
DROP TABLE lightning_payments;
ALTER TABLE lightning_payments_old RENAME TO lightning_payments;

DROP TABLE lightning_receives;
ALTER TABLE lightning_receives_old RENAME TO lightning_receives;

-- Recreate triggers
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
