use iced::widget::{column, container, pick_list, scrollable, text, PickList};
use iced::{Element, Length, Padding};

use crate::components::{
    h_button, h_header, h_input, menu_style, pick_list_style, InputArgs, SvgIcon,
};
use crate::{HarborWallet, Message};

pub fn transfer(harbor: &HarborWallet) -> Element<Message> {
    let federation_names: Vec<&str> = harbor
        .federation_list
        .iter()
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

    let amount_input = h_input(InputArgs {
        label: "Amount",
        placeholder: "420",
        value: &harbor.transfer_amount_input_str,
        on_input: Message::TransferAmountInputChanged,
        numeric: true,
        suffix: Some("sats"),
        ..InputArgs::default()
    });

    // TODO: atually transfer
    let transfer_button = h_button("Transfer", SvgIcon::LeftRight, false).on_press(Message::Noop);

    let list = column![source, destination, amount_input, transfer_button].spacing(48);

    container(scrollable(
        column![h_header("Transfer", "Rebalance your funds."), list]
            .spacing(48)
            .width(Length::Fill)
            .max_width(512)
            .padding(Padding::new(48.)),
    ))
    .into()
}
