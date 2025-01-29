use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::event::{self, Event};
use iced::{Border, Color, Element, Length, Point, Rectangle, Size, Theme, Vector, mouse, Renderer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    TopRight,
}

pub struct Absolute<'a, Message: 'a> {
    content: Element<'a, Message>,
    overlay: Option<Element<'a, Message>>,
    position: Position,
}

impl<'a, Message: 'a> Absolute<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        overlay: Option<Element<'a, Message>>,
        position: Position,
    ) -> Self {
        Self {
            content: content.into(),
            overlay,
            position,
        }
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Absolute<'a, Message> {
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
        widget::tree::State::new(())
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
        )
    }

    fn children(&self) -> Vec<Tree> {
        let mut children = vec![Tree::new(&self.content)];
        if let Some(overlay) = &self.overlay {
            children.push(Tree::new(overlay));
        }
        children
    }

    fn diff(&self, tree: &mut Tree) {
        let mut children = vec![&self.content];
        if let Some(overlay) = &self.overlay {
            children.push(overlay);
        }
        tree.diff_children(&children);
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget()
            .operate(&mut state.children[0], layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        self.content.as_widget_mut().on_event(
            &mut state.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
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
        if let Some(overlay) = &mut self.overlay {
            let (first, second) = state.children.split_at_mut(1);

            let base = self
                .content
                .as_widget_mut()
                .overlay(&mut first[0], layout, renderer, translation);

            let overlay = overlay::Element::new(Box::new(AbsoluteOverlay {
                content: overlay,
                tree: &mut second[0],
                position: self.position,
                base_layout: layout.bounds(),
                base_position: layout.position(),
            }));

            Some(
                overlay::Group::with_children(base.into_iter().chain(Some(overlay)).collect())
                    .overlay(),
            )
        } else {
            None
        }
    }
}

struct AbsoluteOverlay<'a, 'b, Message> {
    content: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    position: Position,
    base_layout: Rectangle,
    base_position: Point,
}

impl<Message> overlay::Overlay<Message, Theme, Renderer> for AbsoluteOverlay<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds)
            .width(Length::Shrink)
            .height(Length::Shrink);

        let mut node = self.content.as_widget().layout(self.tree, renderer, &limits);

        let translation = match self.position {
            Position::TopRight => Vector::new(
                self.base_layout.width - node.size().width - 24.0,
                24.0,
            ),
        };

        node.move_to(self.base_position + translation)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }

    fn is_over(&self, layout: Layout<'_>, _renderer: &Renderer, cursor_position: Point) -> bool {
        layout.bounds().contains(cursor_position)
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        self.content.as_widget_mut().on_event(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        )
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(self.tree, layout, renderer, Vector::default())
    }
}

impl<'a, Message> From<Absolute<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(absolute: Absolute<'a, Message>) -> Self {
        Element::new(absolute)
    }
} 
