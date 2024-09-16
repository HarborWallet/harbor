use crate::components::{TransactionDirection, TransactionItem, TransactionItemKind};
use crate::db_models::schema::lightning_payments;
use crate::db_models::PaymentStatus;
use bitcoin::hashes::hex::FromHex;
use diesel::prelude::*;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_core::Amount;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use std::str::FromStr;

#[derive(QueryableByName, Queryable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = lightning_payments)]
pub struct LightningPayment {
    operation_id: String,
    fedimint_id: String,
    payment_hash: String,
    bolt11: String,
    amount_msats: i64,
    fee_msats: i64,
    preimage: Option<String>,
    status: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = lightning_payments)]
struct NewLightningPayment {
    operation_id: String,
    fedimint_id: String,
    payment_hash: String,
    bolt11: String,
    amount_msats: i64,
    fee_msats: i64,
    status: i32,
}

impl LightningPayment {
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

    pub fn preimage(&self) -> Option<[u8; 32]> {
        self.preimage
            .as_ref()
            .map(|p| FromHex::from_hex(p).expect("invalid preimage"))
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
    ) -> anyhow::Result<()> {
        // Make sure the amount matches
        if bolt11
            .amount_milli_satoshis()
            .is_some_and(|a| a != amount.msats)
        {
            return Err(anyhow::anyhow!("Internal error: amount mismatch"));
        }

        let payment_hash = bolt11.payment_hash().to_string();
        let new = NewLightningPayment {
            operation_id: operation_id.fmt_full().to_string(),
            fedimint_id: fedimint_id.to_string(),
            payment_hash,
            bolt11: bolt11.to_string(),
            amount_msats: amount.msats as i64,
            fee_msats: fee.msats as i64,
            status: PaymentStatus::Pending as i32,
        };

        diesel::insert_into(lightning_payments::table)
            .values(new)
            .execute(conn)?;

        Ok(())
    }

    pub fn get_by_operation_id(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
    ) -> anyhow::Result<Option<Self>> {
        Ok(lightning_payments::table
            .filter(lightning_payments::operation_id.eq(operation_id.fmt_full().to_string()))
            .first::<Self>(conn)
            .optional()?)
    }

    pub fn set_preimage(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
        preimage: [u8; 32],
    ) -> anyhow::Result<()> {
        diesel::update(
            lightning_payments::table
                .filter(lightning_payments::operation_id.eq(operation_id.fmt_full().to_string())),
        )
        .set((
            lightning_payments::preimage.eq(Some(hex::encode(preimage))),
            lightning_payments::status.eq(PaymentStatus::Success as i32),
        ))
        .execute(conn)?;

        Ok(())
    }

    pub fn mark_as_failed(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
    ) -> anyhow::Result<()> {
        diesel::update(
            lightning_payments::table
                .filter(lightning_payments::operation_id.eq(operation_id.fmt_full().to_string())),
        )
        .set(lightning_payments::status.eq(PaymentStatus::Failed as i32))
        .execute(conn)?;

        Ok(())
    }

    pub fn get_history(conn: &mut SqliteConnection) -> anyhow::Result<Vec<Self>> {
        Ok(lightning_payments::table
            .filter(lightning_payments::status.eq(PaymentStatus::Success as i32))
            .load::<Self>(conn)?)
    }
}

impl From<LightningPayment> for TransactionItem {
    fn from(payment: LightningPayment) -> Self {
        Self {
            kind: TransactionItemKind::Lightning,
            amount: payment.amount().sats_round_down(),
            direction: TransactionDirection::Outgoing,
            timestamp: payment.created_at.and_utc().timestamp() as u64,
        }
    }
}
