use iced::widget::{column, container, scrollable};
use iced::Element;
use iced::{Length, Padding};

use crate::components::{h_button, h_header, h_input, SvgIcon};
use crate::{HarborWallet, Message};

pub fn donate(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header("Donate", "Support Harbor development.");

    let donate_input = h_input(
        "Amount",
        "",
        &harbor.donate_amount_str,
        Message::DonateAmountChanged,
        None,
        false,
        None,
        Some("sats"),
    );

    let donate_button = h_button("Donate", SvgIcon::Heart, false).on_press(Message::Donate);

    let column = column![header, donate_input, donate_button].spacing(48);

    container(scrollable(
        column
            .spacing(48)
            .width(Length::Fill)
            .max_width(512)
            .padding(Padding::new(48.)),
    ))
    .into()
}
