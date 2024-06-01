use fedimint_core::{config::FederationId, core::ModuleKind};
use iced::{
    widget::{column, row},
    Element,
};

use crate::Message;

use super::{bold_text, regular_text, truncate_text};

#[derive(Debug, Clone)]
pub struct FederationItem {
    pub id: FederationId,
    pub name: String,
    pub guardians: Option<Vec<String>>,
    pub module_kinds: Option<Vec<ModuleKind>>,
}

pub fn h_federation_item(item: &FederationItem) -> Element<Message> {
    let FederationItem {
        id,
        name,
        guardians,
        module_kinds,
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

    if let Some(module_kinds) = module_kinds {
        let module_str = module_kinds
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        let modules_row = row![
            bold_text("Modules: ".to_string(), 24),
            regular_text(module_str, 24),
        ]
        .spacing(8);
        column = column.push(modules_row);
    }

    column.into()
}
