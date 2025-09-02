// Mostly stolen from https://github.com/iced-rs/iced/blob/master/examples/toast/src/main.rs but I made it pretty
use std::fmt;
use std::time::{Duration, Instant};

use crate::Message;
use iced::Border;
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::event::Event;
use iced::widget::button::Status;
use iced::widget::{button, column, container, horizontal_space, row, text};
use iced::{Alignment, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use iced::{Color, Font, mouse};
use iced::{Shadow, window};

use super::{SvgIcon, darken, lighten, map_icon};

pub const DEFAULT_TIMEOUT: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastStatus {
    #[default]
    Neutral,
    Good,
    Bad,
}

impl ToastStatus {
    pub const ALL: &'static [Self] = &[Self::Neutral, Self::Good, Self::Bad];
}

impl fmt::Display for ToastStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Neutral => "Neutral",
            Self::Good => "Good",
            Self::Bad => "Bad",
        }
        .fmt(f)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Toast {
    pub title: String,
    pub body: Option<String>,
    pub status: ToastStatus,
}

pub struct ToastManager<'a> {
    content: Element<'a, Message>,
    toasts: Vec<Element<'a, Message>>,
    timeout_secs: u64,
    on_close: Box<dyn Fn(usize) -> Message + 'a>,
}

impl<'a> ToastManager<'a> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        toasts: &'a [Toast],
        on_close: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        let toasts = toasts
            .iter()
            .enumerate()
            .map(|(index, toast)| {
                let close_icon = map_icon(SvgIcon::SmallClose, 12., 12.);

                let close_button = button(close_icon)
                    .style(|theme: &Theme, status| {
                        let border = Border {
                            color: Color::WHITE,
                            width: 0.,
                            radius: (4.).into(),
                        };

                        let background = match status {
                            Status::Hovered => darken(theme.palette().background, 0.1),
                            Status::Pressed => darken(Color::BLACK, 0.1),
                            _ => theme.palette().background,
                        };
                        button::Style {
                            background: Some(background.into()),
                            text_color: Color::WHITE,
                            border,
                            shadow: Shadow::default(),
                        }
                    })
                    .padding(6)
                    .width(Length::Fixed(24.))
                    .height(Length::Fixed(24.));

                let body = toast.body.clone().map(text);

                container(column![
                    container(
                        column![
                            row![
                                text(toast.title.as_str()).font(Font {
                                    family: iced::font::Family::default(),
                                    weight: iced::font::Weight::Bold,
                                    stretch: iced::font::Stretch::Normal,
                                    style: iced::font::Style::Normal,
                                }),
                                horizontal_space(),
                                close_button.on_press((on_close)(index))
                            ]
                            .align_y(Alignment::Center),
                        ]
                        .push_maybe(body)
                    )
                    .width(Length::Fill)
                    .padding(16)
                    .style(match toast.status {
                        ToastStatus::Neutral => neutral,
                        ToastStatus::Good => good,
                        ToastStatus::Bad => bad,
                    }),
                ])
                .max_width(256)
                .into()
            })
            .collect();

        Self {
            content: content.into(),
            toasts,
            timeout_secs: DEFAULT_TIMEOUT,
            on_close: Box::new(on_close),
        }
    }

    pub fn timeout(self, seconds: u64) -> Self {
        Self {
            timeout_secs: seconds,
            ..self
        }
    }
}

impl Widget<Message, Theme, Renderer> for ToastManager<'_> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn tag(&self) -> widget::tree::Tag {
        struct Marker;
        widget::tree::Tag::of::<Marker>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Vec::<Option<Instant>>::new())
    }

    fn children(&self) -> Vec<Tree> {
        std::iter::once(Tree::new(&self.content))
            .chain(self.toasts.iter().map(Tree::new))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let instants = tree.state.downcast_mut::<Vec<Option<Instant>>>();

        // Invalidating removed instants to None allows us to remove
        // them here so that diffing for removed / new toast instants
        // is accurate
        instants.retain(Option::is_some);

        match (instants.len(), self.toasts.len()) {
            (old, new) if old > new => {
                instants.truncate(new);
            }
            (old, new) if old < new => {
                instants.extend(std::iter::repeat(Some(Instant::now())).take(new - old));
            }
            _ => {}
        }

        tree.diff_children(
            &std::iter::once(&self.content)
                .chain(self.toasts.iter())
                .collect::<Vec<_>>(),
        );
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds(), &mut |operation| {
            self.content
                .as_widget()
                .operate(&mut state.children[0], layout, renderer, operation);
        });
    }

    fn update(
        &mut self,
        state: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut state.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &state.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let instants = state.state.downcast_mut::<Vec<Option<Instant>>>();

        let (content_state, toasts_state) = state.children.split_at_mut(1);

        let content = self.content.as_widget_mut().overlay(
            &mut content_state[0],
            layout,
            renderer,
            translation,
        );

        let toasts = (!self.toasts.is_empty()).then(|| {
            overlay::Element::new(Box::new(Overlay {
                position: layout.bounds().position() + translation,
                toasts: &mut self.toasts,
                state: toasts_state,
                instants,
                on_close: &self.on_close,
                timeout_secs: self.timeout_secs,
            }))
        });
        let overlays = content.into_iter().chain(toasts).collect::<Vec<_>>();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

struct Overlay<'a, 'b, Message> {
    position: Point,
    toasts: &'b mut [Element<'a, Message>],
    state: &'b mut [Tree],
    instants: &'b mut [Option<Instant>],
    on_close: &'b dyn Fn(usize) -> Message,
    timeout_secs: u64,
}

