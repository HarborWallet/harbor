use crate::{
    components::{h_button, h_input, harbor_logo, the_spinner, InputArgs, SvgIcon},
    UnlockStatus, WelcomeStatus,
};
use iced::{
    widget::{center, column, container, text},
    Theme,
};
use iced::{Alignment, Element, Length};

use crate::{HarborWallet, Message};

pub fn welcome(harbor: &HarborWallet) -> Element<Message> {
    let column = match harbor.init_status {
        WelcomeStatus::Loading | WelcomeStatus::Inited | WelcomeStatus::Initing => {
            let welcome_message = text("Welcome, we're glad you are here.").size(24);

            let spinner: Element<'static, Message, Theme> = the_spinner();

            column![harbor_logo(), welcome_message, spinner]
                .spacing(32)
                .align_x(Alignment::Center)
                .width(Length::Fixed(350.))
        }
        WelcomeStatus::NeedsInit => {
            let welcome_message = text("Welcome, we're glad you are here.").size(24);

            if let Some(error) = &harbor.init_failure_reason {
                column![
                    harbor_logo(),
                    welcome_message,
                    text(format!(
                        "Failed to initialize wallet. Config error: {}",
                        error
                    ))
                    .size(24)
                    .color(iced::Color::from_rgb8(250, 0, 80))
                ]
                .spacing(32)
                .align_x(Alignment::Center)
                .width(Length::Fixed(350.))
            } else {
                let action = if harbor.unlock_status == UnlockStatus::Unlocking {
                    None
                } else {
                    Some(Message::Init(harbor.password_input_str.clone()))
                };

                let new_wallet = h_button(
                    "Create New Wallet",
                    SvgIcon::Plus,
                    harbor.unlock_status == UnlockStatus::Unlocking,
                )
                .on_press_maybe(action.clone())
                .width(Length::Fill);

                let password_input = h_input(InputArgs {
                    label: "Password",
                    value: &harbor.password_input_str,
                    on_input: Message::PasswordInputChanged,
                    on_submit: action.clone(),
                    disabled: harbor.unlock_status == UnlockStatus::Unlocking,
                    secure: true,
                    id: Some("password_init_input"),
                    ..InputArgs::default()
                });

                column![harbor_logo(), welcome_message, password_input, new_wallet]
                    .spacing(32)
                    .align_x(Alignment::Center)
                    .width(Length::Fixed(350.))
            }
        }
    };

    container(center(column)).into()
}
