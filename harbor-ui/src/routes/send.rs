use iced::widget::{column, text, Checkbox};
use iced::Color;
use iced::Element;

use crate::components::{
    basic_layout, h_button, h_header, h_input, h_screen_header, InputArgs, SvgIcon,
};
use crate::{HarborWallet, Message, SendStatus};

pub fn send(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Send", "Send to an on-chain address or lightning invoice.");

    // todo disable amount input if max is selected
    let amount_input = h_input(InputArgs {
        label: "Amount",
        placeholder: "420",
        value: &harbor.send_amount_input_str,
        on_input: Message::SendAmountInputChanged,
        numeric: true,
        suffix: Some("sats"),
        ..InputArgs::default()
    });

    let dest_input = h_input(InputArgs {
        label: "Destination",
        placeholder: "abc123...",
        value: &harbor.send_dest_input_str,
        on_input: Message::SendDestInputChanged,
        ..InputArgs::default()
    });

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

    let checkbox = Checkbox::new("Send Max", harbor.is_max).on_toggle(Message::SetIsMax);

    let column = if let Some(failure_message) = failure_message {
        let dangit_button =
            h_button("Dangit", SvgIcon::Squirrel, false).on_press(Message::SendStateReset);
        column![header, failure_message, dangit_button]
    } else if let Some(success_message) = success_message {
        let nice_button = h_button("Nice", SvgIcon::Heart, false).on_press(Message::SendStateReset);
        column![header, success_message, nice_button]
    } else {
        column![header, amount_input, checkbox, dest_input, send_button]
    };

    column![
        h_screen_header(harbor, true),
        basic_layout(column.spacing(48)),
    ]
    .into()
}
