use crate::components::{h_button, h_input, SvgIcon};
use iced::widget::{center, column, container, Svg};
use iced::{Alignment, Element, Length};

use crate::{HarborWallet, Message};

pub fn unlock(harbor: &HarborWallet) -> Element<Message> {
    // let receive_button = h_button("Receive", SvgIcon::DownLeft).on_press(Message::Receive(100));
    let unlock_button = h_button("Unlock", SvgIcon::DownLeft)
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
    );

    let harbor_logo = Svg::from_path("assets/harbor_logo.svg")
        .width(167)
        .height(61);

    container(center(
        column![harbor_logo, password_input, unlock_button]
            .spacing(32)
            .align_items(Alignment::Center)
            .width(Length::Fixed(256.)),
    ))
    .into()
}
