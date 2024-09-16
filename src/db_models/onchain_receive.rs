use crate::components::{TransactionDirection, TransactionItem, TransactionItemKind};
use crate::db_models::schema::on_chain_receives;
use crate::db_models::PaymentStatus;
use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Txid};
use diesel::prelude::*;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use std::str::FromStr;

#[derive(QueryableByName, Queryable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = on_chain_receives)]
pub struct OnChainReceive {
    operation_id: String,
    fedimint_id: String,
    address: String,
    pub amount_sats: Option<i64>,
    pub fee_sats: Option<i64>,
    txid: Option<String>,
    status: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = on_chain_receives)]
struct NewOnChainReceive {
    operation_id: String,
    fedimint_id: String,
    address: String,
    status: i32,
}

impl OnChainReceive {
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
        address: Address,
    ) -> anyhow::Result<()> {
        let new = NewOnChainReceive {
            operation_id: operation_id.fmt_full().to_string(),
            fedimint_id: fedimint_id.to_string(),
            address: address.to_string(),
            status: PaymentStatus::Pending as i32,
        };

        diesel::insert_into(on_chain_receives::table)
            .values(new)
            .execute(conn)?;

        Ok(())
    }

    pub fn get_by_operation_id(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
    ) -> anyhow::Result<Option<Self>> {
        Ok(on_chain_receives::table
            .filter(on_chain_receives::operation_id.eq(operation_id.fmt_full().to_string()))
            .first::<Self>(conn)
            .optional()?)
    }

    pub fn set_txid(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
        txid: Txid,
        amount_sats: u64,
        fee_sats: u64,
    ) -> anyhow::Result<()> {
        diesel::update(
            on_chain_receives::table
                .filter(on_chain_receives::operation_id.eq(operation_id.fmt_full().to_string())),
        )
        .set((
            on_chain_receives::txid.eq(Some(txid.to_string())),
            on_chain_receives::amount_sats.eq(Some(amount_sats as i64)),
            on_chain_receives::fee_sats.eq(Some(fee_sats as i64)),
            on_chain_receives::status.eq(PaymentStatus::WaitingConfirmation as i32),
        ))
        .execute(conn)?;

        Ok(())
    }

    pub fn mark_as_confirmed(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
    ) -> anyhow::Result<()> {
        diesel::update(
            on_chain_receives::table
                .filter(on_chain_receives::operation_id.eq(operation_id.fmt_full().to_string()))
                .filter(on_chain_receives::txid.is_not_null()), // make sure it has a txid
        )
        .set(on_chain_receives::status.eq(PaymentStatus::Success as i32))
        .execute(conn)?;

        Ok(())
    }

    pub fn mark_as_failed(
        conn: &mut SqliteConnection,
        operation_id: OperationId,
    ) -> anyhow::Result<()> {
        diesel::update(
            on_chain_receives::table
                .filter(on_chain_receives::operation_id.eq(operation_id.fmt_full().to_string())),
        )
        .set(on_chain_receives::status.eq(PaymentStatus::Failed as i32))
        .execute(conn)?;

        Ok(())
    }

    pub fn get_history(conn: &mut SqliteConnection) -> anyhow::Result<Vec<Self>> {
        Ok(on_chain_receives::table
            .filter(
                on_chain_receives::status
                    .eq(PaymentStatus::Success as i32)
                    .or(on_chain_receives::status.eq(PaymentStatus::WaitingConfirmation as i32)),
            )
            .load::<Self>(conn)?)
    }
}

impl From<OnChainReceive> for TransactionItem {
    fn from(payment: OnChainReceive) -> Self {
        Self {
            kind: TransactionItemKind::Onchain,
            amount: payment.amount_sats.unwrap_or(0) as u64, // todo handle this better
            direction: TransactionDirection::Incoming,
            timestamp: payment.created_at.and_utc().timestamp() as u64,
        }
    }
}
