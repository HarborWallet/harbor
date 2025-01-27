use iced::{
    widget::{
        button::{self, Status},
        center, horizontal_space, rich_text, row, span, text, Button, Row,
    },
    Border, Color, Element, Length, Shadow, Theme,
};

use crate::{Message, Route};

use super::{darken, lighten, link, map_icon, the_spinner, SvgIcon};

pub fn h_button(text_str: &str, icon: SvgIcon, loading: bool) -> Button<'_, Message, Theme> {
    let spinner: Element<'static, Message, Theme> = the_spinner();
    let svg = map_icon(icon, 24., 24.);
    let content = if loading {
        row![spinner].align_y(iced::Alignment::Center)
    } else if text_str.is_empty() {
        row![svg].align_y(iced::Alignment::Center)
    } else {
        row![svg, text(text_str).size(24.)]
            .align_y(iced::Alignment::Center)
            .spacing(16)
    };

    Button::new(center(content))
        .style(move |theme, status| {
            let gray = lighten(theme.palette().background, 0.5);

            let border_color = if loading || matches!(status, Status::Disabled) {
                gray
            } else {
                Color::WHITE
            };

            let border = Border {
                color: border_color,
                width: 2.,
                radius: (8.).into(),
            };

            let background = if loading {
                theme.palette().background
            } else {
                match status {
                    Status::Hovered => lighten(theme.palette().background, 0.1),
                    Status::Pressed => darken(Color::BLACK, 0.1),
                    _ => theme.palette().background,
                }
            };

            let text_color = if loading { gray } else { Color::WHITE };

            button::Style {
                background: Some(background.into()),
                text_color,
                border,
                shadow: Shadow::default(),
            }
        })
        .width(Length::Fill)
        .height(Length::Fixed(64.))
}

pub fn h_icon_button(icon: SvgIcon) -> Button<'static, Message, Theme> {
    let svg = map_icon(icon, 24., 24.);
    let content = row![svg].align_y(iced::Alignment::Center);

    Button::new(center(content))
        .style(move |theme, status| {
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

            let text_color = Color::WHITE;

            button::Style {
                background: Some(background.into()),
                text_color,
                border,
                shadow: Shadow::default(),
            }
        })
        .width(Length::Fixed(48.))
        .height(Length::Fixed(48.))
}

pub fn sidebar_button(
    text_str: &str,
    icon: SvgIcon,
    self_route: Route,
    active_route: Route,
) -> Button<'_, Message, Theme> {
    let is_active = self_route == active_route;
    let svg = map_icon(icon, 24., 24.);
    let content = row!(svg, text(text_str).size(24.), horizontal_space(),)
        .align_y(iced::Alignment::Center)
        .spacing(16)
        .padding(8);

    Button::new(content)
        .style(move |theme, status| {
            let gray = lighten(theme.palette().background, 0.5);

            let border = Border {
                color: Color::WHITE,
                width: 0.,
                radius: (8.).into(),
            };

            let bg_color = if is_active {
                lighten(theme.palette().background, 0.1)
            } else {
                lighten(theme.palette().background, 0.05)
            };

            let background = match (status, is_active) {
                (_, true) => bg_color,
                (Status::Hovered, false) => lighten(bg_color, 0.05),
                (Status::Pressed, false) => darken(bg_color, 0.1),
                _ => bg_color,
            };

            let text_color = match status {
                Status::Disabled => gray,
                _ => Color::WHITE,
            };

            button::Style {
                background: Some(background.into()),
                text_color,
                border,
                shadow: Shadow::default(),
            }
        })
        .width(Length::Fixed(192.))
}

pub fn text_link<'a>(text_str: String, url: String) -> Row<'a, Message, Theme> {
    let svg = map_icon(SvgIcon::ExternalLink, 16., 16.);
    let text = rich_text([span(text_str)
        .link(Message::UrlClicked(url.clone()))
        .underline(true)
        .color(link())]);
    row![svg, text].align_y(iced::Alignment::Center).spacing(8)
}
