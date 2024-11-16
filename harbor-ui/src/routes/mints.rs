use iced::widget::{column, text};
use iced::Element;

use crate::components::{basic_layout, h_button, h_federation_item, h_header, h_input, SvgIcon};
use crate::{AddFederationStatus, HarborWallet, Message, PeekStatus};

use super::{MintSubroute, Route};

fn mints_list(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Mints", "Manage your mints here.");

    let list = if harbor.federation_list.is_empty() {
        column![text("No federations added yet.").size(18)]
    } else {
        let active_federation = harbor.active_federation.as_ref().expect("No active federation");

        harbor
            .federation_list
            .iter()
            .fold(column![], |column, item| {
                column.push(h_federation_item(item, item.id != active_federation.id))
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

    let column = match &harbor.peek_federation_item {
        None => {
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

            let peek_mint_button = h_button("Preview", SvgIcon::Eye, harbor.peek_status == PeekStatus::Peeking)
                .on_press(Message::PeekFederation(harbor.mint_invite_code_str.clone()));

            column![header, mint_input, peek_mint_button].spacing(48)
        }

        Some(peek_federation_item) => {
            let federation_preview = h_federation_item(peek_federation_item, false);

            let add_mint_button = h_button(
                "Add Mint",
                SvgIcon::Plus,
                harbor.add_federation_status == AddFederationStatus::Adding
            )
            .on_press(Message::AddFederation(harbor.mint_invite_code_str.clone()));

            let start_over_button = h_button("Start Over", SvgIcon::Restart, false)
                .on_press(Message::CancelAddFederation);

            column![
                header,
                federation_preview,
                add_mint_button,
                start_over_button
            ]
            .spacing(48)
        }
    };

    basic_layout(column)
}

pub fn mints(harbor: &HarborWallet) -> Element<Message> {
    match harbor.active_route {
        Route::Mints(MintSubroute::Add) => mints_add(harbor),
        _ => mints_list(harbor),
    }
}
