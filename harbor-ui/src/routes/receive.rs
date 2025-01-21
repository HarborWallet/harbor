use crate::components::{
    basic_layout, h_button, h_caption_text, h_header, h_input, h_screen_header,
    InputArgs, SvgIcon,
};
use crate::{HarborWallet, Message, ReceiveMethod, ReceiveStatus};
use iced::widget::container::Style;
use iced::widget::{column, container, qr_code, radio, text};
use iced::Color;
use iced::{Border, Element, Font};

pub fn receive(harbor: &HarborWallet) -> Element<Message> {
    let receive_string = harbor
        .receive_invoice
        .as_ref()
        .map(|i| i.to_string())
        .or_else(|| harbor.receive_address.as_ref().map(|a| a.to_string()));

    let reset_button =
        h_button("Start over", SvgIcon::Restart, false).on_press(Message::ReceiveStateReset);

    let column = match (&harbor.receive_success_msg, receive_string) {
        // Starting state
        (None, None) => {
            let header = if !harbor.onchain_receive_enabled {
                h_header("Deposit", "Receive via lightning.")
            } else {
                h_header("Deposit", "Receive on-chain or via lightning.")
            };

            let amount_input = h_input(InputArgs {
                label: "Amount",
                placeholder: "420",
                value: &harbor.receive_amount_str,
                on_input: Message::ReceiveAmountChanged,
                numeric: true,
                suffix: Some("sats"),
                ..InputArgs::default()
            });

            let generating = harbor.receive_status == ReceiveStatus::Generating;

            let generate_button = h_button("Generate Invoice", SvgIcon::Qr, generating)
                .on_press(Message::GenerateInvoice);

            let generate_address_button = h_button("Generate Address", SvgIcon::Qr, generating)
                .on_press(Message::GenerateAddress);

            if !harbor.onchain_receive_enabled {
                column![header, amount_input, generate_button]
            } else {
                let lightning_choice = radio(
                    "Lightning",
                    ReceiveMethod::Lightning,
                    Some(harbor.receive_method),
                    Message::ReceiveMethodChanged,
                )
                .text_size(18);
                let lightning_caption =
                    h_caption_text("Good for small amounts. Instant settlement, low fees.");
                let lightning = column![lightning_choice, lightning_caption,].spacing(8);

                let onchain_choice = radio(
                    "On-chain",
                    ReceiveMethod::OnChain,
                    Some(harbor.receive_method),
                    Message::ReceiveMethodChanged,
                )
                .text_size(18);
                let onchain_caption = h_caption_text(
                    "Good for large amounts. Requires on-chain fees and 10 block confirmations.",
                );
                let onchain = column![onchain_choice, onchain_caption,].spacing(8);

                let method_choice_label = text("Method").size(24);
                let method_choice = column![method_choice_label, lightning, onchain].spacing(16);

                match harbor.receive_method {
                    ReceiveMethod::Lightning => {
                        column![header, method_choice, amount_input, generate_button]
                    }
                    ReceiveMethod::OnChain => {
                        column![header, method_choice, generate_address_button]
                    }
                }
            }
        }
        // We've generated an invoice or address
        (None, Some(receive_string)) => {
            let header = h_header("Receive", "Scan this QR or copy the string.");

            let data = harbor.receive_qr_data.as_ref().unwrap();
            let qr_code = qr_code(data).style(|_theme| iced::widget::qr_code::Style {
                background: Color::WHITE,
                cell: Color::BLACK,
            });
            let qr_container = container(qr_code).padding(16).style(|_theme| Style {
                background: Some(iced::Background::Color(Color::WHITE)),
                border: Border {
                    radius: (8.).into(),
                    ..Border::default()
                },
                ..Style::default()
            });

            let first_20_chars = receive_string.chars().take(20).collect::<String>();

            column![
                header,
                qr_container,
                text(format!("{first_20_chars}...")).size(16).font(Font {
                    family: iced::font::Family::Monospace,
                    weight: iced::font::Weight::Normal,
                    stretch: iced::font::Stretch::Normal,
                    style: iced::font::Style::Normal,
                }),
                h_button("Copy to clipboard", SvgIcon::Copy, false)
                    .on_press(Message::CopyToClipboard(receive_string)),
                reset_button
            ]
        }
        // Success states aren't rendered here, we jump to the history screen
        _ => {
            panic!("Success states aren't rendered here, you should be on the history screen right now!")
        }
    };

    column![
        h_screen_header(harbor, true),
        basic_layout(column.spacing(48)),
    ]
    .into()
}
