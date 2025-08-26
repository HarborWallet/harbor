use crate::components::{SvgIcon, format_amount, h_button, h_screen_header};
use iced::widget::{center, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::{HarborWallet, Message};

use super::Route;

pub fn home(harbor: &HarborWallet) -> Element<Message> {
    let formatted_balance = harbor
        .active_federation()
        .map_or_else(|| format_amount(0), |f| format_amount(f.balance));

    let balance = text(formatted_balance).size(64);
    let send_disabled = harbor.active_federation().is_none_or(|f| f.balance == 0);
    let receive_disabled = harbor.active_federation().is_none();
    let send_button = h_button("Send", SvgIcon::UpRight, false);
    let receive_button = h_button("Deposit", SvgIcon::DownLeft, false);

    let buttons = row![
        if send_disabled {
            send_button
        } else {
            send_button.on_press(Message::Navigate(Route::Send))
        },
        if receive_disabled {
            receive_button
        } else {
            receive_button.on_press(Message::Navigate(Route::Receive))
        }
    ]
    .spacing(32);

    column![
        h_screen_header(harbor, false, false),
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
