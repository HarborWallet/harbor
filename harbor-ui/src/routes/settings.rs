use iced::widget::{column, text};
use iced::Element;

use crate::components::{basic_layout, h_button, h_header, SvgIcon, Toast, ToastStatus};
use crate::{HarborWallet, Message};

pub fn settings(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Settings", "The fun stuff.");

    let add_good_toast_button =
        h_button("Nice!", SvgIcon::Plus, false).on_press(Message::AddToast(Toast {
            title: "Hello".to_string(),
            body: "This is a toast".to_string(),
            status: ToastStatus::Good,
        }));

    let add_error_toast_button =
        h_button("Error Toast", SvgIcon::Plus, false).on_press(Message::AddToast(Toast {
            title: "Error".to_string(),
            body: "This is a toast".to_string(),
            status: ToastStatus::Bad,
        }));

    let column = match (harbor.settings_show_seed_words, &harbor.seed_words) {
        (true, Some(s)) => {
            let button = h_button("Hide Seed Words", SvgIcon::Squirrel, false)
                .on_press(Message::ShowSeedWords(false));

            let words = text(s).size(24);

            let copy_button = h_button("Copy Seed Words", SvgIcon::Copy, false)
                .on_press(Message::CopyToClipboard(s.clone()));

            column![header, button, words, copy_button]
        }
        _ => {
            let button = h_button("Show Seed Words", SvgIcon::Squirrel, false)
                .on_press(Message::ShowSeedWords(true));

            column![
                header,
                button,
                add_good_toast_button,
                add_error_toast_button
            ]
        }
    };

    basic_layout(column.spacing(48))
}
