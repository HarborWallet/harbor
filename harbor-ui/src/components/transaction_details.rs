use super::{format_amount, format_timestamp, side_panel_style, subtitle};
use crate::Message;
use crate::components::{SvgIcon, map_icon, text_link};
use harbor_client::MintIdentifier;
use harbor_client::bitcoin::Network;
use harbor_client::bitcoin::hex::DisplayHex;
use harbor_client::db_models::MintItem;
use harbor_client::db_models::transaction_item::{
    TransactionDirection, TransactionItem, TransactionItemKind,
};
use harbor_client::fedimint_core::config::FederationId;
use iced::widget::{column, container, rich_text, row, span, text, vertical_space};
use iced::{Alignment, Element, Length};

pub fn h_transaction_details<'a>(
    item: &'a TransactionItem,
    federation_list: &'a [MintItem],
    network: Network,
) -> Element<'a, Message> {
    let TransactionItem {
        kind,
        amount,
        direction,
        mint_identifier,
        timestamp,
        status: _,
        txid,
        preimage,
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
        .find(|f| &f.id == mint_identifier)
        .cloned()
        .unwrap_or(MintItem::unknown(
            mint_identifier
                .federation_id()
                .unwrap_or(FederationId::dummy()),
        ));

    // Choose the right icon based on the mint type
    let mint_icon = match mint_identifier {
        MintIdentifier::Cashu(_) => map_icon(SvgIcon::Squirrel, 16., 16.),
        MintIdentifier::Fedimint(_) => map_icon(SvgIcon::People, 16., 16.),
    };

    let mint_section = column![
        text(mint_label).size(16).style(subtitle),
        row![mint_icon, text(mint.name.clone()).size(16)]
            .align_y(Alignment::Center)
            .spacing(8)
    ]
    .spacing(8);

    // Create the amount section
    let amount_section = column![
        text("Amount").size(16).style(subtitle),
        text(formatted_amount).size(16)
    ]
    .spacing(8);

    // Create the time section
    let time_section = column![
        text("Time").size(16).style(subtitle),
        text(formatted_timestamp).size(16)
    ]
    .spacing(8);

    let mut details = column![mint_section, amount_section, time_section].spacing(16);

    // Add TXID if it exists
    if let Some(txid) = txid {
        let base_url = match network {
            Network::Bitcoin => "https://mempool.space/tx/",
            Network::Testnet => "https://mempool.space/testnet3/tx/",
            Network::Testnet4 => "https://mempool.space/testnet4/tx/",
            Network::Signet => "https://mutinynet.com/tx/",
            _ => panic!("Unsupported network"),
        };
        let url = format!("{}{}", base_url, txid);
        details = details.push(
            column![
                text("TXID").size(16).style(subtitle),
                text_link(txid.to_string(), url)
            ]
            .spacing(8),
        );
    }

    // Add preimage if it exists
    if let Some(preimage) = preimage {
        let hex = preimage.to_lower_hex_string();
        // Take first 5 chars, add "...", and append last 5 chars
        let first_five = &hex[..5];
        let last_five = &hex[hex.len() - 5..];
        details = details.push(
            column![
                text("Preimage").size(16).style(subtitle),
                row![
                    rich_text([span(format!("{first_five}...{last_five}")).link(hex)])
                        .on_link_click(move |a: String| Message::CopyToClipboard(a))
                ]
            ]
            .spacing(8),
        );
    }

    let title_row = row![text(title).size(24),].align_y(Alignment::Center);

    container(
        column![
            vertical_space().height(Length::Fixed(16.)),
            title_row,
            details
        ]
        .spacing(16)
        .padding(16),
    )
    .style(side_panel_style)
    .width(Length::Fixed(300.))
    .height(Length::Fill)
    .into()
}
