use iced::widget::{column, text};
use iced::Element;

use crate::components::{basic_layout, h_header, h_screen_header, h_transaction_item, hr};
use crate::{HarborWallet, Message};

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

    let column = column![header, transactions].spacing(48);

    column![
        h_screen_header(harbor, true),
        basic_layout(column.spacing(48)),
    ]
    .into()
}
