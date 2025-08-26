use iced::{
    Border, Color, Length, Shadow, Theme,
    widget::{
        Button,
        button::{self, Status},
    },
};

use crate::Message;

use super::{SvgIcon, darken, lighten, map_icon};

pub fn mini_copy(text: String) -> Button<'static, Message, Theme> {
    let icon = map_icon(SvgIcon::Copy, 24., 24.);

    Button::new(icon)
        .on_press(Message::CopyToClipboard(text))
        .style(|theme: &Theme, status| {
            let border = Border {
                color: Color::WHITE,
                width: 0.,
                radius: (8.).into(),
            };

            let background = match status {
                Status::Hovered => lighten(theme.palette().background, 0.1),
                Status::Pressed => darken(Color::BLACK, 0.1),
                _ => theme.palette().background,
            };
            button::Style {
                background: Some(background.into()),
                text_color: Color::WHITE,
                border,
                shadow: Shadow::default(),
            }
        })
        .padding(6)
        .width(Length::Fixed(32.))
        .height(Length::Fixed(32.))
}
