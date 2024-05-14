use crate::components::{h_button, SvgIcon};
use iced::widget::{center, column, container, text_input, Svg};
use iced::{Alignment, Element, Length};

use crate::{HarborWallet, Message};

pub fn unlock(harbor: &HarborWallet) -> Element<Message> {
    // let receive_button = h_button("Receive", SvgIcon::DownLeft).on_press(Message::Receive(100));
    let unlock_button = h_button("Unlock", SvgIcon::DownLeft)
        .on_press(Message::Unlock(harbor.password_input_str.clone()))
        .width(Length::Fill);

    let password_input =
        text_input("password", &harbor.password_input_str).on_input(Message::PasswordInputChanged);

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
