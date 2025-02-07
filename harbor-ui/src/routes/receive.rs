use crate::components::{
    basic_layout, h_button, h_caption_text, h_header, h_input, h_screen_header,
    operation_status_for_id, InputArgs, SvgIcon,
};
use crate::{HarborWallet, Message, ReceiveMethod, ReceiveStatus};
use iced::widget::container::Style;
use iced::widget::{column, container, qr_code, radio, row, text};
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

    let column = match receive_string {
        None => {
            let header = if !harbor.onchain_receive_enabled {
                h_header("Deposit", "Receive via lightning.")
            } else {
                h_header("Deposit", "Receive on-chain or via lightning.")
            };

            let generating = harbor.receive_status == ReceiveStatus::Generating;

            let amount_input = h_input(InputArgs {
                label: "Amount",
                placeholder: "420",
                value: &harbor.receive_amount_str,
                on_input: Message::ReceiveAmountChanged,
                numeric: true,
                suffix: Some("sats"),
                disabled: generating,
                ..InputArgs::default()
            });

            let generate_invoice_button = || {
                h_button("Generate Invoice", SvgIcon::Qr, generating)
                    .on_press(Message::GenerateInvoice)
            };

            let generate_address_button = || {
                h_button("Generate Address", SvgIcon::Qr, generating)
                    .on_press(Message::GenerateAddress)
            };

            let start_over_button = || {
                h_button("Start Over", SvgIcon::Restart, false)
                    .on_press(Message::CancelReceiveGeneration)
            };

            let mut button_and_status = if generating {
                column![row![start_over_button(), generate_invoice_button()].spacing(8)]
            } else {
                column![generate_invoice_button()]
            };

            // Add status display with 16px spacing if we have a current operation
            if let Some(status) = harbor
                .current_receive_id
                .and_then(|id| operation_status_for_id(harbor, Some(id)))
            {
                button_and_status = button_and_status.push(status).spacing(16);
            }

            if !harbor.onchain_receive_enabled {
                column![header, amount_input, button_and_status]
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
                        let mut button_and_status = if generating {
                            column![row![start_over_button(), generate_invoice_button()].spacing(8)]
                        } else {
                            column![generate_invoice_button()]
                        };

                        // Add status display with 16px spacing if we have a current operation
                        if let Some(status) = harbor
                            .current_receive_id
                            .and_then(|id| operation_status_for_id(harbor, Some(id)))
                        {
                            button_and_status = button_and_status.push(status).spacing(16);
                        }

                        column![header, method_choice, amount_input, button_and_status]
                    }
                    ReceiveMethod::OnChain => {
                        let mut button_and_status = if generating {
                            column![row![start_over_button(), generate_address_button()].spacing(8)]
                        } else {
                            column![generate_address_button()]
                        };

                        // Add status display with 16px spacing if we have a current operation
                        if let Some(status) = harbor
                            .current_receive_id
                            .and_then(|id| operation_status_for_id(harbor, Some(id)))
                        {
                            button_and_status = button_and_status.push(status).spacing(16);
                        }

                        column![header, method_choice, button_and_status]
                    }
                }
            }
        }
        // We've generated an invoice or address
        Some(receive_string) => {
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
    };

    column![
        h_screen_header(harbor, true),
        basic_layout(column.spacing(48)),
    ]
    .into()
}
