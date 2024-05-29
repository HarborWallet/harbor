use iced::{
    widget::{column, row, svg, text},
    Element,
};

use crate::Message;

use super::{format_amount, format_timestamp, map_icon, subtitle, MUTINY_GREEN, MUTINY_RED};

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
    pub direction: TransactionDirection,
    pub timestamp: u64,
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
    let kind_icon = match kind {
        TransactionItemKind::Lightning => map_icon(super::SvgIcon::Bolt, 24., 24.),
        TransactionItemKind::Onchain => map_icon(super::SvgIcon::Chain, 24., 24.),
    };

    let direction_icon = match direction {
        TransactionDirection::Incoming => {
            map_icon(super::SvgIcon::DownLeft, 24., 24.).style(|_theme, _status| svg::Style {
                color: Some(MUTINY_GREEN),
            })
        }
        TransactionDirection::Outgoing => {
            map_icon(super::SvgIcon::UpRight, 24., 24.).style(|_theme, _status| svg::Style {
                color: Some(MUTINY_RED),
            })
        }
    };

    let formatted_amount = text(format_amount(*amount)).size(24);

    let row = row![direction_icon, kind_icon, formatted_amount,]
        .align_items(iced::Alignment::Center)
        .spacing(16);

    let timestamp = text(format_timestamp(timestamp)).size(18).style(subtitle);

    let col = column![row, timestamp].spacing(8);

    col.into()
}
