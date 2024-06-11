use crate::{
    components::{h_button, h_input, harbor_logo, the_spinner, SvgIcon},
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
                .align_items(Alignment::Center)
                .width(Length::Fixed(350.))
        }
        WelcomeStatus::NeedsInit => {
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

            let password_input = h_input(
                "Password",
                "",
                &harbor.password_input_str,
                Message::PasswordInputChanged,
                action.clone(),
                true,
                Some("password_init_input"),
                None,
            );

            let welcome_message = text("Welcome, we're glad you are here.").size(24);

            column![harbor_logo(), welcome_message, password_input, new_wallet]
                .spacing(32)
                .align_items(Alignment::Center)
                .width(Length::Fixed(350.))
        }
    };

    container(center(column)).into()
}
