use iced::widget::{column, text};
use iced::Element;

use super::{format_amount, subtitle};
use crate::Message;

pub fn h_balance_display(balance: u64) -> Element<'static, Message> {
    let balance_row = text(format_amount(balance)).size(24);
    let balance_subtitle = text("Your balance").size(18).style(subtitle);
    column![balance_row, balance_subtitle].spacing(4).into()
}
