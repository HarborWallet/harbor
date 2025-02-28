use iced::Element;
use iced::widget::text;

use crate::Message;

use super::subtitle;

pub fn h_caption_text(string: &'static str) -> Element<'static, Message> {
    text(string).size(18).style(subtitle).into()
}
