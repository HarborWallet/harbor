use iced::widget::text::Style;
use iced::widget::{column, container, scrollable, text};
use iced::{Color, Length};
use iced::{Element, Padding, Theme};

use crate::components::{h_button, h_input, lighten, SvgIcon};
use crate::{HarborWallet, Message};

pub fn send(harbor: &HarborWallet) -> Element<Message> {
    let header = column![
        text("Send").size(32),
        text("Send to an on-chain address or lighting invoice.")
            .size(18)
            .style(|theme: &Theme| {
                let gray = lighten(theme.palette().background, 0.5);
                Style { color: Some(gray) }
            })
    ]
    .spacing(8);

    let amount_input = h_input(
        "Amount",
        "420",
        &harbor.send_amount_input_str,
        Message::SendAmountInputChanged,
        None,
        false,
        None,
        Some("sats"),
    );

    let dest_input = h_input(
        "Destination",
        "abc123...",
        &harbor.send_dest_input_str,
        Message::SendDestInputChanged,
        None,
        false,
        None,
        None,
    );

    let send_button = h_button("Send", SvgIcon::UpRight, false)
        .on_press(Message::Send(harbor.send_dest_input_str.clone()));

    let body = column![header, amount_input, dest_input, send_button].spacing(48);

    let failure_message = harbor
        .send_failure_reason
        .as_ref()
        .map(|r| text(r).size(50).color(Color::from_rgb(255., 0., 0.)));

    let column = if let Some(failure_message) = failure_message {
        column![body, failure_message]
    } else {
        column![body]
    };

    container(scrollable(
        column
            .spacing(48)
            .width(Length::Fill)
            .max_width(512)
            .padding(Padding::new(48.)),
    ))
    .into()
}
