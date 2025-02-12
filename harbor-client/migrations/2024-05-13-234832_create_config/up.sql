CREATE TABLE profile
(
    id                      TEXT PRIMARY KEY NOT NULL,
    seed_words              TEXT             NOT NULL,
    onchain_receive_enabled INTEGER          NOT NULL DEFAULT 0,
    tor_enabled             INTEGER          NOT NULL DEFAULT 1
);

CREATE TABLE fedimint
(
    id          TEXT PRIMARY KEY NOT NULL,
    invite_code TEXT             NOT NULL UNIQUE,
    value       BLOB             NOT NULL,
    active      INTEGER          NOT NULL DEFAULT 1
);

CREATE TABLE mint_metadata
(
    id                          TEXT PRIMARY KEY NOT NULL,
    name                        TEXT,
    welcome_message             TEXT,
    federation_expiry_timestamp TIMESTAMP,
    preview_message             TEXT,
    popup_end_timestamp         TIMESTAMP,
    popup_countdown_message     TEXT,
    created_at                  TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                  TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE lightning_payments
(
    operation_id TEXT PRIMARY KEY NOT NULL,
    fedimint_id  TEXT             NOT NULL REFERENCES fedimint (id),
    payment_hash TEXT             NOT NULL,
    bolt11       TEXT             NOT NULL,
    amount_msats BIGINT           NOT NULL,
    fee_msats    BIGINT           NOT NULL,
    preimage     TEXT,
    status       INTEGER          NOT NULL,
    created_at   TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE lightning_receives
(
    operation_id TEXT PRIMARY KEY NOT NULL,
    fedimint_id  TEXT             NOT NULL REFERENCES fedimint (id),
    payment_hash TEXT             NOT NULL,
    bolt11       TEXT             NOT NULL,
    amount_msats BIGINT           NOT NULL,
    fee_msats    BIGINT           NOT NULL,
    preimage     TEXT             NOT NULL,
    status       INTEGER          NOT NULL,
    created_at   TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE on_chain_payments
(
    operation_id TEXT PRIMARY KEY NOT NULL,
    fedimint_id  TEXT             NOT NULL REFERENCES fedimint (id),
    address      TEXT             NOT NULL,
    amount_sats  BIGINT           NOT NULL,
    fee_sats     BIGINT           NOT NULL,
    txid         TEXT,
    status       INTEGER          NOT NULL,
    created_at   TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE on_chain_receives
(
    operation_id TEXT PRIMARY KEY NOT NULL,
    fedimint_id  TEXT             NOT NULL REFERENCES fedimint (id),
    address      TEXT             NOT NULL,
    amount_sats  BIGINT,
    fee_sats     BIGINT,
    txid         TEXT,
    status       INTEGER          NOT NULL,
    created_at   TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create triggers to set the updated_at timestamps on update
CREATE TRIGGER update_timestamp_mint_metadata
    AFTER UPDATE
    ON mint_metadata
    FOR EACH ROW
BEGIN
UPDATE mint_metadata
SET updated_at = CURRENT_TIMESTAMP
WHERE id = OLD.id;
END;
CREATE TRIGGER update_timestamp_lightning_payments
    AFTER UPDATE
    ON lightning_payments
    FOR EACH ROW
BEGIN
UPDATE lightning_payments
SET updated_at = CURRENT_TIMESTAMP
WHERE operation_id = OLD.operation_id;
END;
CREATE TRIGGER update_timestamp_lightning_receives
    AFTER UPDATE
    ON lightning_receives
    FOR EACH ROW
BEGIN
UPDATE lightning_receives
SET updated_at = CURRENT_TIMESTAMP
WHERE operation_id = OLD.operation_id;
END;
CREATE TRIGGER update_timestamp_on_chain_payments
    AFTER UPDATE
    ON on_chain_payments
    FOR EACH ROW
BEGIN
UPDATE on_chain_payments
SET updated_at = CURRENT_TIMESTAMP
WHERE operation_id = OLD.operation_id;
END;
CREATE TRIGGER update_timestamp_on_chain_receives
    AFTER UPDATE
    ON on_chain_receives
    FOR EACH ROW
BEGIN
UPDATE on_chain_receives
SET updated_at = CURRENT_TIMESTAMP
WHERE operation_id = OLD.operation_id;
END;
