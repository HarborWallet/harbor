use crate::components::{TransactionDirection, TransactionItem, TransactionItemKind};
use crate::db_models::schema::on_chain_payments;
use crate::db_models::PaymentStatus;
use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Txid};
use diesel::prelude::*;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use std::str::FromStr;

#[derive(QueryableByName, Queryable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = on_chain_payments)]
pub struct OnChainPayment {
    operation_id: String,
    fedimint_id: String,
    address: String,
    pub amount_sats: i64,
    pub fee_sats: i64,
    txid: Option<String>,
    status: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = on_chain_payments)]
struct NewOnChainPayment {
    operation_id: String,
    fedimint_id: String,
    address: String,
    amount_sats: i64,
    fee_sats: i64,
    status: i32,
}

impl OnChainPayment {
    pub fn operation_id(&self) -> OperationId {
        OperationId::from_str(&self.operation_id).expect("invalid operation id")
    }

    pub fn fedimint_id(&self) -> FederationId {
        FederationId::from_str(&self.fedimint_id).expect("invalid fedimint id")
    }

    pub fn address(&self) -> Address<NetworkUnchecked> {
        Address::from_str(&self.address).expect("invalid address")
    }

    pub fn txid(&self) -> Option<Txid> {
        self.txid
            .as_ref()
            .map(|p| Txid::from_str(p).expect("invalid txid"))
    }

    pub fn status(&self) -> PaymentStatus {
        PaymentStatus::from_i32(self.status)
    }

    pub fn create(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
        fedimint_id: FederationId,
        address: Address<NetworkUnchecked>,
        amount_sats: u64,
        fee_sats: u64,
    ) -> anyhow::Result<()> {
        let new = NewOnChainPayment {
            operation_id: operation_id.fmt_full().to_string(),
            fedimint_id: fedimint_id.to_string(),
            address: address.assume_checked().to_string(),
            amount_sats: amount_sats as i64,
            fee_sats: fee_sats as i64,
            status: PaymentStatus::Pending as i32,
        };

        diesel::insert_into(on_chain_payments::table)
            .values(new)
            .execute(conn)?;

        Ok(())
    }

    pub fn get_by_operation_id(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
    ) -> anyhow::Result<Option<Self>> {
        Ok(on_chain_payments::table
            .filter(on_chain_payments::operation_id.eq(operation_id.fmt_full().to_string()))
            .first::<Self>(conn)
            .optional()?)
    }

    pub fn set_txid(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
        txid: Txid,
    ) -> anyhow::Result<()> {
        diesel::update(
            on_chain_payments::table
                .filter(on_chain_payments::operation_id.eq(operation_id.fmt_full().to_string())),
        )
        .set((
            on_chain_payments::txid.eq(Some(txid.to_string())),
            // fedimint doesn't tell us when the tx is confirmed so just jump to success
            on_chain_payments::status.eq(PaymentStatus::Success as i32),
        ))
        .execute(conn)?;

        Ok(())
    }

    pub fn mark_as_failed(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
    ) -> anyhow::Result<()> {
        diesel::update(
            on_chain_payments::table
                .filter(on_chain_payments::operation_id.eq(operation_id.fmt_full().to_string())),
        )
        .set(on_chain_payments::status.eq(PaymentStatus::Failed as i32))
        .execute(conn)?;

        Ok(())
    }

    pub fn get_history(conn: &mut SqliteConnection) -> anyhow::Result<Vec<Self>> {
        Ok(on_chain_payments::table
            .filter(on_chain_payments::status.eq(PaymentStatus::Success as i32))
            .load::<Self>(conn)?)
    }
}

impl From<OnChainPayment> for TransactionItem {
    fn from(payment: OnChainPayment) -> Self {
        Self {
            kind: TransactionItemKind::Onchain,
            amount: payment.amount_sats as u64,
            direction: TransactionDirection::Outgoing,
            timestamp: payment.created_at.and_utc().timestamp() as u64,
        }
    }
}
