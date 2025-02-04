use iced::widget::{button, center, column, container, row, stack, text};
use iced::{Color, Element, Length, Shadow, Theme, Vector};

use crate::Message;

use super::{h_small_button, light_container_style, SvgIcon};

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
