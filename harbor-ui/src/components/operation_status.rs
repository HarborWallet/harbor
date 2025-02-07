use iced::widget::{column, container, text};
use iced::{Element, Length};
use uuid::Uuid;

use super::{subtitle, tag_style, very_subtle};
use crate::HarborWallet;
use crate::Message;

pub fn operation_status(harbor: &HarborWallet) -> Option<Element<'static, Message>> {
    operation_status_for_id(harbor, None)
}

pub fn operation_status_for_id(
    harbor: &HarborWallet,
    id: Option<Uuid>,
) -> Option<Element<'static, Message>> {
    let status_text = if let Some(id) = id {
        // Get status for specific operation
        harbor
            .operation_status
            .get(&id)
            .map(|s| vec![s.message.clone()])
            .unwrap_or_default()
    } else {
        // Get all operation statuses
        harbor
            .operation_status
            .values()
            .map(|s| s.message.clone())
            .collect()
    };

    let status_column = column![text(status_text.join("\n")).size(18).style(subtitle)]
        // Add Tor notice if enabled
        .push_maybe(if harbor.tor_enabled {
            Some(
                text("Tor enabled. Please be patient!")
                    .size(14)
                    .style(very_subtle),
            )
        } else {
            None
        })
        .spacing(8);

    if status_text.is_empty() {
        None
    } else {
        Some(
            container(status_column)
                .width(Length::Fill)
                .padding(8)
                .style(tag_style)
                .into(),
        )
    }
}
