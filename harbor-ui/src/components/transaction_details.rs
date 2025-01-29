use super::{format_amount, format_timestamp, light_container_style, subtitle};
use crate::components::{map_icon, text_link, SvgIcon};
use crate::Message;
use bitcoin::Network;
use harbor_client::db_models::transaction_item::{
    TransactionDirection, TransactionItem, TransactionItemKind,
};
use harbor_client::db_models::FederationItem;
use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};

pub fn h_transaction_details<'a>(
    item: &'a TransactionItem,
    federation_list: &'a [FederationItem],
) -> Element<'a, Message> {
    let TransactionItem {
        kind,
        amount,
        direction,
        timestamp,
        federation_id,
        txid,
    } = item;

    // Create title based on type and direction
    let title = match (kind, direction) {
        (TransactionItemKind::Lightning, TransactionDirection::Incoming) => "Lightning Receive",
        (TransactionItemKind::Lightning, TransactionDirection::Outgoing) => "Lightning Send",
        (TransactionItemKind::Onchain, TransactionDirection::Incoming) => "On-chain Receive",
        (TransactionItemKind::Onchain, TransactionDirection::Outgoing) => "On-chain Send",
    };

    let formatted_amount = format_amount(*amount);
    let formatted_timestamp = format_timestamp(timestamp);

    // Create the mint section with appropriate label
    let mint_label = match direction {
        TransactionDirection::Incoming => "To",
        TransactionDirection::Outgoing => "From",
    };

    let mint = federation_list
        .iter()
        .find(|f| f.id == *federation_id)
        .cloned()
        .unwrap_or(FederationItem::unknown(*federation_id));

    let mint_section = column![
        text(mint_label).size(16).style(subtitle),
        row![
            map_icon(SvgIcon::People, 24., 24.),
            text(mint.name.clone()).size(24)
        ]
        .align_y(Alignment::Center)
        .spacing(16)
    ]
    .spacing(8);

    // Create the amount section
    let amount_section = column![
        text("Amount").size(16).style(subtitle),
        text(formatted_amount).size(24)
    ]
    .spacing(8);

    // Create the time section
    let time_section = row![
        text("Time").size(16).style(subtitle),
        text(formatted_timestamp).size(16)
    ]
    .spacing(8);

    let mut details = column![mint_section, amount_section, time_section].spacing(16);

    // TODO: need preimages so we can do lightning too

    // Add TXID if it exists
    if let Some(txid) = txid {
        // TODO: where do we get the network from?
        let network = Network::Signet;
        let base_url = match network {
            Network::Signet => "https://mutinynet.com/tx/",
            _ => panic!("Unsupported network"),
        };
        let url = format!("{}{}", base_url, txid);
        details = details.push(
            row![
                text("TXID").size(16).style(subtitle),
                text_link(txid.to_string(), url)
            ]
            .spacing(8),
        );
    }

    let title_row = row![text(title).size(24),].align_y(Alignment::Center);

    container(column![title_row, details].spacing(16).padding(16))
        .style(light_container_style)
        .width(Length::Fixed(300.))
        .height(Length::Shrink)
        .into()
}
