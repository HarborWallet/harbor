use iced::{
    widget::{row, text},
    Element,
};

use crate::Message;

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
    kind: TransactionItemKind,
    amount: u64,
    direction: TransactionDirection,
    timestamp: u64,
}

impl TransactionItem {
    pub fn make_dummy() -> Self {
        Self {
            kind: TransactionItemKind::Lightning,
            amount: 100,
            direction: TransactionDirection::Incoming,
            timestamp: 0,
        }
    }

    pub fn make_dummy_onchain() -> Self {
        Self {
            kind: TransactionItemKind::Onchain,
            amount: 100,
            direction: TransactionDirection::Outgoing,
            timestamp: 0,
        }
    }
}

pub fn h_transaction_item(item: &TransactionItem) -> Element<Message> {
    let TransactionItem {
        kind,
        amount,
        direction,
        timestamp,
    } = item;
    let row = row![
        text(format!("{kind:?}")).size(24),
        text(format!("{amount} sats")).size(24),
        text(format!("{direction:?}")).size(24),
        text(format!("{timestamp}")).size(24),
    ]
    .spacing(16);

    row.into()
}
