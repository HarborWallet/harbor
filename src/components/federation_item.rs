use fedimint_core::config::FederationId;
use iced::{
    widget::{column, row, text},
    Element,
};

use crate::Message;

use super::{bold_text, regular_text, truncate_text};

#[derive(Debug, Clone)]
pub struct FederationItem {
    pub id: FederationId,
    pub name: String,
    pub guardians: Option<Vec<String>>,
}

pub fn h_federation_item(item: &FederationItem) -> Element<Message> {
    let FederationItem {
        id,
        name,
        guardians,
    } = item;

    let name_row = row![
        bold_text("Name: ".to_string(), 24),
        regular_text(name.to_string(), 24)
    ]
    .spacing(8);
    let id_row = row![
        bold_text("Federation id: ".to_string(), 24),
        regular_text(truncate_text(&id.to_string(), 20, true).to_string(), 24)
    ]
    .spacing(8);
    // Create the column and conditionally add guardians_row if guardians are available
    let mut column = column![name_row, id_row];

    if let Some(guardians) = guardians {
        let guardian_str = guardians.join(", ");
        let guardians_row = row![
            bold_text("Guardians: ".to_string(), 24),
            regular_text(guardian_str, 24),
        ]
        .spacing(8);
        column = column.push(guardians_row);
    }

    column.into()
}
