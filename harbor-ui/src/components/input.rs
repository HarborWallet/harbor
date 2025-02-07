use iced::{
    widget::{
        column, row, text,
        text_input::{self, focus, Id},
        TextInput,
    },
    Background, Border, Color, Element, Task, Theme,
};

use crate::Message;

use super::{darken, lighten};

pub fn focus_input_id(id: &'static str) -> Task<Message> {
    let id = Id::new(id);
    focus(id)
}

pub struct InputArgs<'a> {
    pub label: &'static str,
    pub placeholder: &'static str,
    pub value: &'a str,
    pub on_input: fn(String) -> Message,
    pub on_submit: Option<Message>,
    pub disabled: bool,
    pub secure: bool,
    pub numeric: bool,
    pub id: Option<&'static str>,
    pub suffix: Option<&'static str>,
}

impl Default for InputArgs<'_> {
    fn default() -> Self {
        Self {
            label: "",
            placeholder: "",
            value: "",
            on_input: |_| Message::Noop,
            on_submit: None,
            disabled: false,
            secure: false,
            numeric: false,
            id: None,
            suffix: None,
        }
    }
}

pub fn h_input(args: InputArgs<'_>) -> Element<Message, Theme> {
    let InputArgs {
        label,
        placeholder,
        value,
        on_input,
        on_submit,
        disabled,
        secure,
        numeric,
        id,
        suffix,
    } = args;

    let on_submit = on_submit.unwrap_or(Message::Noop);

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

            let value = if text_input::Status::Disabled == status {
                gray
            } else {
                Color::WHITE
            };

            let darker_gray = darken(gray, 0.1);

            let placeholder = if text_input::Status::Disabled == status {
                darker_gray
            } else {
                gray
            };

            text_input::Style {
                background: Background::Color(Color::BLACK),
                border,
                placeholder,
                value,
                icon: Color::WHITE,
                selection: theme.palette().primary,
            }
        })
        .size(24)
        .padding(8)
        .secure(secure);

    let input = if disabled {
        input
    } else {
        // If the input isn't disable we can add the on_input and on_submit handlers
        input
            .on_input(move |text| {
                let text = if numeric {
                    let num = text.parse::<u64>().unwrap_or(0);
                    // If the value is already 0, typing 1 turns it into 10
                    // Which is annoying, so we'll just clear it
                    if num == 0 {
                        "".to_string()
                    } else {
                        num.to_string()
                    }
                } else {
                    text
                };
                on_input(text)
            })
            .on_submit(on_submit)
    };

    let label = text(label).size(24);

    let input = if let Some(id) = id {
        let id = Id::new(id);
        input.id(id)
    } else {
        input
    };

    let input = if let Some(suffix) = suffix {
        let suffix_text = text(suffix).size(24);
        row![input, suffix_text]
            .spacing(8)
            .align_y(iced::Alignment::Center)
    } else {
        row![input]
    };

    column![label, input].spacing(8).into()
}
