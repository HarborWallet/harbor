use iced::widget::{column, text, Checkbox};
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

    let checkbox = Checkbox::new("Send Max", harbor.is_max).on_toggle(Message::SetIsMax);

    // Add status display
    let status_display = harbor
        .current_send_id
        .and_then(|id| harbor.operation_status.get(&id))
        .map(|status| text(&status.message));

    let mut content = column![header, amount_input, checkbox, dest_input, send_button].spacing(48);

    if let Some(status) = status_display {
        content = content.push(status);
    }

    column![h_screen_header(harbor, true), basic_layout(content),].into()
}
