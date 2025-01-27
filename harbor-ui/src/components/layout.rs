use iced::widget::{container, horizontal_space, row, scrollable, Column};
use iced::Length;
use iced::{Element, Padding};

use crate::Message;

use super::light_container_style;

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

pub fn basic_layout_with_sidebar<'a>(
    column: Column<'a, Message>,
    sidebar: Column<'a, Message>,
) -> Element<'a, Message> {
    row![
        // First column is scrollable
        scrollable(
            column
                .width(Length::Fixed(512.))
                .padding(Padding::new(48.))
                .max_width(512)
        ),
        // Second column is sidebar
        container(sidebar)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(light_container_style)
    ]
    .into()
}
