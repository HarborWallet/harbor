use crate::{
    components::{h_button, h_input, harbor_logo, InputArgs, SvgIcon},
    UnlockStatus,
};
use iced::{
    widget::{center, column, container, text},
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

    let password_input = h_input(InputArgs {
        label: "Password",
        value: &harbor.password_input_str,
        on_input: Message::PasswordInputChanged,
        on_submit: action,
        disabled: harbor.unlock_status == UnlockStatus::Unlocking,
        secure: true,
        id: Some("password_unlock_input"),
        ..InputArgs::default()
    });

    let page_column = column![harbor_logo(), password_input, unlock_button,]
        .spacing(32)
        .align_x(Alignment::Center)
        .width(Length::Fixed(256.));

    let page_column = page_column.push_maybe(harbor.unlock_failure_reason.as_ref().map(|r| {
        text(format!("Error: {:?}", r))
            .size(24)
            .color(Color::from_rgb8(250, 0, 80))
    }));

    container(center(page_column)).into()
}
