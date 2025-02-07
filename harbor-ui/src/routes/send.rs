use iced::widget::column;
use iced::Element;

use crate::components::{
    basic_layout, h_button, h_checkbox, h_header, h_input, h_screen_header,
    operation_status_for_id, InputArgs, SvgIcon,
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
        disabled: harbor.is_max,
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

    let checkbox = h_checkbox("Send Max", None, harbor.is_max, Message::SetIsMax);

    let mut button_and_status = column![send_button];

    // Add status display with 16px spacing
    if let Some(status) = harbor
        .current_send_id
        .and_then(|id| operation_status_for_id(harbor, Some(id)))
    {
        button_and_status = button_and_status.push(status).spacing(16);
    }

    let content = column![
        header,
        amount_input,
        checkbox,
        dest_input,
        button_and_status
    ]
    .spacing(48);

    column![h_screen_header(harbor, true), basic_layout(content),].into()
}
