use chrono::{DateTime, Local};
use iced::Color;
use palette::{FromColor, Hsl, rgb::Rgb};

pub fn darken(color: Color, amount: f32) -> Color {
    let mut hsl = to_hsl(color);

    hsl.lightness = if hsl.lightness - amount < 0.0 {
        0.0
    } else {
        hsl.lightness - amount
    };

    from_hsl(hsl)
}

pub fn lighten(color: Color, amount: f32) -> Color {
    let mut hsl = to_hsl(color);

    hsl.lightness = if hsl.lightness + amount > 1.0 {
        1.0
    } else {
        hsl.lightness + amount
    };

    from_hsl(hsl)
}

fn to_hsl(color: Color) -> Hsl {
    Hsl::from_color(Rgb::from(color))
}

fn from_hsl(hsl: Hsl) -> Color {
    Rgb::from_color(hsl).into()
}

pub fn format_timestamp(timestamp: &u64) -> String {
    let signed = timestamp.to_owned() as i64;
    let utc = DateTime::from_timestamp(signed, 0).unwrap();
    let date_time: DateTime<Local> = DateTime::from(utc);
    format!("{}", date_time.format("%m/%d/%Y, %l:%M %P"))
}

pub fn format_amount(amount: u64) -> String {
    if amount == 1 {
        return "1 sat".to_string();
    }
    // https://stackoverflow.com/questions/26998485/is-it-possible-to-print-a-number-formatted-with-thousand-separator-in-rust
    // Rust is a real baby about doing useful things
    let num = amount
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",");

    format!("{num} sats")
}

pub fn truncate_text(input: &str, max_len: usize, center: bool) -> String {
    if input.len() <= max_len {
        input.to_string()
    } else if center {
        // center the elllipses around middle of the string
        format!(
            "{}...{}",
            &input[..(max_len / 2)],
            &input[(input.len() - max_len / 2)..]
        )
    } else {
        format!("{}...", &input[input.len() - max_len..])
    }
}
