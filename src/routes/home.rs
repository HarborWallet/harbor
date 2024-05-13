use crate::components::{h_button, SvgIcon};
use iced::widget::{center, column, container, row, text};
use iced::Color;
use iced::{Alignment, Element};

use crate::{HarborWallet, Message};

pub fn home(harbor: &HarborWallet) -> Element<Message> {
    let balance = text(format!("{} sats", harbor.balance.sats_round_down())).size(64);
    let send_button = h_button("Send", SvgIcon::UpRight).on_press(Message::Send(100));
    let receive_button = h_button("Receive", SvgIcon::DownLeft).on_press(Message::Receive(100));
    let buttons = row![send_button, receive_button].spacing(32);

    let failure_message = harbor
        .send_failure_reason
        .as_ref()
        .map(|r| text(r).size(50).color(Color::from_rgb(255., 0., 0.)));

    let column = if let Some(failure_message) = failure_message {
        column![balance, failure_message, buttons]
    } else {
        column![balance, buttons]
    };
    container(center(column.spacing(32).align_items(Alignment::Center))).into()
}
