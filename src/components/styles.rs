use iced::{
    font,
    widget::{text, Text},
    Font,
};

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
