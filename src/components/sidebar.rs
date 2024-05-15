use crate::components::SvgIcon;
use iced::widget::container::Style;
use iced::widget::{column, container, vertical_space, Svg};
use iced::Border;
use iced::{Alignment, Element, Shadow};

use crate::{HarborWallet, Message, Route};

use super::{lighten, sidebar_button};

pub fn sidebar(harbor: &HarborWallet) -> Element<Message> {
    let sidebar = container(
        column![
            Svg::from_path("assets/harbor_logo.svg")
                .width(167)
                .height(61),
            sidebar_button("Home", SvgIcon::Home, Route::Home, harbor.active_route)
                .on_press(Message::Navigate(Route::Home)),
            sidebar_button("Mints", SvgIcon::People, Route::Mints, harbor.active_route)
                .on_press(Message::Navigate(Route::Mints)),
            sidebar_button(
                "Transfer",
                SvgIcon::LeftRight,
                Route::Transfer,
                harbor.active_route
            )
            .on_press(Message::Navigate(Route::Transfer)),
            sidebar_button(
                "History",
                SvgIcon::Squirrel,
                Route::History,
                harbor.active_route
            )
            .on_press(Message::Navigate(Route::History)),
            vertical_space(),
            sidebar_button(
                "Settings",
                SvgIcon::Settings,
                Route::Settings,
                harbor.active_route
            )
            .on_press(Message::Navigate(Route::Settings)),
            sidebar_button("Donate", SvgIcon::Heart, Route::Donate, harbor.active_route)
                .on_press(Message::Navigate(Route::Donate)),
        ]
        .spacing(8)
        .align_items(Alignment::Start),
    )
    .padding(8)
    .style(|theme| -> Style {
        Style {
            text_color: None,
            background: Some(lighten(theme.palette().background, 0.05).into()),
            border: Border::default(),
            shadow: Shadow::default(),
        }
    });
    sidebar.into()
}
