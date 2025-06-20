use super::{
    MUTINY_GREEN, MUTINY_RED, darken, format_amount, format_timestamp, lighten, link, map_icon,
};
use crate::Message;
use harbor_client::db_models::PaymentStatus;
use harbor_client::db_models::transaction_item::{
    TransactionDirection, TransactionItem, TransactionItemKind,
};
use iced::{
    Border, Color, Element,
    widget::{Button, button, column, row, svg, text},
};

pub fn h_transaction_item(item: &TransactionItem, is_selected: bool) -> Element<Message> {
    let TransactionItem {
        kind,
        amount,
        fee_msats: _,
        direction,
        timestamp,
        mint_identifier: _,
        status,
        txid: _,
        preimage: _,
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

    let amount_str = if matches!(kind, TransactionItemKind::Onchain)
        && matches!(direction, TransactionDirection::Incoming)
        && matches!(status, PaymentStatus::WaitingConfirmation)
    {
        format!("{} (Pending)", format_amount(*amount))
    } else {
        format_amount(*amount)
    };

    let formatted_amount = text(amount_str).size(24);

    let row = row![direction_icon, kind_icon, formatted_amount,]
        .align_y(iced::Alignment::Center)
        .spacing(16);

    let timestamp_text = format_timestamp(timestamp);
    let timestamp = text(timestamp_text).color(link());
    let col = column![row, timestamp].spacing(8);

    Button::new(col)
        .on_press(Message::SelectTransaction(Some(item.clone())))
        .style(move |theme, status| {
            let background = if is_selected {
                lighten(theme.palette().background, 0.05)
            } else {
                match status {
                    button::Status::Hovered => lighten(theme.palette().background, 0.1),
                    button::Status::Pressed => darken(theme.palette().background, 0.05),
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
        .padding([8, 16])
        .into()
}
