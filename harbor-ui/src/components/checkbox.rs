use iced::widget::{Column, checkbox, text};
use iced::{Element, Length};

use super::{styles, very_subtle};

pub fn h_checkbox<'a, Message: 'a + Clone>(
    label: &'static str,
    description: Option<&'static str>,
    is_checked: bool,
    is_disabled: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let mut content = Column::new().spacing(8).width(Length::Fill);

    let on_toggle = if is_disabled { None } else { Some(on_toggle) };
    let checkbox = checkbox(label, is_checked)
        .on_toggle_maybe(on_toggle)
        .size(24)
        .text_size(24)
        .style(styles::checkbox_style);

    content = content.push(checkbox);

    if let Some(desc) = description {
        content = content.push(text(desc).style(very_subtle).size(14));
    }

    content.into()
}
