use crate::components::{
    basic_layout, font_mono, h_button, h_caption_text, h_header, h_input, h_screen_header,
    h_small_button, operation_status_for_id, InputArgs, SvgIcon,
};
use crate::{HarborWallet, Message, ReceiveMethod, ReceiveStatus};
use iced::widget::container::Style;
use iced::widget::{column, container, horizontal_space, qr_code, radio, row, text};
use iced::{Border, Element};
use iced::{Color, Length};

/// Main view function.
pub fn receive(harbor: &HarborWallet) -> Element<Message> {
    if let Some(receive_string) = harbor
        .receive_invoice
        .as_ref()
        .map(|s| s.to_string())
        .or_else(|| harbor.receive_address.as_ref().map(|a| a.to_string()))
    {
        render_generated_view(receive_string, harbor)
    } else {
        render_receive_form(harbor)
    }
}

/// Renders the view before an invoice/address is generated.
fn render_receive_form(harbor: &HarborWallet) -> Element<Message> {
    let header = if harbor.onchain_receive_enabled {
        h_header("Deposit", "Receive on-chain or via lightning.")
    } else {
        h_header("Deposit", "Receive via lightning.")
    };

    let content = if harbor.onchain_receive_enabled {
        let method_choice = render_method_choice(harbor);
        match harbor.receive_method {
            ReceiveMethod::Lightning => {
                column![header, method_choice, render_lightning_view(harbor)]
            }
            ReceiveMethod::OnChain => {
                column![header, method_choice, render_onchain_view(harbor)]
            }
        }
    } else {
        column![header, render_lightning_view(harbor)]
    };

    column![
        h_screen_header(harbor, true, false),
        basic_layout(content.spacing(48))
    ]
    .into()
}

/// Renders the Lightning view including the amount input.
fn render_lightning_view(harbor: &HarborWallet) -> Element<Message> {
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

    // Create the "Generate Invoice" button.
    let generate_invoice_button =
        h_button("Generate Invoice", SvgIcon::Qr, generating).on_press(Message::GenerateInvoice);

    let buttons = if generating {
        // When generating, include a "Start Over" next to the generate button.
        let start_over_button = h_button("Start Over", SvgIcon::Restart, false)
            .on_press(Message::CancelReceiveGeneration);
        let mut button_group = column![row![start_over_button, generate_invoice_button].spacing(8)];

        if let Some(status) = harbor
            .current_receive_id
            .and_then(|id| operation_status_for_id(harbor, Some(id)))
        {
            button_group = button_group.push(status).spacing(16);
        }
        button_group
    } else {
        column![generate_invoice_button]
    };

    column![amount_input, buttons].spacing(48).into()
}

/// Renders the on-chain view.
fn render_onchain_view(harbor: &HarborWallet) -> Element<Message> {
    let generating = harbor.receive_status == ReceiveStatus::Generating;

    // Create the "Generate Address" button.
    let generate_address_button =
        h_button("Generate Address", SvgIcon::Qr, generating).on_press(Message::GenerateAddress);

    let buttons = if generating {
        let start_over_button = h_button("Start Over", SvgIcon::Restart, false)
            .on_press(Message::CancelReceiveGeneration);
        let mut button_group = column![row![start_over_button, generate_address_button].spacing(8)];

        if let Some(status) = harbor
            .current_receive_id
            .and_then(|id| operation_status_for_id(harbor, Some(id)))
        {
            button_group = button_group.push(status).spacing(16);
        }
        button_group
    } else {
        column![generate_address_button]
    };

    buttons.into()
}

/// Renders the method selector for on-chain enabled wallets.
fn render_method_choice(harbor: &HarborWallet) -> Element<Message> {
    let lightning_choice = radio(
        "Lightning",
        ReceiveMethod::Lightning,
        Some(harbor.receive_method),
        Message::ReceiveMethodChanged,
    )
    .text_size(18);

    let lightning_caption = h_caption_text("Good for small amounts. Instant settlement, low fees.");
    let lightning = column![lightning_choice, lightning_caption].spacing(8);

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
    let onchain = column![onchain_choice, onchain_caption].spacing(8);

    let method_choice_label = text("Method").size(24);

    column![method_choice_label, lightning, onchain]
        .spacing(16)
        .into()
}

/// Renders the view for a generated invoice/address.
fn render_generated_view(receive_string: String, harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Receive", "Scan this QR or copy the string.");

    let qr_title = if harbor.receive_method == ReceiveMethod::Lightning {
        "Lightning Invoice"
    } else {
        "On-chain Address"
    };

    let data = harbor
        .receive_qr_data
        .as_ref()
        .expect("QR data should be present");

    // TODO: update iced so we can set the size of the qr code
    let qr = qr_code(data)
        .total_size(iced::Pixels(256.))
        .style(|_theme| iced::widget::qr_code::Style {
            background: Color::WHITE,
            cell: Color::BLACK,
        });
    let qr_container = container(qr)
        .align_x(iced::Alignment::Center)
        .width(iced::Length::Fill);

    // Create a row with the truncated text and copy button
    let copy_button = h_small_button("", SvgIcon::Copy, false)
        .on_press(Message::CopyToClipboard(receive_string.clone()));

    let str = if receive_string.len() <= 20 {
        receive_string
    } else {
        let first_10_chars = receive_string.chars().take(10).collect::<String>();
        let last_10_chars = receive_string
            .chars()
            .skip(receive_string.chars().count() - 10)
            .collect::<String>();
        format!("{first_10_chars}...{last_10_chars}")
    };

    let text_and_copy = container(
        row![
            text(str).size(16).font(font_mono()).color(Color::BLACK),
            horizontal_space(),
            copy_button
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .padding(8);

    let qr_column = container(
        column![
            text(qr_title)
                .size(16)
                .font(font_mono())
                .color(Color::BLACK),
            qr_container,
            text_and_copy
        ]
        .spacing(16),
    )
    .padding(16)
    .style(|_theme| Style {
        background: Some(iced::Background::Color(Color::WHITE)),
        border: Border {
            radius: (8.).into(),
            ..Border::default()
        },
        ..Style::default()
    });

    let reset_button =
        h_button("Start over", SvgIcon::Restart, false).on_press(Message::ReceiveStateReset);

    let content = column![header, column![qr_column, reset_button].spacing(16)];

    column![
        // Disable the network switcher once we have an invoice or address
        h_screen_header(harbor, true, true),
        basic_layout(content.spacing(48))
    ]
    .into()
}
