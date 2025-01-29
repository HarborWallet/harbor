use crate::components::{text_link, SvgIcon};
use crate::Message;
use bitcoin::Network;
use harbor_client::db_models::transaction_item::{
    TransactionDirection, TransactionItem, TransactionItemKind,
};
use iced::widget::{button, column, container, row, text};
use iced::{Border, Color, Element, Length, Theme};

use super::{darken, format_amount, format_timestamp, h_icon_button, light_container_style, lighten};

pub fn h_transaction_details(item: &TransactionItem) -> Element<Message> {
    let TransactionItem {
        kind,
        amount,
        direction,
        timestamp,
        txid,
    } = item;

    let kind_text = match kind {
        TransactionItemKind::Lightning => "Lightning",
        TransactionItemKind::Onchain => "On-chain",
    };

    let direction_text = match direction {
        TransactionDirection::Incoming => "Received",
        TransactionDirection::Outgoing => "Sent",
    };

    let formatted_amount = format_amount(*amount);
    let formatted_timestamp = format_timestamp(timestamp);

    let mut details = column![
        text(format!("Type: {}", kind_text)).size(16),
        text(format!("Direction: {}", direction_text)).size(16),
        text(format!("Amount: {}", formatted_amount)).size(16),
        text(format!("Time: {}", formatted_timestamp)).size(16),
    ]
    .spacing(8);

    if let Some(txid) = txid {
        // todo: where do we get the network from?
        let network = Network::Signet;
        let base_url = match network {
            Network::Signet => "https://mutinynet.com/tx/",
            _ => panic!("Unsupported network"),
        };
        let url = format!("{}{}", base_url, txid);
        details = details.push(
            row![
                text("Transaction ID: ").size(16),
                text_link(txid.to_string(), url)
            ]
            .spacing(4),
        );
    }

    let close_button = h_icon_button(SvgIcon::ChevronRight).on_press(Message::SelectTransaction(None));

    container(row![
        container(close_button).padding(8),
        column![text("Transaction Details").size(24), details]
            .padding(8),
    ])
    .style(light_container_style)
    .width(Length::Fixed(300.))
    .height(Length::Shrink)
    .padding(8)
    .into()
}
