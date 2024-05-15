use iced::widget::{column, container, scrollable, text};
use iced::{Color, Length};
use iced::{Element, Padding};

use crate::components::{h_button, h_header, h_input, SvgIcon};
use crate::{HarborWallet, Message, SendStatus};

pub fn send(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Send", "Send to an on-chain address or lightning invoice.");

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

    let send_button = h_button(
        "Send",
        SvgIcon::UpRight,
        harbor.send_status == SendStatus::Sending,
    )
    .on_press(Message::Send(harbor.send_dest_input_str.clone()));

    let failure_message = harbor
        .send_failure_reason
        .as_ref()
        .map(|r| text(r).size(32).color(Color::from_rgb(255., 0., 0.)));

    let success_message = harbor.send_success_msg.as_ref().map(|r| {
        text(format!("Success: {r:?}"))
            .size(32)
            .color(Color::from_rgb(0., 255., 0.))
    });

    let column = if let Some(failure_message) = failure_message {
        let dangit_button =
            h_button("Dangit", SvgIcon::Squirrel, false).on_press(Message::SendStateReset);
        column![header, failure_message, dangit_button]
    } else if let Some(success_message) = success_message {
        let nice_button = h_button("Nice", SvgIcon::Heart, false).on_press(Message::SendStateReset);
        column![header, success_message, nice_button]
    } else {
        column![header, amount_input, dest_input, send_button]
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
