use fedimint_core::config::FederationId;
use iced::{
    widget::{row, text},
    Element,
};

use crate::Message;

#[derive(Debug, Clone)]
pub struct FederationItem {
    pub id: FederationId,
    pub name: String,
}

pub fn h_federation_item(item: &FederationItem) -> Element<Message> {
    let FederationItem { id, name } = item;
    let row = row![text(name).size(24), text(format!("{id:?}")).size(24),].spacing(16);

    row.into()
}
