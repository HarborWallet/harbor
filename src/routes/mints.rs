use iced::widget::{column, container, scrollable};
use iced::Length;
use iced::{Alignment, Element};

use crate::{HarborWallet, Message};

pub fn mints(_harbor: &HarborWallet) -> Element<Message> {
    container(
        scrollable(
            column!["These are the mints!",]
                .spacing(32)
                .align_items(Alignment::Center)
                .width(Length::Fill),
        )
        .height(Length::Fill),
    )
    .into()
}
