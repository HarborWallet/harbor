use iced::{
    widget::{
        button::{self, Status},
        center, horizontal_space, row, text, Button, Svg,
    },
    Border, Color, Length, Shadow, Theme,
};

use crate::{Message, Route};

use super::{darken, lighten};

pub enum SvgIcon {
    ChevronDown,
    DownLeft,
    Heart,
    Home,
    LeftRight,
    People,
    Settings,
    Squirrel,
    UpRight,
}

fn map_icon(icon: SvgIcon) -> Svg<'static, Theme> {
    match icon {
        SvgIcon::ChevronDown => Svg::from_path("assets/icons/chevron_down.svg"),
        SvgIcon::DownLeft => Svg::from_path("assets/icons/down_left.svg"),
        SvgIcon::Heart => Svg::from_path("assets/icons/heart.svg"),
        SvgIcon::Home => Svg::from_path("assets/icons/home.svg"),
        SvgIcon::LeftRight => Svg::from_path("assets/icons/left_right.svg"),
        SvgIcon::People => Svg::from_path("assets/icons/people.svg"),
        SvgIcon::Settings => Svg::from_path("assets/icons/settings.svg"),
        SvgIcon::Squirrel => Svg::from_path("assets/icons/squirrel.svg"),
        SvgIcon::UpRight => Svg::from_path("assets/icons/up_right.svg"),
    }
}

pub fn h_button(text_str: &str, icon: SvgIcon) -> Button<'_, Message, Theme> {
    let svg: Svg<'_, Theme> = map_icon(icon);
    let content = row!(
        svg.width(Length::Fixed(24.)).height(Length::Fixed(24.)),
        text(text_str).size(24.) // .font(Font {
                                 //     family: iced::font::Family::default(),
                                 //     weight: iced::font::Weight::Bold,
                                 //     stretch: iced::font::Stretch::Normal,
                                 //     style: iced::font::Style::Normal,
                                 // })
    )
    .align_items(iced::Alignment::Center)
    .spacing(16);

    Button::new(center(content))
        .style(|theme, status| {
            let border = Border {
                color: Color::WHITE,
                width: 2.,
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
        .width(Length::Fixed(192.))
        .height(Length::Fixed(64.))
}

pub fn sidebar_button(
    text_str: &str,
    icon: SvgIcon,
    self_route: Route,
    active_route: Route,
) -> Button<'_, Message, Theme> {
    let is_active = self_route == active_route;
    let svg: Svg<'_, Theme> = map_icon(icon);
    let content = row!(
        svg.width(Length::Fixed(24.)).height(Length::Fixed(24.)),
        text(text_str).size(24.),
        horizontal_space(),
        // .font(Font {

        //     family: iced::font::Family::default(),
        //     weight: iced::font::Weight::Bold,
        //     stretch: iced::font::Stretch::Normal,
        //     style: iced::font::Style::Normal,
        // })
    )
    .align_items(iced::Alignment::Center)
    .spacing(16)
    .padding(16);

    Button::new(content)
        .style(move |theme, status| {
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
            button::Style {
                background: Some(background.into()),
                text_color: Color::WHITE,
                border,
                shadow: Shadow::default(),
            }
        })
        .width(Length::Fixed(192.))
}