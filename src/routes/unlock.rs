use crate::{
    components::{h_button, h_input, SvgIcon},
    UnlockStatus,
};
use iced::{
    widget::{center, column, container, text, Svg},
    Color,
};
use iced::{Alignment, Element, Length};

use crate::{HarborWallet, Message};

pub fn unlock(harbor: &HarborWallet) -> Element<Message> {
    let action = if harbor.unlock_status == UnlockStatus::Unlocking {
        None
    } else {
        Some(Message::Unlock(harbor.password_input_str.clone()))
    };

    let unlock_button = h_button(
        "Unlock",
        SvgIcon::DownLeft,
        harbor.unlock_status == UnlockStatus::Unlocking,
    )
    .on_press_maybe(action.clone())
    .width(Length::Fill);

    let password_input = h_input(
        "Password",
        "",
        &harbor.password_input_str,
        Message::PasswordInputChanged,
        action,
        true,
        Some("password_unlock_input"),
        None,
    );

    let harbor_logo = Svg::from_path("assets/harbor_logo.svg")
        .width(167)
        .height(61);

    let page_column = column![harbor_logo, password_input, unlock_button,]
        .spacing(32)
        .align_items(Alignment::Center)
        .width(Length::Fixed(256.));

    let page_column = page_column.push_maybe(harbor.unlock_failure_reason.as_ref().map(|r| {
        text(format!("Error: {:?}", r))
            .size(24)
            .color(Color::from_rgb8(250, 0, 80))
    }));

    container(center(page_column)).into()
}
