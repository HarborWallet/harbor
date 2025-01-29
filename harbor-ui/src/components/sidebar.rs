use crate::components::indicator::Position;
use crate::components::SvgIcon;
use crate::routes::MintSubroute;
use iced::widget::container::Style;
use iced::widget::{column, container, row, text, vertical_space};
use iced::{Alignment, Element, Shadow};
use iced::{Border, Theme};

use crate::{HarborWallet, Message, Route};

use super::{harbor_logo, indicator, lighten, map_icon, sidebar_button};

pub fn sidebar(harbor: &HarborWallet) -> Element<Message> {
    let transfer_disabled = harbor.federation_list.is_empty();
    let transfer_button = sidebar_button(
        "Transfer",
        SvgIcon::LeftRight,
        Route::Transfer,
        harbor.active_route,
    );
    let add_a_mint_cta = container(
        row![
            map_icon(SvgIcon::ArrowLeft, 14., 14.),
            text("Add a mint to get started").size(14)
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding(8)
    .style(|theme: &Theme| container::Style {
        text_color: Some(theme.palette().text),
        background: Some(theme.palette().primary.into()),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let sidebar = container(
        column![
            harbor_logo(),
            sidebar_button("Home", SvgIcon::Home, Route::Home, harbor.active_route)
                .on_press(Message::Navigate(Route::Home)),
            indicator(
                sidebar_button(
                    "Mints",
                    SvgIcon::People,
                    Route::Mints(MintSubroute::List),
                    harbor.active_route
                )
                .on_press(Message::Navigate(Route::Mints(MintSubroute::List))),
                add_a_mint_cta,
                Position::Right,
                harbor.show_add_a_mint_cta
            ),
            if !transfer_disabled {
                transfer_button.on_press(Message::Navigate(Route::Transfer))
            } else {
                transfer_button
            },
            sidebar_button(
                "History",
                SvgIcon::Clock,
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
        .align_x(Alignment::Start),
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
