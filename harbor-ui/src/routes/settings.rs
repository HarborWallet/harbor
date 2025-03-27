use harbor_client::bitcoin::Network;
use iced::widget::{column, pick_list, row, text};
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
        Some("Receive bitcoin on-chain with all Fedimint mints."),
        harbor.onchain_receive_enabled,
        false,
        |enabled| {
            // Only warn them if they're enabling it
            if enabled {
                Message::SetConfirmModal(Some(crate::components::ConfirmModalState {
                    title: "WARNING: Use at your own risk!".to_string(),
                    description: "On-chain receive is not fully supported on older mints and CAN RESULT IN LOSS OF FUNDS. If you can't think of why that would happen, then this feature is not for you!".to_string(),
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
        false,
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

    let show_seed_words_button =
        h_button("Show Seed Words", SvgIcon::Eye, false).on_press(Message::ShowSeedWords(true));

    let debug_stuff = if cfg!(debug_assertions) {
        Some(debug_stuff(harbor))
    } else {
        None
    };

    let column = column![
        header,
        onchain_receive_checkbox,
        tor_enabled_checkbox,
        network_column,
        show_seed_words_button,
        open_data_dir_button,
    ]
    .push_maybe(debug_stuff);

    basic_layout(column.spacing(48))
}

// Function to format seed words in a two-column layout
pub fn render_seed_words(seed_words: &str) -> Element<'static, Message> {
    let words: Vec<&str> = seed_words.split_whitespace().collect();

    // Create left column (words 1-6)
    let left_column = column(
        words
            .iter()
            .take(6)
            .enumerate()
            .map(|(i, word)| text(format!("{}. {}", i + 1, word)).into())
            .collect::<Vec<Element<'_, Message>>>(),
    )
    .spacing(10);

    // Create right column (words 7-12)
    let right_column = column(
        words
            .iter()
            .skip(6)
            .take(6)
            .enumerate()
            .map(|(i, word)| text(format!("{}. {}", i + 7, word)).into())
            .collect::<Vec<Element<'_, Message>>>(),
    )
    .spacing(10);

    // Create a container for the words with some spacing
    let words_container = column![
        row![left_column, right_column].spacing(40),
        // Add copy button at the bottom
        row![
            h_button("Copy Seed Words", SvgIcon::Copy, false)
                .on_press(Message::CopyToClipboard(seed_words.to_string()))
        ]
    ]
    .spacing(20);

    words_container.into()
}
