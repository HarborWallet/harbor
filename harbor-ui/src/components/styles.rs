use iced::{
    overlay::menu,
    widget::{container::Style as ContainerStyle, pick_list, text::Style},
    Border, Color, Shadow, Theme,
};

use super::{darken, lighten};

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

pub fn menu_style(theme: &Theme) -> menu::Style {
    let border = Border {
        color: Color::WHITE,
        width: 1.,
        radius: (8.).into(),
    };

    let background = theme.palette().background;
    let selected_background = lighten(theme.palette().background, 0.05);

    menu::Style {
        background: background.into(),
        border,
        selected_background: selected_background.into(),
        selected_text_color: Color::WHITE,
        text_color: Color::WHITE,
    }
}

pub fn borderless_pick_list_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let background = match status {
        pick_list::Status::Hovered => lighten(theme.palette().background, 0.05),
        pick_list::Status::Opened => darken(Color::BLACK, 0.1),
        _ => theme.palette().background,
    };

    pick_list::Style {
        border: Border {
            color: Color::WHITE,
            width: 0.,
            radius: (8.).into(),
        },
        background: background.into(),
        text_color: Color::WHITE,
        placeholder_color: Color::WHITE,
        handle_color: Color::WHITE,
    }
}

pub fn pick_list_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let border = Border {
        color: Color::WHITE,
        width: 2.,
        radius: (8.).into(),
    };

    let background = match status {
        pick_list::Status::Hovered => lighten(theme.palette().background, 0.05),
        pick_list::Status::Opened => darken(Color::BLACK, 0.1),
        _ => theme.palette().background,
    };

    pick_list::Style {
        border,
        background: background.into(),
        text_color: Color::WHITE,
        placeholder_color: Color::WHITE,
        handle_color: Color::WHITE,
    }
}

pub fn light_container_style(theme: &Theme) -> ContainerStyle {
    let gray = lighten(theme.palette().background, 0.05);
    let border = Border {
        color: gray,
        width: 0.,
        radius: (8.).into(),
    };

    ContainerStyle {
        text_color: None,
        background: Some(gray.into()),
        border,
        shadow: Shadow::default(),
    }
}

pub fn side_panel_style(theme: &Theme) -> ContainerStyle {
    let gray = lighten(theme.palette().background, 0.05);
    let border = Border {
        color: gray,
        width: 0.,
        radius: (0.).into(),
    };

    ContainerStyle {
        text_color: None,
        background: Some(gray.into()),
        border,
        shadow: Shadow::default(),
    }
}
