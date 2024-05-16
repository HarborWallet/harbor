use iced::widget::text;
use iced::widget::text::Style;
use iced::{Element, Theme};

use crate::Message;

use super::lighten;

pub fn h_caption_text(string: &'static str) -> Element<'static, Message> {
    text(string)
        .size(18)
        .style(|theme: &Theme| {
            let gray = lighten(theme.palette().background, 0.5);
            Style { color: Some(gray) }
        })
        .into()
}
