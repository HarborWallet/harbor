-- Add table to store individual payments for Bolt12 receives.
-- Bolt12 offers can be paid multiple times, so we store each successful payment
-- as a separate row linked to the original receive (lightning_receives.operation_id).

CREATE TABLE IF NOT EXISTS lightning_receive_payments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    receive_operation_id TEXT NOT NULL REFERENCES lightning_receives(operation_id) ON DELETE CASCADE,
    amount_msats BIGINT NOT NULL,
    fee_msats BIGINT NOT NULL DEFAULT 0,
    payment_hash TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index to quickly lookup payments for a given receive
CREATE INDEX IF NOT EXISTS idx_lightning_receive_payments_receive_op_id
    ON lightning_receive_payments (receive_operation_id);

-- Index to order by creation time for history queries
CREATE INDEX IF NOT EXISTS idx_lightning_receive_payments_created_at
    ON lightning_receive_payments (created_at);
