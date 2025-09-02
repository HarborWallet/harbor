use iced::{Element, Length};

use crate::components::{basic_layout, h_header, h_transaction_details, h_transaction_item, hr};
use crate::{HarborWallet, Message};
use iced::widget::{column, horizontal_space, row, stack, text};

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
                    .is_some_and(|selected| selected == item);
                column
                    .push(h_transaction_item(item, is_selected))
                    .push(hr())
            })
            .spacing(16)
    };
    let left_column = column![header, transactions].spacing(48);

    let content = basic_layout(left_column);
    let mut layers = stack![content];

    if let Some(selected_tx) = &harbor.selected_transaction {
        let details = h_transaction_details(selected_tx, &harbor.mint_list, harbor.config.network);

        layers = layers.push(row![
            horizontal_space(),
            details,
            horizontal_space().width(Length::Fixed(10.))
        ]);
    };

    layers.into()
}
