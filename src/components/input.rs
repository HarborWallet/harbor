use iced::{
    widget::{
        column, text,
        text_input::{self, focus, Id},
        TextInput,
    },
    Background, Border, Color, Command, Element, Theme,
};

use crate::Message;

use super::{darken, lighten};

pub fn focus_input_id(id: &'static str) -> Command<Message> {
    let id = Id::new(id);
    focus(id)
}

pub fn h_input<'a>(
    label: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    on_submit: Message,
    secure: bool,
    id: Option<&'static str>,
) -> Element<'a, Message, Theme> {
    let input = TextInput::new(placeholder, value)
        .style(|theme: &Theme, status| {
            let gray = lighten(theme.palette().background, 0.5);
            let border_color = match status {
                text_input::Status::Active => Color::WHITE,
                text_input::Status::Focused => theme.palette().primary,
                text_input::Status::Hovered => darken(Color::WHITE, 0.2),
                text_input::Status::Disabled => gray,
            };
            let border = Border {
                color: border_color,
                width: 2.,
                radius: (8.).into(),
            };

            text_input::Style {
                background: Background::Color(Color::BLACK),
                border,
                placeholder: gray,
                value: Color::WHITE,
                icon: Color::WHITE,
                selection: theme.palette().primary,
            }
        })
        .size(24)
        .padding(8)
        .secure(secure)
        .on_input(on_input)
        .on_submit(on_submit);

    let label = text(label).size(24);

    if let Some(id) = id {
        let id = Id::new(id);
        // input.id(id);
        column![label, input.id(id)].spacing(8).into()
    } else {
        column![label, input].spacing(8).into()
    }
}
