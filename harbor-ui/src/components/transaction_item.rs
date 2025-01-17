use crate::Message;
use bitcoin::Network;
use harbor_client::db_models::transaction_item::{
    TransactionDirection, TransactionItem, TransactionItemKind,
};
use iced::{
    widget::{column, row, svg, text},
    Element,
};

use super::{format_amount, format_timestamp, link, map_icon, text_link, MUTINY_GREEN, MUTINY_RED};

pub fn h_transaction_item(item: &TransactionItem) -> Element<Message> {
    let TransactionItem {
        kind,
        amount,
        direction,
        timestamp,
        txid,
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
        .align_y(iced::Alignment::Center)
        .spacing(16);

    // todo: where do we get the network from?
    let network = Network::Signet;
    let base_url = match network {
        Network::Signet => "https://mutinynet.com/tx/",
        _ => panic!("Unsupported network"),
    };

    let col = if let Some(txid) = txid {
        let url = format!("{}{}", base_url, txid);
        let timestamp_text = format_timestamp(timestamp);
        let timestamp = text_link(timestamp_text, url);
        column![row, timestamp].spacing(8)
    } else {
        let timestamp_text = format_timestamp(timestamp);
        let timestamp = text(timestamp_text).color(link());
        column![row, timestamp].spacing(8)
    };

    col.into()
}
