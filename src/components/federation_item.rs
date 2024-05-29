use fedimint_core::config::FederationId;
use iced::{
    widget::{column, row, text},
    Alignment, Element,
};

use crate::Message;

use super::{format_amount, map_icon, subtitle, SvgIcon};

#[derive(Debug, Clone)]
pub struct FederationItem {
    pub id: FederationId,
    pub name: String,
    pub balance: u64,
}

pub fn h_federation_item(item: &FederationItem) -> Element<Message> {
    let FederationItem { id, name, balance } = item;
    let title = row![map_icon(SvgIcon::People, 24., 24.), text(name).size(24)]
        .align_items(Alignment::Center)
        .spacing(16);
    let balance = text(format_amount(*balance)).size(24);

    let id = text(format!("{id}")).size(18).style(subtitle);

    let column = column![title, balance, id].spacing(16);

    column.into()
}
