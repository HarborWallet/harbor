use crate::{HarborWallet, Message, ReceiveStatus};
use harbor_client::db_models::FederationItem;
use iced::{
    widget::{column, horizontal_space, pick_list, rich_text, row, span, text},
    Alignment, Element, Length, Padding,
};

use super::{
    borderless_pick_list_style, format_amount, gray, green, hr, map_icon, menu_style, red, vr,
    SvgIcon,
};

pub fn h_screen_header(
    harbor: &HarborWallet,
    show_balance: bool,
    disable_switcher: bool,
) -> Element<Message> {
    if let Some(item) = harbor.active_federation() {
        let FederationItem { name, .. } = item;
        let people_icon = map_icon(SvgIcon::People, 24., 24.);

        let federation_names: Vec<String> = harbor
            .federation_list
            .iter()
            .map(|f| f.name.clone())
            .collect();

        let is_generating = harbor.receive_status == ReceiveStatus::Generating;

        let federation_list =
            // TODO: find a nicer way to disable pick_list than Noop
            pick_list(federation_names, Some(name.clone()), move |selected_name| {
                if is_generating || disable_switcher {
                    Message::Noop
                } else if let Some(federation) = harbor
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
            .padding(Padding::new(0.).left(16));
        let formatted_balance = format_amount(item.balance);

        let balance = row![text(formatted_balance).size(24)]
            .align_y(Alignment::Center)
            .padding(16);

        let row = row![current_federation].spacing(16);

        let shield_icon = map_icon(SvgIcon::Shield, 16., 16.);
        let shield_alert_icon = map_icon(SvgIcon::ShieldAlert, 16., 16.);
        let tor_enabled = harbor.tor_enabled;
        let secured = if tor_enabled {
            row![
                rich_text([
                    span("Tor ").size(16).color(gray()),
                    span("enabled").size(16).color(green()),
                ]),
                shield_icon
            ]
        } else {
            row![
                rich_text([
                    span("Tor ").size(16).color(gray()),
                    span("disabled").size(16).color(red()),
                ]),
                shield_alert_icon
            ]
        }
        .align_y(Alignment::Center)
        .spacing(8)
        .padding(Padding::new(0.).right(16));

        column![
            row.push_maybe(show_balance.then_some(vr()))
                .push_maybe(show_balance.then_some(balance))
                .push(horizontal_space())
                .push(vr())
                .push(secured)
                .align_y(Alignment::Center)
                .height(Length::Shrink),
            hr()
        ]
        .into()
    } else {
        row![].spacing(16).into()
    }
}
