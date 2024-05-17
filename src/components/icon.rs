use iced::{widget::Svg, Element, Theme};

use crate::Message;

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
    Copy,
    Plus,
    Qr,
    Restart,
    SmallClose,
}

pub fn map_icon<'a>(icon: SvgIcon, width: f32, height: f32) -> Element<'a, Message, Theme> {
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
        SvgIcon::Copy => Svg::from_path("assets/icons/copy.svg"),
        SvgIcon::Plus => Svg::from_path("assets/icons/plus.svg"),
        SvgIcon::Qr => Svg::from_path("assets/icons/qr.svg"),
        SvgIcon::Restart => Svg::from_path("assets/icons/restart.svg"),
        SvgIcon::SmallClose => Svg::from_path("assets/icons/small_close.svg"),
    }
    .width(width)
    .height(height)
    .into()
}
