use crate::components::{format_amount, h_button, h_screen_header, SvgIcon};
use iced::widget::{center, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::{HarborWallet, Message};

use super::Route;

pub fn home(harbor: &HarborWallet) -> Element<Message> {
    let formatted_balance = format_amount(harbor.balance_sats);
    let balance = text(formatted_balance).size(64);
    let send_button =
        h_button("Send", SvgIcon::UpRight, false).on_press(Message::Navigate(Route::Send));
    let receive_button =
        h_button("Deposit", SvgIcon::DownLeft, false).on_press(Message::Navigate(Route::Receive));
    let buttons = row![receive_button, send_button].spacing(32);

    column![
        h_screen_header(harbor, false),
        container(center(
            column![balance, buttons]
                .spacing(32)
                .align_items(Alignment::Center)
                .max_width(512)
        ))
        .height(Length::Fill)
    ]
    .into()
}
