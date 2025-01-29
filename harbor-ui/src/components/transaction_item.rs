use crate::Message;
use bitcoin::Network;
use harbor_client::db_models::transaction_item::{
    TransactionDirection, TransactionItem, TransactionItemKind,
};
use iced::{
    widget::{button, column, row, svg, text, Button},
    Border, Color, Element,
};

use super::{
    darken, format_amount, format_timestamp, lighten, link, map_icon, text_link, MUTINY_GREEN,
    MUTINY_RED,
};

pub fn h_transaction_item(item: &TransactionItem, is_selected: bool) -> Element<Message> {
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

    Button::new(col)
        .on_press(Message::SelectTransaction(Some(*item)))
        .style(move |theme, status| {
            let background = if is_selected {
                lighten(theme.palette().background, 0.05)
            } else {
                match status {
                    button::Status::Hovered => lighten(theme.palette().background, 0.1),
                    button::Status::Pressed => darken(Color::BLACK, 0.1),
                    _ => theme.palette().background,
                }
            };

            button::Style {
                background: Some(background.into()),
                text_color: Color::WHITE,
                border: Border {
                    color: Color::WHITE,
                    width: 0.,
                    radius: (8.).into(),
                },
                shadow: iced::Shadow::default(),
            }
        })
        .width(iced::Length::Fill)
        .padding(8)
        .into()
}
