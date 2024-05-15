use iced::widget::container::Style;
use iced::widget::{column, container, qr_code, scrollable, text};
use iced::{Border, Element, Font, Padding};
use iced::{Color, Length};

use crate::components::{h_button, h_header, h_input, SvgIcon};
use crate::{HarborWallet, Message, ReceiveStatus};

pub fn receive(harbor: &HarborWallet) -> Element<Message> {
    let receive_string = harbor
        .receive_invoice
        .as_ref()
        .map(|i| i.to_string())
        .or_else(|| harbor.receive_address.as_ref().map(|a| a.to_string()));

    let success_message = harbor.receive_success_msg.as_ref().map(|r| {
        text(format!("Success: {r:?}"))
            .size(32)
            .color(Color::from_rgb(0., 255., 0.))
    });

    let column = if let Some(success_message) = success_message {
        let header = h_header("Success!", "You did a good job!");
        let reset_button =
            h_button("Start over", SvgIcon::Squirrel, false).on_press(Message::ReceiveStateReset);
        column![header, success_message, reset_button]
    } else if let Some(string) = receive_string {
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

        let first_20_chars = string.chars().take(20).collect::<String>();

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
                .on_press(Message::CopyToClipboard(string)),
            h_button("Start over", SvgIcon::Squirrel, false).on_press(Message::ReceiveStateReset),
        ]
    } else {
        let header = h_header("Receive", "Receive on-chain or via lightning.");

        let amount_input = h_input(
            "Amount",
            "420",
            &harbor.receive_amount_str,
            Message::ReceiveAmountChanged,
            None,
            false,
            None,
            Some("sats"),
        );

        // TODO how to separate lighting and onchain?
        let generating = harbor.receive_status == ReceiveStatus::Generating;

        let generate_button = h_button("Generate Invoice", SvgIcon::DownLeft, generating)
            .on_press(Message::GenerateInvoice);

        let generate_address_button = h_button("Generate Address", SvgIcon::Squirrel, generating)
            .on_press(Message::GenerateAddress);

        column![
            header,
            amount_input,
            generate_button,
            generate_address_button
        ]
    };

    container(scrollable(
        column
            .spacing(48)
            .width(Length::Fill)
            .max_width(512)
            .padding(Padding::new(48.)),
    ))
    .into()
}
