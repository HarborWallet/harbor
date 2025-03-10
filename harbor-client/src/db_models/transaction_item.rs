use crate::MintIdentifier;
use crate::db_models::PaymentStatus;
use bitcoin::Txid;
use bitcoin::hashes::Hash;
use fedimint_core::config::FederationId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionItemKind {
    Lightning,
    Onchain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionItem {
    pub kind: TransactionItemKind,
    pub amount: u64,
    pub txid: Option<Txid>,
    pub direction: TransactionDirection,
    pub mint_identifier: MintIdentifier,
    pub status: PaymentStatus,
    pub timestamp: u64,
}

impl TransactionItem {
    pub fn make_dummy() -> Self {
        Self {
            kind: TransactionItemKind::Lightning,
            amount: 100,
            txid: None,
            direction: TransactionDirection::Incoming,
            mint_identifier: MintIdentifier::Fedimint(FederationId::dummy()),
            status: PaymentStatus::Success,
            timestamp: 0,
        }
    }

    pub fn make_dummy_onchain() -> Self {
        Self {
            kind: TransactionItemKind::Onchain,
            amount: 100,
            txid: Some(Txid::all_zeros()),
            direction: TransactionDirection::Outgoing,
            mint_identifier: MintIdentifier::Fedimint(FederationId::dummy()),
            status: PaymentStatus::Success,
            timestamp: 0,
        }
    }
}
