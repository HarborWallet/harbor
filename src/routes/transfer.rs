use iced::widget::{column, container, scrollable, text_input};
use iced::Length;
use iced::{Alignment, Element};

use crate::{HarborWallet, Message};

pub fn transfer(harbor: &HarborWallet) -> Element<Message> {
    container(
        scrollable(
            column![
                "Let's transfer some ecash!",
                text_input("how much?", &harbor.transfer_amount_str)
                    .on_input(Message::TransferAmountChanged,)
            ]
            .spacing(32)
            .align_items(Alignment::Center)
            .width(Length::Fill),
        )
        .height(Length::Fill),
    )
    .into()
}
