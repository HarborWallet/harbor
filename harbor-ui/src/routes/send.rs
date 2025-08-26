use iced::Element;
use iced::widget::{column, row};

use crate::components::{
    ConfirmModalState, InputArgs, SvgIcon, basic_layout, h_button, h_checkbox, h_header, h_input,
    h_screen_header, operation_status_for_id,
};
use crate::{HarborWallet, Message, SendStatus};

pub fn send(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header(
        "Send",
        "Send to an on-chain address, lightning invoice, or Bolt12 offer.",
    );

    let dest_input = h_input(InputArgs {
        label: "Destination",
        placeholder: "lnbc1... or lno... or bc1...",
        value: &harbor.send_dest_input_str,
        on_input: Message::SendDestInputChanged,
        ..InputArgs::default()
    });

    let amount_input = h_input(InputArgs {
        label: "Amount",
        placeholder: "420",
        value: &harbor.send_amount_input_str,
        on_input: Message::SendAmountInputChanged,
        numeric: true,
        suffix: Some("sats"),
        disabled: harbor.is_max || harbor.input_has_amount,
        ..InputArgs::default()
    });

    let send_button = h_button(
        "Send",
        SvgIcon::UpRight,
        harbor.send_status == SendStatus::Sending,
    )
    .on_press(Message::Send(harbor.send_dest_input_str.clone()));

    let checkbox = h_checkbox(
        "Send Max",
        None,
        harbor.is_max,
        harbor.input_has_amount,
        Message::SetIsMax,
    );

    let mut button_and_status = if harbor.send_status == SendStatus::Sending {
        // When sending, include a "Start Over" next to the send button
        let start_over_button = h_button("Start Over", SvgIcon::Restart, false)
            .on_press(Message::SetConfirmModal(Some(ConfirmModalState {
                title: "Are you sure?".to_string(),
                description: "We'll attempt to cancel this payment, but since it's begun it's possible for it to still go through.".to_string(),
                confirm_action: Box::new(Message::SendStateReset),
                cancel_action: Box::new(Message::SetConfirmModal(None)),
                confirm_button_text: "Start Over".to_string(),
            })));
        column![row![start_over_button, send_button].spacing(8)]
    } else {
        column![send_button]
    };

    // Add status display with 16px spacing
    if let Some(status) = harbor
        .current_send_id
        .and_then(|id| operation_status_for_id(harbor, Some(id)))
    {
        button_and_status = button_and_status.push(status).spacing(16);
    }

    let content = column![
        header,
        dest_input,
        amount_input,
        checkbox,
        button_and_status
    ]
    .spacing(48);

    column![h_screen_header(harbor, true, false), basic_layout(content)].into()
}
