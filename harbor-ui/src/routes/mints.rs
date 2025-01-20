use iced::widget::column;
use iced::Element;

use crate::components::{
    basic_layout, h_button, h_federation_item, h_header, h_input, InputArgs, SvgIcon,
};
use crate::{AddFederationStatus, HarborWallet, Message, PeekStatus};

use super::{MintSubroute, Route};

// Expects to always have at least one federation, otherwise we should be on the add mint screen
fn mints_list(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Mints", "Manage your mints here.");

    let active_federation = harbor
        .active_federation
        .as_ref()
        .expect("No active federation");

    let list = harbor
        .federation_list
        .iter()
        .fold(column![], |column, item| {
            column.push(h_federation_item(
                item,
                item.id != active_federation.id,
                true,
            ))
        })
        .spacing(48);

    let add_mint_button = h_button("Add Mint", SvgIcon::Plus, false)
        .on_press(Message::Navigate(Route::Mints(MintSubroute::Add)));

    let column = column![header, list, add_mint_button].spacing(48);

    basic_layout(column)
}

fn mints_add(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Add Mint", "Add a new mint to your wallet.");

    let column = match &harbor.peek_federation_item {
        None => {
            let mint_input = h_input(InputArgs {
                label: "Mint Invite Code",
                value: &harbor.mint_invite_code_str,
                on_input: Message::MintInviteCodeInputChanged,
                disabled: harbor.peek_status == PeekStatus::Peeking,
                ..InputArgs::default()
            });

            let peek_mint_button = h_button(
                "Preview",
                SvgIcon::Eye,
                harbor.peek_status == PeekStatus::Peeking,
            )
            .on_press(Message::PeekFederation(harbor.mint_invite_code_str.clone()));

            column![header, mint_input, peek_mint_button].spacing(48)
        }

        Some(peek_federation_item) => {
            let federation_preview = h_federation_item(peek_federation_item, false, false);

            let add_mint_button = h_button(
                "Add Mint",
                SvgIcon::Plus,
                harbor.add_federation_status == AddFederationStatus::Adding,
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
    if harbor.federation_list.is_empty() {
        mints_add(harbor)
    } else {
        match harbor.active_route {
            Route::Mints(MintSubroute::Add) => mints_add(harbor),
            _ => mints_list(harbor),
        }
    }
}
