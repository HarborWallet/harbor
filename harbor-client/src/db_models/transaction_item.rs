use bitcoin::hashes::Hash;
use bitcoin::Txid;
use fedimint_core::config::FederationId;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionItemKind {
    Lightning,
    Onchain,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransactionItem {
    pub kind: TransactionItemKind,
    pub amount: u64,
    pub txid: Option<Txid>,
    pub direction: TransactionDirection,
    pub federation_id: FederationId,
    pub timestamp: u64,
}

impl TransactionItem {
    pub fn make_dummy() -> Self {
        Self {
            kind: TransactionItemKind::Lightning,
            amount: 100,
            txid: None,
            direction: TransactionDirection::Incoming,
            federation_id: FederationId::dummy(),
            timestamp: 0,
        }
    }

    pub fn make_dummy_onchain() -> Self {
        Self {
            kind: TransactionItemKind::Onchain,
            amount: 100,
            txid: Some(Txid::all_zeros()),
            direction: TransactionDirection::Outgoing,
            federation_id: FederationId::dummy(),
            timestamp: 0,
        }
    }
}
