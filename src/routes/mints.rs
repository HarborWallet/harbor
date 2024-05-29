use iced::widget::{column, text};
use iced::{Color, Element};

use crate::components::{basic_layout, h_button, h_federation_item, h_header, h_input, SvgIcon};
use crate::{HarborWallet, Message};

use super::{MintSubroute, Route};

fn mints_list(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Mints", "Manage your mints here.");

    let list = if harbor.federation_list.is_empty() {
        column![text("No federations added yet.").size(18)]
    } else {
        harbor
            .federation_list
            .iter()
            .fold(column![], |column, item| {
                column.push(h_federation_item(item))
            })
            .spacing(48)
    };

    let add_mint_button = h_button("Add Mint", SvgIcon::Plus, false)
        .on_press(Message::Navigate(Route::Mints(MintSubroute::Add)));

    let column = column![header, list, add_mint_button].spacing(48);

    basic_layout(column)
}

fn mints_add(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Add Mint", "Add a new mint to your wallet.");

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

    // let peek_mint_button = h_button("Peek Mint", SvgIcon::Squirrel, false)
    //     .on_press(Message::PeekFederation(harbor.mint_invite_code_str.clone()));

    let column = column![
        header,
        mint_input,
        // peek_mint_button,
        add_mint_button
    ]
    .spacing(48);

    // TODO: better error styling
    let column = column.push_maybe(
        harbor
            .add_federation_failure_reason
            .as_ref()
            .map(|r| text(r).size(18).color(Color::from_rgb8(255, 0, 0))),
    );

    basic_layout(column.spacing(48))
}

pub fn mints(harbor: &HarborWallet) -> Element<Message> {
    match harbor.active_route {
        Route::Mints(MintSubroute::Add) => mints_add(harbor),
        _ => mints_list(harbor),
    }
}
