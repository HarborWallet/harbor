use crate::components::{h_button, h_input, SvgIcon};
use iced::{
    widget::{center, column, container, text, Svg},
    Color, Theme,
};
use iced::{Alignment, Element, Length};

use crate::{HarborWallet, Message};

pub fn unlock(harbor: &HarborWallet) -> Element<Message, Theme> {
    let unlock_button = h_button("Unlock", SvgIcon::DownLeft, false)
        .on_press(Message::Unlock(harbor.password_input_str.clone()))
        .width(Length::Fill);

    let password_input = h_input(
        "Password",
        "",
        &harbor.password_input_str,
        Message::PasswordInputChanged,
        Message::Unlock(harbor.password_input_str.clone()),
        true,
        Some("password_unlock_input"),
        None,
    );

    let harbor_logo = Svg::from_path("assets/harbor_logo.svg")
        .width(167)
        .height(61);

    if let Some(e) = &harbor.unlock_failure_reason {
        let error_reason = text(format!("Error: {:?}", e))
            .size(24)
            .color(Color::from_rgb8(250, 0, 80));

        let page_columns = column![harbor_logo, password_input, unlock_button, error_reason]
            .spacing(32)
            .align_items(Alignment::Center)
            .width(Length::Fixed(256.));
        container(center(page_columns)).into()
    } else {
        let page_columns = column![harbor_logo, password_input, unlock_button]
            .spacing(32)
            .align_items(Alignment::Center)
            .width(Length::Fixed(256.));
        container(center(page_columns)).into()
    }
}
