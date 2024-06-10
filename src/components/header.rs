use iced::widget::{column, text};
use iced::Element;

use crate::Message;

use super::subtitle as subtitle_style;

pub fn h_header(title: &'static str, subtitle: &'static str) -> Element<'static, Message> {
    column![
        text(title).size(32),
        text(subtitle).size(18).style(subtitle_style)
    ]
    .spacing(8)
    .into()
}
