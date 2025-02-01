use bitcoin::Network;
use iced::widget::{column, pick_list, text, Checkbox};
use iced::{Element, Padding};

use crate::components::{basic_layout, h_button, h_header, menu_style, pick_list_style, SvgIcon};
use crate::components::{Toast, ToastStatus};
use crate::{HarborWallet, Message};

pub fn settings(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Settings", "The fun stuff.");

    let onchain_receive_checkbox =
        Checkbox::new("Enable On-chain Receive", harbor.onchain_receive_enabled)
            .on_toggle(Message::SetOnchainReceiveEnabled);

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
    .handle(pick_list::Handle::Arrow {
        size: Some(iced::Pixels(24.)),
    })
    .menu_style(menu_style);

    let add_good_toast_button =
        h_button("Nice!", SvgIcon::Plus, false).on_press(Message::AddToast(Toast {
            title: "Hello".to_string(),
            body: Some("This is a toast".to_string()),
            status: ToastStatus::Good,
        }));

    let add_error_toast_button =
        h_button("Error Toast", SvgIcon::Plus, false).on_press(Message::AddToast(Toast {
            title: "Error".to_string(),
            body: Some("This is a toast".to_string()),
            status: ToastStatus::Bad,
        }));

    let test_confirm_modal_button = h_button("Test Confirm Modal", SvgIcon::Shield, false)
        .on_press(Message::SetConfirmModal(Some(
            crate::components::ConfirmModalState {
                title: "Test Modal".to_string(),
                description:
                    "This is a test of the confirm modal. Are you sure you want to proceed?"
                        .to_string(),
                confirm_action: Box::new(Message::Batch(vec![
                    Message::AddToast(Toast {
                        title: "You confirmed!".to_string(),
                        body: None,
                        status: ToastStatus::Good,
                    }),
                    Message::SetConfirmModal(None),
                ])),
                cancel_action: Box::new(Message::SetConfirmModal(None)),
                confirm_button_text: "Confirm".to_string(),
            },
        )));

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

            column![
                header,
                button,
                onchain_receive_checkbox,
                network_list,
                add_good_toast_button,
                add_error_toast_button,
                test_confirm_modal_button
            ]
        }
    };

    basic_layout(column.spacing(48))
}
