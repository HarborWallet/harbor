use iced::Length;
use iced::widget::{Column, container, horizontal_space, row, scrollable};
use iced::{Element, Padding};

use crate::Message;

pub fn basic_layout(column: Column<Message>) -> Element<Message> {
    container(
        scrollable(row![
            column
                .width(Length::Fixed(512.))
                .padding(Padding::new(48.))
                .max_width(512),
            horizontal_space(),
        ])
        .height(Length::Fill),
    )
    .into()
}
