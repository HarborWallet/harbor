use crate::components::{TransactionDirection, TransactionItem, TransactionItemKind};
use crate::db_models::schema::lightning_receives;
use crate::db_models::PaymentStatus;
use bitcoin::hashes::hex::FromHex;
use diesel::prelude::*;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_core::Amount;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use std::str::FromStr;

#[derive(QueryableByName, Queryable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = lightning_receives)]
pub struct LightningReceive {
    operation_id: String,
    fedimint_id: String,
    payment_hash: String,
    bolt11: String,
    amount_msats: i64,
    fee_msats: i64,
    preimage: String,
    status: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = lightning_receives)]
struct NewLightningReceive {
    operation_id: String,
    fedimint_id: String,
    payment_hash: String,
    bolt11: String,
    amount_msats: i64,
    fee_msats: i64,
    preimage: String,
    status: i32,
}

impl LightningReceive {
    pub fn operation_id(&self) -> OperationId {
        OperationId::from_str(&self.operation_id).expect("invalid operation id")
    }

    pub fn fedimint_id(&self) -> FederationId {
        FederationId::from_str(&self.fedimint_id).expect("invalid fedimint id")
    }

    pub fn payment_hash(&self) -> [u8; 32] {
        FromHex::from_hex(&self.payment_hash).expect("invalid payment hash")
    }

    pub fn bolt11(&self) -> Bolt11Invoice {
        Bolt11Invoice::from_str(&self.bolt11).expect("invalid bolt11")
    }

    pub fn amount(&self) -> Amount {
        Amount::from_msats(self.amount_msats as u64)
    }

    pub fn fee(&self) -> Amount {
        Amount::from_msats(self.fee_msats as u64)
    }

    pub fn preimage(&self) -> [u8; 32] {
        FromHex::from_hex(&self.preimage).expect("invalid preimage")
    }

    pub fn status(&self) -> PaymentStatus {
        PaymentStatus::from_i32(self.status)
    }

    pub fn create(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
        fedimint_id: FederationId,
        bolt11: Bolt11Invoice,
        amount: Amount,
        fee: Amount,
        preimage: [u8; 32],
    ) -> anyhow::Result<()> {
        // Make sure the amount matches
        if bolt11
            .amount_milli_satoshis()
            .is_some_and(|a| a != amount.msats)
        {
            return Err(anyhow::anyhow!("Internal error: amount mismatch"));
        }

        let payment_hash = bolt11.payment_hash().to_string();
        let new = NewLightningReceive {
            operation_id: operation_id.fmt_full().to_string(),
            fedimint_id: fedimint_id.to_string(),
            payment_hash,
            bolt11: bolt11.to_string(),
            amount_msats: amount.msats as i64,
            fee_msats: fee.msats as i64,
            preimage: hex::encode(preimage),
            status: PaymentStatus::Pending as i32,
        };

        diesel::insert_into(lightning_receives::table)
            .values(new)
            .execute(conn)?;

        Ok(())
    }

    pub fn get_by_operation_id(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
    ) -> anyhow::Result<Option<Self>> {
        Ok(lightning_receives::table
            .filter(lightning_receives::operation_id.eq(operation_id.fmt_full().to_string()))
            .first::<Self>(conn)
            .optional()?)
    }

    pub fn mark_as_success(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
    ) -> anyhow::Result<()> {
        diesel::update(
            lightning_receives::table
                .filter(lightning_receives::operation_id.eq(operation_id.fmt_full().to_string())),
        )
        .set(lightning_receives::status.eq(PaymentStatus::Success as i32))
        .execute(conn)?;

        Ok(())
    }

    pub fn mark_as_failed(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
    ) -> anyhow::Result<()> {
        diesel::update(
            lightning_receives::table
                .filter(lightning_receives::operation_id.eq(operation_id.fmt_full().to_string())),
        )
        .set(lightning_receives::status.eq(PaymentStatus::Failed as i32))
        .execute(conn)?;

        Ok(())
    }

    pub fn get_history(conn: &mut SqliteConnection) -> anyhow::Result<Vec<Self>> {
        Ok(lightning_receives::table
            .filter(lightning_receives::status.eq(PaymentStatus::Success as i32))
            .load::<Self>(conn)?)
    }
}

impl From<LightningReceive> for TransactionItem {
    fn from(payment: LightningReceive) -> Self {
        Self {
            kind: TransactionItemKind::Lightning,
            amount: payment.amount().sats_round_down(),
            direction: TransactionDirection::Incoming,
            timestamp: payment.created_at.and_utc().timestamp() as u64,
        }
    }
}
