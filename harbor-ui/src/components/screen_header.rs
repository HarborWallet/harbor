use crate::{HarborWallet, Message};
use harbor_client::db_models::FederationItem;
use iced::{
    widget::{column, pick_list, row, text},
    Alignment, Element, Length, Padding,
};

use super::{borderless_pick_list_style, format_amount, hr, map_icon, menu_style, vr, SvgIcon};

pub fn h_screen_header(harbor: &HarborWallet, show_balance: bool) -> Element<Message> {
    if let Some(item) = harbor.active_federation.as_ref() {
        let FederationItem { name, balance, .. } = item;
        let people_icon = map_icon(SvgIcon::People, 24., 24.);

        let federation_names: Vec<String> = harbor
            .federation_list
            .iter()
            .map(|f| f.name.clone())
            .collect();

        let federation_list = pick_list(federation_names, Some(name.clone()), |selected_name| {
            if let Some(federation) = harbor
                .federation_list
                .iter()
                .find(|f| f.name == selected_name)
            {
                Message::ChangeFederation(federation.id)
            } else {
                Message::Noop
            }
        })
        .style(borderless_pick_list_style)
        .padding(Padding::from(16))
        .handle(pick_list::Handle::Arrow {
            size: Some(iced::Pixels(24.)),
        })
        .menu_style(menu_style);

        let current_federation = row![people_icon, federation_list]
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
                .push(vr())
                .push(text("Connection Secured by Tor").size(12))
                .align_y(Alignment::Center)
                .height(Length::Shrink),
            hr()
        ]
        .into()
    } else {
        row![].spacing(16).into()
    }
}
