use crate::Message;
use harbor_client::db_models::FederationItem;
use iced::{
    widget::{column, container, row, text},
    Alignment, Element,
};

use super::{format_amount, map_icon, SvgIcon};
use super::{h_button, light_container_style, subtitle};

pub fn h_federation_item(item: &FederationItem, invite_code: Option<String>) -> Element<Message> {
    let FederationItem {
        id,
        name,
        balance,
        guardians,
        module_kinds: _, // We don't care about module kinds
    } = item;

    let name_row = row![map_icon(SvgIcon::People, 24., 24.), text(name).size(24)]
        .align_y(Alignment::Center)
        .spacing(16);

    let mut column = column![name_row].spacing(32);

    if let Some(guardians) = guardians {
        let count = guardians.len();
        let guardian_text = if count == 1 {
            "1 guardian".to_string()
        } else {
            format!("{} guardians", count)
        };
        column = column.push(text(guardian_text).size(18).style(subtitle));
    }

    match invite_code {
        // Preview mode with Add button
        Some(code) => {
            let add_mint_button =
                h_button("Add Mint", SvgIcon::Plus, false).on_press(Message::AddFederation(code));
            column = column.push(add_mint_button);
        }
        // Normal mode with balance and Remove button
        None => {
            let balance_row = text(format_amount(*balance)).size(24);
            let balance_subtitle = text("Your balance").size(18).style(subtitle);
            let balance_col = column![balance_row, balance_subtitle].spacing(8);
            column = column.push(balance_col);

            let remove_button = h_button("Remove Mint", SvgIcon::Trash, false)
                .on_press(Message::RemoveFederation(*id));
            column = column.push(remove_button);
        }
    }

    container(column)
        .padding(16)
        .style(light_container_style)
        .into()
}
