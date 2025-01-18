use crate::components::{format_amount, h_button, h_screen_header, SvgIcon};
use iced::widget::{center, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::{HarborWallet, Message};

use super::Route;

pub fn home(harbor: &HarborWallet) -> Element<Message> {
    let formatted_balance = if let Some(federation) = &harbor.active_federation {
        format_amount(federation.balance)
    } else {
        format_amount(0)
    };

    let balance = text(formatted_balance).size(64);
    let send_disabled = harbor
        .active_federation
        .as_ref()
        .map_or(true, |f| f.balance == 0);
    let receive_disabled = harbor.active_federation.is_none();
    let send_button = h_button("Send", SvgIcon::UpRight, false);
    let receive_button = h_button("Deposit", SvgIcon::DownLeft, false);

    let buttons = row![
        if !send_disabled {
            send_button.on_press(Message::Navigate(Route::Send))
        } else {
            send_button
        },
        if !receive_disabled {
            receive_button.on_press(Message::Navigate(Route::Receive))
        } else {
            receive_button
        }
    ]
    .spacing(32);

    column![
        h_screen_header(harbor, false),
        container(center(
            column![balance, buttons]
                .spacing(32)
                .align_x(Alignment::Center)
                .max_width(512)
        ))
        .height(Length::Fill)
    ]
    .into()
}
