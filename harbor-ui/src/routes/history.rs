use iced::Element;

use crate::components::{
    basic_layout, basic_layout_with_sidebar, h_header, h_transaction_details, h_transaction_item,
    hr,
};
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
                column.push(h_transaction_item(item)).push(hr())
            })
            .spacing(16)
    };
    let left_column = column![header, transactions].spacing(48);

    if let Some(selected_tx) = &harbor.selected_transaction {
        // TODO: auto-collapse sidebar when window width narrows below X px
        basic_layout_with_sidebar(left_column, column![h_transaction_details(selected_tx)])
    } else {
        basic_layout(left_column)
    }
}
