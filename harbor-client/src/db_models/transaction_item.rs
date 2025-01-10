use bitcoin::hashes::Hash;
use bitcoin::Txid;

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

#[derive(Debug, Clone, Copy)]
pub struct TransactionItem {
    pub kind: TransactionItemKind,
    pub amount: u64,
    pub txid: Option<Txid>,
    pub direction: TransactionDirection,
    pub timestamp: u64,
}

impl TransactionItem {
    pub fn make_dummy() -> Self {
        Self {
            kind: TransactionItemKind::Lightning,
            amount: 100,
            txid: None,
            direction: TransactionDirection::Incoming,
            timestamp: 0,
        }
    }

    pub fn make_dummy_onchain() -> Self {
        Self {
            kind: TransactionItemKind::Onchain,
            amount: 100,
            txid: Some(Txid::all_zeros()),
            direction: TransactionDirection::Outgoing,
            timestamp: 0,
        }
    }
}
