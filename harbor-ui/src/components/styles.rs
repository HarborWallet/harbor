use iced::{
    Border, Color, Shadow, Theme,
    overlay::menu,
    widget::{checkbox, container::Style as ContainerStyle, pick_list, text::Style},
};

use super::{darken, lighten};

use iced::{
    Font, font,
    widget::{Text, text},
};

pub fn subtitle(theme: &Theme) -> Style {
    let gray = lighten(theme.palette().background, 0.7);
    Style { color: Some(gray) }
}

pub fn very_subtle(theme: &Theme) -> Style {
    let gray = lighten(theme.palette().background, 0.5);
    Style { color: Some(gray) }
}

pub fn link() -> Color {
    // This is the same theme.pallette().background just without needing `Theme`
    lighten(Color::from_rgb8(23, 23, 25), 0.7)
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

pub fn green() -> Color {
    Color::from_rgb8(40, 164, 127)
}

pub fn red() -> Color {
    Color::from_rgb8(250, 0, 80)
}

pub fn menu_style(theme: &Theme) -> menu::Style {
    let border = Border {
        color: Color::WHITE,
        width: 0.,
        radius: (8.).into(),
    };

    let background = lighten(theme.palette().background, 0.05);
    let selected_background = lighten(theme.palette().background, 0.1);

    menu::Style {
        background: background.into(),
        border,
        selected_background: selected_background.into(),
        selected_text_color: Color::WHITE,
        text_color: Color::WHITE,
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
        pick_list::Status::Opened { .. } => darken(Color::BLACK, 0.1),
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

pub fn borderless_pick_list_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let mut style = pick_list_style(theme, status);
    style.border = Border {
        color: Color::WHITE,
        width: 0.,
        radius: (8.).into(),
    };
    style
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

pub fn tag_style(theme: &Theme) -> ContainerStyle {
    let gray = lighten(theme.palette().background, 0.1);
    let border = Border {
        color: gray,
        width: 0.,
        radius: (4.).into(),
    };

    ContainerStyle {
        text_color: None,
        background: Some(gray.into()),
        border,
        shadow: Shadow::default(),
    }
}

pub fn checkbox_style(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let background = theme.palette().background;
    let gray = lighten(theme.palette().background, 0.5);

    let background = match status {
        checkbox::Status::Hovered { is_checked: _ } => lighten(background, 0.1),
        checkbox::Status::Active { is_checked: _ } => background,
        checkbox::Status::Disabled { is_checked: _ } => background,
    };

    let text_color = match status {
        checkbox::Status::Disabled { .. } => gray,
        checkbox::Status::Active { is_checked: _ } => Color::WHITE,
        checkbox::Status::Hovered { is_checked: _ } => Color::WHITE,
    };

    checkbox::Style {
        icon_color: theme.palette().primary,
        text_color: Some(text_color),
        background: background.into(),
        border: Border {
            color: text_color,
            width: 2.0,
            radius: (8.0).into(),
        },
    }
}

pub fn font_mono() -> Font {
    Font {
        family: font::Family::Monospace,
        weight: font::Weight::Normal,
        stretch: font::Stretch::Normal,
        style: font::Style::Normal,
    }
}
