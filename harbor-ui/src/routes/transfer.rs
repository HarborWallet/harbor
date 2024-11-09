use iced::widget::{column, container, scrollable};
use iced::Element;
use iced::{Length, Padding};

use crate::components::h_header;
use crate::{HarborWallet, Message};

pub fn transfer(_harbor: &HarborWallet) -> Element<Message> {
    container(scrollable(
        column![h_header("Transfer", "Coming soon!")]
            .spacing(48)
            .width(Length::Fill)
            .max_width(512)
            .padding(Padding::new(48.)),
    ))
    .into()
}
