use crate::MintIdentifier;
use crate::db_models::PaymentStatus;
use crate::db_models::schema::{lightning_receive_payments, lightning_receives};
use crate::db_models::transaction_item::{
    TransactionDirection, TransactionItem, TransactionItemKind,
};
use bitcoin::hashes::hex::FromHex;
use cdk::mint_url::MintUrl;
use diesel::prelude::*;
use fedimint_core::Amount;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use std::str::FromStr;

#[derive(QueryableByName, Queryable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = lightning_receives)]
pub struct LightningReceive {
    pub operation_id: String,
    fedimint_id: Option<String>,
    cashu_mint_url: Option<String>,
    payment_hash: Option<String>,
    bolt11: Option<String>,
    bolt12_offer: Option<String>,
    amount_msats: i64,
    fee_msats: i64,
    status: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = lightning_receives)]
struct NewLightningReceive {
    operation_id: String,
    fedimint_id: Option<String>,
    cashu_mint_url: Option<String>,
    payment_hash: Option<String>,
    bolt11: Option<String>,
    bolt12_offer: Option<String>,
    amount_msats: i64,
    fee_msats: i64,
    status: i32,
}

#[derive(Queryable, Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = lightning_receive_payments)]
pub struct LightningReceivePayment {
    pub id: i32,
    pub receive_operation_id: String,
    pub amount_msats: i64,
    pub fee_msats: i64,
    pub payment_hash: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = lightning_receive_payments)]
struct NewLightningReceivePayment {
    pub receive_operation_id: String,
    pub amount_msats: i64,
    pub fee_msats: i64,
    pub payment_hash: Option<String>,
}

impl LightningReceive {
    pub fn operation_id(&self) -> OperationId {
        OperationId::from_str(&self.operation_id).expect("invalid operation id")
    }

    pub fn fedimint_id(&self) -> Option<FederationId> {
        self.fedimint_id
            .as_ref()
            .map(|f| FederationId::from_str(f).expect("invalid fedimint_id"))
    }

    pub fn mint_url(&self) -> Option<MintUrl> {
        self.cashu_mint_url
            .as_ref()
            .map(|url| MintUrl::from_str(url).expect("invalid mint url"))
    }

    pub fn mint_identifier(&self) -> MintIdentifier {
        match self.fedimint_id() {
            Some(f) => MintIdentifier::Fedimint(f),
            None => MintIdentifier::Cashu(self.mint_url().expect("missing mint url")),
        }
    }

    pub fn payment_hash(&self) -> Option<[u8; 32]> {
        self.payment_hash
            .as_ref()
            .map(|h| FromHex::from_hex(h).expect("invalid payment hash"))
    }

    pub fn bolt11(&self) -> Option<Bolt11Invoice> {
        self.bolt11
            .as_ref()
            .map(|b| Bolt11Invoice::from_str(b).expect("invalid bolt11"))
    }

    pub fn bolt12_offer(&self) -> Option<&str> {
        self.bolt12_offer.as_deref()
    }

    pub fn amount(&self) -> Amount {
        Amount::from_msats(self.amount_msats as u64)
    }

    pub fn fee(&self) -> Amount {
        Amount::from_msats(self.fee_msats as u64)
    }

