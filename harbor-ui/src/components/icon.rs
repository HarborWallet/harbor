use iced::{
    widget::{svg::Handle, Svg},
    Theme,
};

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
    Bolt,
    Chain,
    Eye,
}

macro_rules! icon_handle {
    ($icon:expr) => {
        Svg::new(Handle::from_memory(include_bytes!(concat!(
            "../../assets/icons/",
            $icon
        ))))
    };
}

pub fn map_icon<'a>(icon: SvgIcon, width: f32, height: f32) -> Svg<'a, Theme> {
    match icon {
        SvgIcon::ChevronDown => icon_handle!("chevron_down.svg"),
        SvgIcon::DownLeft => icon_handle!("down_left.svg"),
        SvgIcon::Heart => icon_handle!("heart.svg"),
        SvgIcon::Home => icon_handle!("home.svg"),
        SvgIcon::LeftRight => icon_handle!("left_right.svg"),
        SvgIcon::People => icon_handle!("people.svg"),
        SvgIcon::Settings => icon_handle!("settings.svg"),
        SvgIcon::Squirrel => icon_handle!("squirrel.svg"),
        SvgIcon::UpRight => icon_handle!("up_right.svg"),
        SvgIcon::Copy => icon_handle!("copy.svg"),
        SvgIcon::Plus => icon_handle!("plus.svg"),
        SvgIcon::Qr => icon_handle!("qr.svg"),
        SvgIcon::Restart => icon_handle!("restart.svg"),
        SvgIcon::SmallClose => icon_handle!("small_close.svg"),
        SvgIcon::Bolt => icon_handle!("bolt.svg"),
        SvgIcon::Chain => icon_handle!("chain.svg"),
        SvgIcon::Eye => icon_handle!("eye.svg"),
    }
    .width(width)
    .height(height)
}

pub fn harbor_logo() -> Svg<'static, Theme> {
    Svg::new(Handle::from_memory(include_bytes!(
        "../../assets/harbor_logo.svg"
    )))
    .width(167)
    .height(61)
}
