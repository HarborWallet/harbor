use iced::{widget::text::Style, Color, Theme};

use super::lighten;

use iced::{
    font,
    widget::{text, Text},
    Font,
};

pub fn subtitle(theme: &Theme) -> Style {
    let gray = lighten(theme.palette().background, 0.5);
    Style { color: Some(gray) }
}

pub fn link() -> Color {
    // This is the same theme.pallette().background just without needing `Theme`
    lighten(Color::from_rgb8(23, 23, 25), 0.5)
}

const REGULAR_FONT: Font = Font {
    family: font::Family::SansSerif,
    weight: font::Weight::Normal,
    stretch: font::Stretch::Normal,
    style: font::Style::Normal,
};

const BOLD_FONT: Font = Font {
    family: font::Family::SansSerif,
    weight: font::Weight::Bold,
    stretch: font::Stretch::Normal,
    style: font::Style::Normal,
};

pub fn bold_text(content: String, size: u16) -> Text<'static> {
    text(content).font(BOLD_FONT).size(size)
}

pub fn regular_text(content: String, size: u16) -> Text<'static> {
    text(content).font(REGULAR_FONT).size(size)
}

pub fn gray() -> Color {
    lighten(Color::from_rgb8(23, 23, 25), 0.5)
}
