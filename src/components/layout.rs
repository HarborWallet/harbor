use iced::widget::{container, scrollable, Column};
use iced::Length;
use iced::{Element, Padding};

use crate::Message;

pub fn basic_layout(column: Column<Message>) -> Element<Message> {
    container(
        scrollable(column.width(Length::Fixed(512.)).padding(Padding::new(48.)))
            .height(Length::Fill),
    )
    .into()
}
