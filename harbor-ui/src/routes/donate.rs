use iced::Element;
use iced::widget::{column, container, scrollable};
use iced::{Length, Padding};

use crate::components::{InputArgs, SvgIcon, h_button, h_header, h_input};
use crate::{HarborWallet, Message};

pub fn donate(harbor: &HarborWallet) -> Element<Message> {
    let header = h_header(
        "Donate",
        "Support Harbor development by donating to the Human Rights Foundation that supports the project.",
    );

    let donate_input = h_input(InputArgs {
        label: "Amount",
        placeholder: "420000",
        value: &harbor.donate_amount_str,
        on_input: Message::DonateAmountChanged,
        numeric: true,
        suffix: Some("sats"),
        ..InputArgs::default()
    });

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
