use crate::Message;
use harbor_client::db_models::FederationItem;
use iced::{
    widget::{column, container, row, text},
    Alignment, Element,
};

use super::{bold_text, h_button, light_container_style, regular_text, subtitle, truncate_text};
use super::{format_amount, map_icon, SvgIcon};

pub fn h_federation_item(
    item: &FederationItem,
    show_button: bool,
    is_added: bool,
) -> Element<Message> {
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

    let balance_row = text(format_amount(*balance)).size(24);
    let balance_subtitle = text("Your balance").size(18).style(subtitle);
    let balance_col = column![balance_row, balance_subtitle].spacing(8);

    column = column.push(balance_col);

    if show_button {
        let button = h_button("Set as active Mint", SvgIcon::Squirrel, false)
            .on_press(Message::ChangeFederation(*id));
        column = column.push(button);
    }

    if is_added {
        let remove_button =
            h_button("Remove Mint", SvgIcon::Trash, false).on_press(Message::RemoveFederation(*id));
        column = column.push(remove_button);
    }

    container(column).padding(16).style(light_container_style).into()
}
