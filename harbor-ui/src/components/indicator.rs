use iced::advanced::{
    Clipboard, Overlay, Shell,
    layout::{self, Layout},
    overlay::{self, Group},
    renderer,
    widget::{Tree, Widget},
};
use iced::widget::container;
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

/// An element to display a widget over another, controlled by a boolean flag.
///
/// # Example
/// ```ignore
/// use crate::components::indicator;
///
/// indicator(
///     "Main content",
///     "Indicator content",
///     indicator::Position::Top,
///     true, // show
/// )
/// ```
pub struct Indicator<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: container::Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    indicator: Element<'a, Message, Theme, Renderer>,
    position: Position,
    show: bool,
    gap: f32,
    padding: f32,
    snap_within_viewport: bool,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme, Renderer> Indicator<'a, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    /// The default padding of an [`Indicator`].
    const DEFAULT_PADDING: f32 = 5.0;

    /// Creates a new [`Indicator`].
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        indicator: impl Into<Element<'a, Message, Theme, Renderer>>,
        position: Position,
        show: bool,
    ) -> Self {
        Indicator {
            content: content.into(),
            indicator: indicator.into(),
            position,
            show,
            gap: 0.0,
            padding: Self::DEFAULT_PADDING,
            snap_within_viewport: true,
            class: Theme::default(),
        }
    }

    /// Sets the gap between the content and its [`Indicator`].
    pub fn gap(mut self, gap: impl Into<f32>) -> Self {
        self.gap = gap.into();
        self
    }

    /// Sets the padding of the [`Indicator`].
    pub fn padding(mut self, padding: impl Into<f32>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets whether the [`Indicator`] is snapped within the viewport.
    pub fn snap_within_viewport(mut self, snap: bool) -> Self {
        self.snap_within_viewport = snap;
        self
    }

    /// Sets the style of the [`Indicator`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> container::Style + 'a) -> Self
    where
        Theme::Class<'a>: From<container::StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as container::StyleFn<'a, Theme>).into();
        self
    }
}

/// The position of the indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// The indicator will appear on the top of the widget.
    #[default]
    Top,
    /// The indicator will appear on the bottom of the widget.
    Bottom,
    /// The indicator will appear on the left of the widget.
    Left,
    /// The indicator will appear on the right of the widget.
    Right,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Indicator<'_, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::new(&self.indicator)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget(), self.indicator.as_widget()]);
    }

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

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
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
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let mut children = tree.children.iter_mut();

        let content = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            viewport,
            translation,
        );

        let indicator = if self.show {
            let indicator_overlay = IndicatorContent {
                position: layout.position() + translation,
                indicator: &self.indicator,
                state: children.next().unwrap(),
                content_bounds: layout.bounds(),
                snap_within_viewport: self.snap_within_viewport,
                positioning: self.position,
                gap: self.gap,
                padding: self.padding,
                class: &self.class,
            };

            Some(overlay::Element::new(Box::new(indicator_overlay)))
        } else {
            None
        };

        if content.is_some() || indicator.is_some() {
            Some(Group::with_children(content.into_iter().chain(indicator).collect()).overlay())
        } else {
            None
        }
    }
}

struct IndicatorContent<'a, 'b, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    position: Point,
    indicator: &'b Element<'a, Message, Theme, Renderer>,
    state: &'b mut Tree,
    content_bounds: Rectangle,
    snap_within_viewport: bool,
    positioning: Position,
    gap: f32,
    padding: f32,
    class: &'b Theme::Class<'a>,
}

impl<Message, Theme, Renderer> Overlay<Message, Theme, Renderer>
    for IndicatorContent<'_, '_, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);

        let indicator_layout = self.indicator.as_widget().layout(
            self.state,
            renderer,
            &layout::Limits::new(
                Size::ZERO,
                self.snap_within_viewport
                    .then(|| viewport.size())
                    .unwrap_or(Size::INFINITY),
            )
            .shrink(iced::Padding::from(self.padding)),
        );

        let bounds = indicator_layout.bounds();
        let x_center = self.position.x + (self.content_bounds.width - bounds.width) / 2.0;
        let y_center = self.position.y + (self.content_bounds.height - bounds.height) / 2.0;

        let mut indicator_bounds = {
            let offset = match self.positioning {
                Position::Top => Vector::new(
                    x_center,
                    self.position.y - bounds.height - self.gap - self.padding,
                ),
                Position::Bottom => Vector::new(
                    x_center,
                    self.position.y + self.content_bounds.height + self.gap + self.padding,
                ),
                Position::Left => Vector::new(
                    self.position.x - bounds.width - self.gap - self.padding,
                    y_center,
                ),
                Position::Right => Vector::new(
                    self.position.x + self.content_bounds.width + self.gap + self.padding,
                    y_center,
                ),
            };

            Rectangle {
                x: offset.x - self.padding,
                y: offset.y - self.padding,
                width: bounds.width + self.padding * 2.0,
                height: bounds.height + self.padding * 2.0,
            }
        };

        if self.snap_within_viewport {
            if indicator_bounds.x < viewport.x {
                indicator_bounds.x = viewport.x;
            } else if viewport.x + viewport.width < indicator_bounds.x + indicator_bounds.width {
                indicator_bounds.x = viewport.x + viewport.width - indicator_bounds.width;
            }

            if indicator_bounds.y < viewport.y {
                indicator_bounds.y = viewport.y;
            } else if viewport.y + viewport.height < indicator_bounds.y + indicator_bounds.height {
                indicator_bounds.y = viewport.y + viewport.height - indicator_bounds.height;
            }
        }

        layout::Node::with_children(
            indicator_bounds.size(),
            vec![indicator_layout.translate(Vector::new(self.padding, self.padding))],
        )
        .translate(Vector::new(indicator_bounds.x, indicator_bounds.y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
    ) {
        let container_style = theme.style(self.class);

        container::draw_background(renderer, &container_style, layout.bounds());

        let defaults = renderer::Style {
            text_color: container_style.text_color.unwrap_or(style.text_color),
        };

        self.indicator.as_widget().draw(
            self.state,
            renderer,
            theme,
            &defaults,
            layout.children().next().unwrap(),
            cursor,
            &Rectangle::with_size(Size::INFINITY),
        );
    }
}

impl<'a, Message, Theme, Renderer> From<Indicator<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: container::Catalog + 'a,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    fn from(indicator: Indicator<'a, Message, Theme, Renderer>) -> Self {
        Self::new(indicator)
    }
}

/// Creates a new [`Indicator`] with the given content and indicator elements.
pub fn indicator<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    indicator: impl Into<Element<'a, Message, Theme, Renderer>>,
    position: Position,
    show: bool,
) -> Indicator<'a, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    Indicator::new(content, indicator, position, show)
}
