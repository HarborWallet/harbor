use iced::Element;

use crate::components::{
    basic_layout, h_header, h_transaction_details, h_transaction_item,
    hr,
};
use crate::components::absolute_overlay::{Absolute, Position};
use crate::{HarborWallet, Message};
use iced::widget::{column, text};

pub fn history(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("History", "Here's what's happened so far.");

    let transactions = if harbor.transaction_history.is_empty() {
        column![text("Nothing has happened yet.").size(18)]
    } else {
        harbor
            .transaction_history
            .iter()
            .fold(column![], |column, item| {
                let is_selected = harbor
                    .selected_transaction
                    .as_ref()
                    .map(|selected| selected == item)
                    .unwrap_or(false);
                column.push(h_transaction_item(item, is_selected)).push(hr())
            })
            .spacing(16)
    };
    let left_column = column![header, transactions].spacing(48);

    let content = basic_layout(left_column);

    if let Some(selected_tx) = &harbor.selected_transaction {
        let details = h_transaction_details(selected_tx);
        Absolute::new(content, Some(details), Position::TopRight).into()
    } else {
        content
    }
}
