use crate::components::lighten;
use iced::widget::{horizontal_rule, rule, vertical_rule};
use iced::{Element, Theme};

use crate::Message;

pub fn hr() -> Element<'static, Message> {
    horizontal_rule(1)
        .style(|theme: &Theme| {
            {
                let gray = lighten(theme.palette().background, 0.1);
                // TODO: is there an easier way to just override the color?
                rule::Style {
                    color: gray,
                    width: 1,
                    radius: (0.).into(),
                    fill_mode: rule::FillMode::Full,
                }
            }
        })
        .into()
}

pub fn vr() -> Element<'static, Message> {
    vertical_rule(1)
        .style(|theme: &Theme| {
            {
                let gray = lighten(theme.palette().background, 0.1);
                // TODO: is there an easier way to just override the color?
                rule::Style {
                    color: gray,
                    width: 1,
                    radius: (0.).into(),
                    fill_mode: rule::FillMode::Full,
                }
            }
        })
        .into()
}
