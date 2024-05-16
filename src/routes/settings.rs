use iced::widget::{column, text};
use iced::Element;

use crate::components::{basic_layout, h_button, h_header, SvgIcon};
use crate::{HarborWallet, Message};

pub fn settings(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Settings", "The fun stuff.");

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

            column![header, button]
        }
    };

    basic_layout(column.spacing(48))
}
