use iced::widget::{column, container, scrollable, text, text_input};
use iced::Length;
use iced::{Alignment, Element};

use crate::components::{h_button, SvgIcon};
use crate::{HarborWallet, Message};

pub fn receive(harbor: &HarborWallet) -> Element<Message> {
    let col = if let Some(invoice) = harbor.receive_invoice.as_ref() {
        column![
            "Here's an invoice!",
            text(format!("{invoice}")).size(16),
            h_button("Copy to clipboard", SvgIcon::Copy)
                .on_press(Message::CopyToClipboard(invoice.to_string())),
        ]
    } else {
        column![
            "How much do you want to receive?",
            text_input("how much?", &harbor.receive_amount_str)
                .on_input(Message::ReceiveAmountChanged),
            h_button("Generate Invoice", SvgIcon::DownLeft).on_press(Message::GenerateInvoice),
        ]
    };

    container(
        scrollable(
            col.spacing(32)
                .align_items(Alignment::Center)
                .width(Length::Fill),
        )
        .height(Length::Fill),
    )
    .into()
}
