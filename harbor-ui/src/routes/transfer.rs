use iced::widget::{PickList, column, container, pick_list, row, scrollable, text};
use iced::{Alignment, Element, Length, Padding};

use crate::components::{
    ConfirmModalState, InputArgs, SvgIcon, h_balance_display, h_button, h_header, h_input,
    menu_style, operation_status_for_id, pick_list_style,
};
use crate::{HarborWallet, Message, SendStatus};

pub fn transfer(harbor: &HarborWallet) -> Element<Message> {
    // We have to have at least 2 federations to be on this screen!
    assert!(harbor.mint_list.len() >= 2);
    let federation_names: Vec<&str> = harbor
        .mint_list
        .iter()
        .filter(|f| f.active)
        .map(|f| f.name.as_str())
        .collect();

    let source_list: PickList<'_, &str, Vec<&str>, &str, Message> = pick_list(
        federation_names.clone(),
        harbor.transfer_from_federation_selection.as_deref(),
        |s| Message::SetTransferFrom(s.to_string()),
    )
    .placeholder("Pick a source mint")
    .style(pick_list_style)
    .padding(Padding::from(16))
    .handle(pick_list::Handle::Arrow {
        size: Some(iced::Pixels(24.)),
    })
    .menu_style(menu_style);

    let source = column![text("Source").size(24), source_list].spacing(16);

    let mut source_row = row![source].spacing(16).align_y(Alignment::End);

    // Show balance for source federation if selected
    if let Some(source_fed) = harbor.transfer_from_federation_selection.as_ref() {
        if let Some(federation) = harbor.mint_list.iter().find(|f| f.name == *source_fed) {
            source_row = source_row.push(h_balance_display(federation.balance));
        }
    }

    let destination_list: PickList<'_, &str, Vec<&str>, &str, Message> = pick_list(
        federation_names,
        harbor.transfer_to_federation_selection.as_deref(),
        |s| Message::SetTransferTo(s.to_string()),
    )
    .placeholder("Pick a destination mint")
    .style(pick_list_style)
    .padding(Padding::from(16))
    .handle(pick_list::Handle::Arrow {
        size: Some(iced::Pixels(24.)),
    })
    .menu_style(menu_style);

    let destination = column![text("Destination").size(24), destination_list].spacing(16);

    let mut destination_row = row![destination].spacing(16).align_y(Alignment::End);

    // Show balance for destination federation if selected
    if let Some(dest_fed) = harbor.transfer_to_federation_selection.as_ref() {
        if let Some(federation) = harbor.mint_list.iter().find(|f| f.name == *dest_fed) {
            destination_row = destination_row.push(h_balance_display(federation.balance));
        }
    }

    let amount_input = h_input(InputArgs {
        label: "Amount",
        placeholder: "420",
        value: &harbor.transfer_amount_input_str,
        on_input: Message::TransferAmountInputChanged,
        numeric: true,
        suffix: Some("sats"),
        ..InputArgs::default()
    });

    let transfer_button = h_button(
        "Transfer",
        SvgIcon::LeftRight,
        harbor.transfer_status == SendStatus::Sending,
    )
    .on_press(Message::Transfer);

    let mut button_and_status = if harbor.transfer_status == SendStatus::Sending {
        // When transferring, include a "Start Over" next to the transfer button
        let start_over_button = h_button("Start Over", SvgIcon::Restart, false)
            .on_press(Message::SetConfirmModal(Some(ConfirmModalState {
                title: "Are you sure?".to_string(),
                description: "We'll attempt to cancel this transfer, but since it's begun it's possible for it to still go through.".to_string(),
                confirm_action: Box::new(Message::TransferStateReset),
                cancel_action: Box::new(Message::SetConfirmModal(None)),
                confirm_button_text: "Start Over".to_string(),
            })));
        column![row![start_over_button, transfer_button].spacing(8)]
    } else {
        column![transfer_button]
    };

    // Add status display with 16px spacing
    if let Some(status) = harbor
        .current_transfer_id
        .and_then(|id| operation_status_for_id(harbor, Some(id)))
    {
        button_and_status = button_and_status.push(status).spacing(16);
    }

    let list = column![source_row, destination_row, amount_input, button_and_status].spacing(48);

    container(scrollable(
        column![h_header("Transfer", "Rebalance your funds."), list]
            .spacing(48)
            .width(Length::Fill)
            .max_width(512)
            .padding(Padding::new(48.)),
    ))
    .into()
}
