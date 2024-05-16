use iced::widget::{column, container, scrollable, text};
use iced::{Color, Element};
use iced::{Length, Padding};

use crate::components::{h_button, h_federation_item, h_header, h_input, SvgIcon};
use crate::{HarborWallet, Message};

pub fn mints(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Mints", "Manage your mints here.");

    let mint_input = h_input(
        "Mint Invite Code",
        "",
        &harbor.mint_invite_code_str,
        Message::MintInviteCodeInputChanged,
        None,
        false,
        None,
        None,
    );

    let add_mint_button = h_button("Add Mint", SvgIcon::Plus, false)
        .on_press(Message::AddFederation(harbor.mint_invite_code_str.clone()));

    let peek_mint_button = h_button("Peek Mint", SvgIcon::Squirrel, false)
        .on_press(Message::PeekFederation(harbor.mint_invite_code_str.clone()));

    let column = column![header, mint_input, peek_mint_button, add_mint_button].spacing(48);

    // TODO: better error styling
    let column = column.push_maybe(
        harbor
            .add_federation_failure_reason
            .as_ref()
            .map(|r| text(r).size(18).color(Color::from_rgb8(255, 0, 0))),
    );

    let column = column.push_maybe(
        harbor
            .peek_federation_item
            .as_ref()
            .map(|item| h_federation_item(item)),
    );

    let column = if harbor.federation_list.is_empty() {
        column.push(text("No federations added yet.").size(18))
    } else {
        let federation_list = harbor
            .federation_list
            .iter()
            .fold(column![], |column, item| {
                column.push(h_federation_item(item))
            });

        column.push(federation_list)
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
