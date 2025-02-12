use crate::Message;
use harbor_client::db_models::FederationItem;
use iced::{
    widget::{column, container, horizontal_space, row, text},
    Alignment, Element, Length,
};

use super::{
    h_balance_display, h_small_button, light_container_style, map_icon, subtitle, tag_style,
    ConfirmModalState, SvgIcon,
};

pub fn h_federation_item_preview(item: &FederationItem) -> Element<Message> {
    let FederationItem {
        id: _,
        name,
        balance: _,
        guardians,
        module_kinds: _, // We don't care about module kinds
        metadata,
        active: _,
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

    if let Some(welcome) = metadata.welcome_message.as_ref() {
        column = column.push(text(welcome).size(18).style(subtitle));
    }

    let preview_tag = container(text("Preview").size(18).style(subtitle))
        .padding(8)
        .style(tag_style);

    column = column.push(row![horizontal_space().width(Length::Fill), preview_tag]);

    container(column)
        .padding(16)
        .style(light_container_style)
        .into()
}

pub fn h_federation_item(item: &FederationItem) -> Element<Message> {
    let FederationItem {
        id,
        name,
        balance,
        guardians,
        module_kinds: _, // We don't care about module kinds
        metadata,
        active: _,
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

    if let Some(welcome) = metadata.welcome_message.as_ref() {
        column = column.push(text(welcome).size(18).style(subtitle));
    }

    column = column.push(h_balance_display(*balance));

    let remove_button = h_small_button("", SvgIcon::Trash, false).on_press(
        Message::SetConfirmModal(Some(ConfirmModalState {
            title: "Are you sure?".to_string(),
            description: format!("This will remove {} from your list of mints.", name),
            confirm_action: Box::new(Message::RemoveFederation(*id)),
            cancel_action: Box::new(Message::SetConfirmModal(None)),
            confirm_button_text: "Remove Mint".to_string(),
        })),
    );

    column = column.push(row![
        horizontal_space().width(Length::Fill),
        remove_button.width(48)
    ]);

    container(column)
        .padding(16)
        .style(light_container_style)
        .into()
}
