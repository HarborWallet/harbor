use iced::widget::text::Style;
use iced::widget::{column, text};
use iced::{Element, Theme};

use crate::Message;

use super::lighten;

pub fn h_header(title: &'static str, subtitle: &'static str) -> Element<'static, Message> {
    column![
        text(title).size(32),
        text(subtitle).size(18).style(|theme: &Theme| {
            let gray = lighten(theme.palette().background, 0.5);
            Style { color: Some(gray) }
        })
    ]
    .spacing(8)
    .into()
}
