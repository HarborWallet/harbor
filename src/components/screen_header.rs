use iced::{
    widget::{column, row, text},
    Alignment, Element, Length,
};
use crate::{HarborWallet, Message};

use super::{format_amount, hr, map_icon, vr, FederationItem, SvgIcon};

pub fn h_screen_header(harbor: &HarborWallet, show_balance: bool) -> Element<Message> {
    if let Some(item) = harbor.active_federation.as_ref() {
        let FederationItem {
            name,
            balance,
            ..
        } = item;
        let people_icon = map_icon(SvgIcon::People, 24., 24.);
        let current_federation = row![people_icon, text(name).size(24)]
            .align_y(Alignment::Center)
            .spacing(16)
            .width(Length::Shrink)
            .padding(16);
        let formatted_balance = format_amount(*balance);

        let balance = row![text(formatted_balance).size(24)]
            .align_y(Alignment::Center)
            .padding(16);

        let row = row![current_federation].spacing(16);

        column![
            row.push_maybe(show_balance.then_some(vr()))
                .push_maybe(show_balance.then_some(balance))
                .height(Length::Shrink),
            hr()
        ]
        .into()
    } else {
        row![].spacing(16).into()
    }
}
