use iced::widget::{button, center, column, container, row, stack, text};
use iced::{Color, Element, Length, Shadow, Theme, Vector};

use crate::Message;

use super::{SvgIcon, h_small_button, light_container_style};

#[derive(Debug, Clone)]
pub struct ConfirmModalState {
    pub title: String,
    pub description: String,
    pub confirm_action: Box<Message>,
    pub cancel_action: Box<Message>,
    pub confirm_button_text: String,
}

impl Default for ConfirmModalState {
    fn default() -> Self {
        Self {
            title: "Confirm Action".to_string(),
            description: "Are you sure you want to proceed?".to_string(),
            confirm_action: Box::new(Message::SetConfirmModal(None)),
            cancel_action: Box::new(Message::SetConfirmModal(None)),
            confirm_button_text: "Confirm".to_string(),
        }
    }
}

pub fn confirm_modal<'a>(
    content: Element<'a, Message>,
    state: Option<&'a ConfirmModalState>,
) -> Element<'a, Message> {
    let mut layers = stack![content];

    if let Some(state) = state {
        let modal_content = container(
            column![
                text(&state.title).size(24),
                text(&state.description),
                row![
                    h_small_button("Cancel", SvgIcon::SmallClose, false)
                        .on_press((*state.cancel_action).clone()),
                    h_small_button(&state.confirm_button_text, SvgIcon::SmallCheck, false)
                        .on_press((*state.confirm_action).clone()),
                ]
                .spacing(10)
            ]
            .spacing(20),
        )
        .width(400)
        .padding(24)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            text_color: Some(theme.palette().text),
            border: light_container_style(theme).border,
            shadow: Shadow {
                color: Color::from_rgba8(0, 0, 0, 0.5),
                offset: Vector::new(4.0, 4.0),
                blur_radius: 8.0,
            },
        });

        // Add the overlay and modal layers
        layers = layers.push(
            // This layer blocks all pointer events from reaching the content below
            button(container(text("")).width(Length::Fill).height(Length::Fill))
                .on_press((*state.cancel_action).clone())
                .style(|_theme: &Theme, _state| button::Style::default()),
        );
        layers = layers.push(
            container(center(modal_content))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme: &Theme| container::Style {
                    background: Some(
                        Color {
                            a: 0.8,
                            ..Color::BLACK
                        }
                        .into(),
                    ),
                    ..container::Style::default()
                }),
        );
    }

    layers.into()
}

#[derive(Debug, Clone)]
pub struct BasicModalState {
    pub title: String,
    pub description: String,
    pub close_action: Box<Message>,
    pub content_renderer: Option<fn(&str) -> Element<'static, Message>>,
    pub content_data: Option<String>,
}

impl Default for BasicModalState {
    fn default() -> Self {
        Self {
            title: "Information".to_string(),
            description: "Modal content goes here.".to_string(),
            close_action: Box::new(Message::SetBasicModal(None)),
            content_renderer: None,
            content_data: None,
        }
    }
}

pub fn basic_modal<'a>(
    content: Element<'a, Message>,
    state: Option<&'a BasicModalState>,
) -> Element<'a, Message> {
    let mut layers = stack![content];

    if let Some(state) = state {
        // Create a header row with title and close button
        let header_row = row![
            text(&state.title).size(24),
            // Use spacer to push close button to the right
            container(text("")).width(Length::Fill),
            h_small_button("", SvgIcon::SmallClose, false).on_press((*state.close_action).clone())
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let mut modal_content_column = column![header_row, text(&state.description),].spacing(20);

        // Add custom content if provided
        if let (Some(renderer), Some(data)) = (&state.content_renderer, &state.content_data) {
            modal_content_column = modal_content_column.push(renderer(data));
        }

        let modal_content = container(modal_content_column)
            .width(400)
            .padding(24)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.palette().background.into()),
                text_color: Some(theme.palette().text),
                border: light_container_style(theme).border,
                shadow: Shadow {
                    color: Color::from_rgba8(0, 0, 0, 0.5),
                    offset: Vector::new(4.0, 4.0),
                    blur_radius: 8.0,
                },
            });

        // Add the overlay and modal layers
        layers = layers.push(
            // This layer blocks all pointer events from reaching the content below
            button(container(text("")).width(Length::Fill).height(Length::Fill))
                .on_press((*state.close_action).clone())
                .style(|_theme: &Theme, _state| button::Style::default()),
        );
        layers = layers.push(
            container(center(modal_content))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme: &Theme| container::Style {
                    background: Some(
                        Color {
                            a: 0.8,
                            ..Color::BLACK
                        }
                        .into(),
                    ),
                    ..container::Style::default()
                }),
        );
    }

    layers.into()
}
