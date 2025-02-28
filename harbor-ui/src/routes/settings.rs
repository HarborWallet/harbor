use bitcoin::Network;
use iced::widget::{column, pick_list, text};
use iced::{Element, Length, Padding};

use crate::components::{
    SvgIcon, basic_layout, debug_stuff, h_button, h_checkbox, h_header, menu_style,
    pick_list_style, regular_text, very_subtle,
};
use crate::{HarborWallet, Message};

pub fn settings(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Settings", "The fun stuff.");

    let onchain_receive_checkbox = h_checkbox(
        "On-chain Receive",
        Some("Receive bitcoin on-chain. Advanced users only. Risky."),
        harbor.onchain_receive_enabled,
        |enabled| {
            // Only warn them if they're enabling it
            if enabled {
                Message::SetConfirmModal(Some(crate::components::ConfirmModalState {
                    title: "WARNING: Use at your own risk!".to_string(),
                    description: "On-chain receive is not fully supported and CAN RESULT IN LOSS OF FUNDS. If you can't think of why that would happen, then this feature is not for you!".to_string(),
                    confirm_action: Box::new(Message::SetOnchainReceiveEnabled(enabled)),
                cancel_action: Box::new(Message::SetConfirmModal(None)),
                    confirm_button_text: "YOLO".to_string(),
                }))
            } else {
                Message::SetOnchainReceiveEnabled(false)
            }
        },
    );

    let network_label = regular_text("Network".to_string(), 24);
    let network_description = text("Switch networks to test Harbor with fake money.")
        .style(very_subtle)
        .size(14);
    let network_list = pick_list(
        [
            Network::Bitcoin,
            Network::Testnet,
            Network::Testnet4,
            Network::Signet,
            Network::Regtest,
        ],
        Some(harbor.config.network),
        |net| {
            Message::SetConfirmModal(Some(crate::components::ConfirmModalState {
                title: "Are you sure?".to_string(),
                description: format!(
                    "Changing network requires a restart, are you sure you want to change to {net}?"
                ),
                confirm_action: Box::new(Message::ChangeNetwork(net)),
                cancel_action: Box::new(Message::SetConfirmModal(None)),
                confirm_button_text: "Confirm".to_string(),
            }))
        },
    )
    .style(pick_list_style)
    .padding(Padding::from(16))
    .width(Length::Fill)
    .handle(pick_list::Handle::Arrow {
        size: Some(iced::Pixels(24.)),
    })
    .menu_style(menu_style);

    let network_column = column![network_label, network_list, network_description].spacing(8);

    let open_data_dir_button = h_button("Open Data Directory", SvgIcon::FolderLock, false)
        .on_press(Message::OpenDataDirectory);

    let tor_enabled_checkbox = h_checkbox(
        "Tor",
        Some("Use Tor for enhanced privacy. Requires restart."),
        harbor.tor_enabled,
        |enabled| {
            Message::SetConfirmModal(Some(crate::components::ConfirmModalState {
                title: "Are you sure?".to_string(),
                description: format!(
                    "Changing Tor settings requires a restart, are you sure you want to {} Tor?",
                    if enabled { "enable" } else { "disable" }
                ),
                confirm_action: Box::new(Message::SetTorEnabled(enabled)),
                cancel_action: Box::new(Message::SetConfirmModal(None)),
                confirm_button_text: "Confirm".to_string(),
            }))
        },
    );

    let column = match (harbor.settings_show_seed_words, &harbor.seed_words) {
        (true, Some(s)) => {
            let button = h_button("Hide Seed Words", SvgIcon::EyeClosed, false)
                .on_press(Message::ShowSeedWords(false));

            let words = text(s).size(24);

            let copy_button = h_button("Copy Seed Words", SvgIcon::Copy, false)
                .on_press(Message::CopyToClipboard(s.clone()));

            column![header, button, words, copy_button]
        }
        _ => {
            let button = h_button("Show Seed Words", SvgIcon::Eye, false)
                .on_press(Message::ShowSeedWords(true));

            let debug_stuff = if cfg!(debug_assertions) {
                Some(debug_stuff(harbor))
            } else {
                None
            };

            column![
                header,
                onchain_receive_checkbox,
                tor_enabled_checkbox,
                network_column,
                button,
                open_data_dir_button,
            ]
            .push_maybe(debug_stuff)
        }
    };

    basic_layout(column.spacing(48))
}
