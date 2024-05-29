use iced::{
    widget::{column, row, text},
    Alignment, Element, Length,
};

use crate::{HarborWallet, Message};

use super::{format_amount, hr, map_icon, vr, FederationItem, SvgIcon};

pub fn h_screen_header(harbor: &HarborWallet, show_balance: bool) -> Element<Message> {
    if let Some(item) = harbor.federation_list.first() {
        let FederationItem {
            name,
            id: _,
            balance: _,
        } = item;
        let people_icon = map_icon(SvgIcon::People, 24., 24.);
        let current_federation = row![people_icon, text(name).size(24)]
            .align_items(Alignment::Center)
            .spacing(16)
            .width(Length::Shrink)
            .padding(16);

        let formatted_balance = format_amount(harbor.balance_sats);

        let balance = row![text(formatted_balance).size(24)]
            .align_items(Alignment::Center)
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
