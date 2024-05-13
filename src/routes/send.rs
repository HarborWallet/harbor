use iced::widget::{column, container, scrollable, text, text_input};
use iced::{Alignment, Element};
use iced::{Color, Length};

use crate::components::{h_button, SvgIcon};
use crate::{HarborWallet, Message};

pub fn send(harbor: &HarborWallet) -> Element<Message> {
    let send_input = column![
        "What's your destination?",
        text_input("invoice", &harbor.send_input_str).on_input(Message::SendInputChanged),
        h_button("Send", SvgIcon::DownLeft).on_press(Message::Send(harbor.send_input_str.clone())),
    ];

    let failure_message = harbor
        .send_failure_reason
        .as_ref()
        .map(|r| text(r).size(50).color(Color::from_rgb(255., 0., 0.)));

    let column = if let Some(failure_message) = failure_message {
        column![send_input, failure_message]
    } else {
        column![send_input]
    };

    container(
        scrollable(
            column
                .spacing(32)
                .align_items(Alignment::Center)
                .width(Length::Fill),
        )
        .height(Length::Fill),
    )
    .into()
}