impl<Message> overlay::Overlay<Message, Theme, Renderer> for Overlay<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds);

        layout::flex::resolve(
            layout::flex::Axis::Vertical,
            renderer,
            &limits,
            Length::Fill,
            Length::Fill,
            10.into(),
            10.0,
            Alignment::End,
            self.toasts,
            self.state,
        )
        .translate(Vector::new(self.position.x, self.position.y))
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if let Event::Window(window::Event::RedrawRequested(now)) = &event {
            let mut next_redraw: Option<window::RedrawRequest> = None;

            self.instants
                .iter_mut()
                .enumerate()
                .for_each(|(index, maybe_instant)| {
                    if let Some(instant) = maybe_instant.as_mut() {
                        let remaining = Duration::from_secs(self.timeout_secs)
                            .saturating_sub(instant.elapsed());

                        if remaining == Duration::ZERO {
                            maybe_instant.take();
                            shell.publish((self.on_close)(index));
                            next_redraw = Some(window::RedrawRequest::NextFrame);
                        } else {
                            let redraw_at = window::RedrawRequest::At(*now + remaining);
                            next_redraw = next_redraw
                                .map(|redraw| redraw.min(redraw_at))
                                .or(Some(redraw_at));
                        }
                    }
                });

            if next_redraw.is_some() {
                shell.request_redraw();
            }
        }

        let viewport = layout.bounds();

        self.toasts
            .iter_mut()
            .zip(self.state.iter_mut())
            .zip(layout.children())
            .zip(self.instants.iter_mut())
            .for_each(|(((child, state), layout), instant)| {
                child.as_widget_mut().update(
                    state, event, layout, cursor, renderer, clipboard, shell, &viewport,
                );

                // If the shell has any messages after update, we should remove the toast
                if !shell.is_empty() {
                    instant.take();
                }
            });
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let viewport = layout.bounds();

        for ((child, state), layout) in self
            .toasts
            .iter()
            .zip(self.state.iter())
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(state, renderer, theme, style, layout, cursor, &viewport);
        }
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.container(None, layout.bounds(), &mut |operation| {
            self.toasts
                .iter()
                .zip(self.state.iter_mut())
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.toasts
            .iter()
            .zip(self.state.iter())
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn is_over(&self, layout: Layout<'_>, _renderer: &Renderer, cursor_position: Point) -> bool {
        layout
            .children()
            .any(|layout| layout.bounds().contains(cursor_position))
    }
}

impl<'a> From<ToastManager<'a>> for Element<'a, Message> {
    fn from(manager: ToastManager<'a>) -> Self {
        Element::new(manager)
    }
}

fn styled(background: Color, border: Color) -> container::Style {
    container::Style {
        background: Some(background.into()),
        text_color: Color::WHITE.into(),
        border: Border {
            color: border,
            width: 1.,
            radius: (4.).into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.25),
            offset: Vector::new(-2., -2.),
            blur_radius: 4.,
        },
    }
}

fn neutral(theme: &Theme) -> container::Style {
    let gray = lighten(theme.palette().background, 0.1);

    styled(gray, gray)
}

fn good(theme: &Theme) -> container::Style {
    let gray = lighten(theme.palette().background, 0.1);
    let green = theme.palette().success;

    styled(gray, green)
}

fn bad(theme: &Theme) -> container::Style {
    let gray = lighten(theme.palette().background, 0.1);
    let red = theme.palette().primary;

    styled(gray, red)
}

// fn primary(theme: &Theme) -> container::Style {
//     let palette = theme.extended_palette();

//     styled(palette.primary.weak)
// }

// fn secondary(theme: &Theme) -> container::Style {
//     let palette = theme.extended_palette();

//     styled(palette.secondary.weak)
// }

// fn success(theme: &Theme) -> container::Style {
//     let palette = theme.extended_palette();

//     styled(palette.success.weak)
// }

// fn danger(theme: &Theme) -> container::Style {
//     let palette = theme.extended_palette();

//     styled(palette.danger.weak)
// }
