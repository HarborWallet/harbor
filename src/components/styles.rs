use iced::{widget::text::Style, Theme};

use super::lighten;

pub fn subtitle(theme: &Theme) -> Style {
    let gray = lighten(theme.palette().background, 0.5);
    Style { color: Some(gray) }
}