    pub fn status(&self) -> PaymentStatus {
        PaymentStatus::from_i32(self.status)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create(
        conn: &mut SqliteConnection,
        operation_id: String,
        fedimint_id: Option<FederationId>,
        cashu_mint_url: Option<MintUrl>,
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
        let new = NewLightningReceive {
            operation_id,
            fedimint_id: fedimint_id.map(|f| f.to_string()),
            cashu_mint_url: cashu_mint_url.map(|f| f.to_string()),
            payment_hash: Some(payment_hash),
            bolt11: Some(bolt11.to_string()),
            bolt12_offer: None,
            amount_msats: amount.msats as i64,
            fee_msats: fee.msats as i64,
            status: PaymentStatus::Pending as i32,
        };

        diesel::insert_into(lightning_receives::table)
            .values(new)
            .execute(conn)?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_bolt12(
        conn: &mut SqliteConnection,
        operation_id: String,
        fedimint_id: Option<FederationId>,
        cashu_mint_url: Option<MintUrl>,
        offer: String,
        amount: Amount,
        fee: Amount,
    ) -> anyhow::Result<()> {
        let new = NewLightningReceive {
            operation_id,
            fedimint_id: fedimint_id.map(|f| f.to_string()),
            cashu_mint_url: cashu_mint_url.map(|f| f.to_string()),
            payment_hash: None,
            bolt11: None,
            bolt12_offer: Some(offer),
            amount_msats: amount.msats as i64,
            fee_msats: fee.msats as i64,
            status: PaymentStatus::Pending as i32,
        };

        diesel::insert_into(lightning_receives::table)
            .values(new)
            .execute(conn)?;

        Ok(())
    }

    pub fn get_by_operation_id(
        conn: &mut SqliteConnection,
        operation_id: String,
    ) -> anyhow::Result<Option<Self>> {
        Ok(lightning_receives::table
            .filter(lightning_receives::operation_id.eq(operation_id))
            .first::<Self>(conn)
            .optional()?)
    }

    pub fn mark_as_success(
        conn: &mut SqliteConnection,
        operation_id: String,
        amount_msats: Option<u64>,
    ) -> anyhow::Result<()> {
        use crate::db_models::schema::lightning_receives::dsl as lr;

        // fetch the existing receive record
        let existing: Option<LightningReceive> = lr::lightning_receives
            .filter(lr::operation_id.eq(&operation_id))
            .order(lr::updated_at.desc())
            .first::<LightningReceive>(conn)
            .optional()?;

        if let Some(rec) = existing {
            if rec.bolt12_offer.is_some() {
                let new_amount = amount_msats.map(|a| a as i64).unwrap_or(rec.amount_msats);

                let new_payment = NewLightningReceivePayment {
                    receive_operation_id: rec.operation_id.clone(),
                    amount_msats: new_amount,
                    fee_msats: rec.fee_msats,
                    payment_hash: rec.payment_hash.clone(),
                };

                diesel::insert_into(lightning_receive_payments::table)
                    .values(new_payment)
                    .execute(conn)?;

                // Update the receive summary row to Success so it appears in history and update timestamp
                diesel::update(
                    lr::lightning_receives.filter(lr::operation_id.eq(rec.operation_id.clone())),
                )
                .set(lr::status.eq(PaymentStatus::Success as i32))
                .execute(conn)?;
            } else {
                // For bolt11 invoices update the existing row to success
                diesel::update(
                    lightning_receives::table
                        .filter(lightning_receives::operation_id.eq(operation_id)),
                )
                .set(lightning_receives::status.eq(PaymentStatus::Success as i32))
                .execute(conn)?;
            }
        }

        Ok(())
    }

    pub fn mark_as_failed(conn: &mut SqliteConnection, operation_id: String) -> anyhow::Result<()> {
        diesel::update(
            lightning_receives::table.filter(lightning_receives::operation_id.eq(operation_id)),
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

    pub fn get_pending(conn: &mut SqliteConnection) -> anyhow::Result<Vec<Self>> {
        Ok(lightning_receives::table
            .filter(
                lightning_receives::status
                    .eq_any([
                        PaymentStatus::Pending as i32,
                        PaymentStatus::WaitingConfirmation as i32,
                    ])
                    .or(lightning_receives::bolt12_offer.is_not_null()),
            )
            .load::<Self>(conn)?)
    }

    pub fn get_bolt12_payments_history(
        conn: &mut SqliteConnection,
    ) -> anyhow::Result<Vec<(LightningReceivePayment, LightningReceive)>> {
        use crate::db_models::schema::lightning_receive_payments::dsl as lrp;
        use crate::db_models::schema::lightning_receives::dsl as lr;

        let results = lrp::lightning_receive_payments
            .inner_join(lr::lightning_receives.on(lrp::receive_operation_id.eq(lr::operation_id)))
            .select((
                (
                    lrp::id,
                    lrp::receive_operation_id,
                    lrp::amount_msats,
                    lrp::fee_msats,
                    lrp::payment_hash,
                    lrp::created_at,
                ),
                (
                    lr::operation_id,
                    lr::fedimint_id,
                    lr::cashu_mint_url,
                    lr::payment_hash,
                    lr::bolt11,
                    lr::bolt12_offer,
                    lr::amount_msats,
                    lr::fee_msats,
                    lr::status,
                    lr::created_at,
                    lr::updated_at,
                ),
            ))
            .load::<(LightningReceivePayment, LightningReceive)>(conn)?;

        Ok(results)
    }
}

impl From<LightningReceive> for TransactionItem {
    fn from(payment: LightningReceive) -> Self {
        Self {
            kind: TransactionItemKind::Lightning,
            amount: payment.amount().sats_round_down(),
            fee_msats: payment.fee_msats as u64,
            txid: None,
            preimage: None,
            direction: TransactionDirection::Incoming,
            mint_identifier: payment.mint_identifier(),
            status: payment.status(),
            timestamp: payment.updated_at.and_utc().timestamp() as u64,
        }
    }
}
