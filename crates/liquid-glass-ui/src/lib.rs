//! UI-facing Liquid Glass components.
//!
//! The widgets implement Iced's advanced widget contract while keeping the
//! renderer-facing scene node independent from Iced.

#![deny(unsafe_code)]

mod theme;

pub use theme::{GlassChrome, GlassRole, UiColorScheme, UiPalette, UiTheme};

use iced::advanced::text::Renderer as TextRenderer;
use iced::{
    Background, Border, Color as IcedColor, Event, Length, Pixels, Point, Rectangle, Shadow, Size,
    Vector,
    advanced::{self, Clipboard, Layout, Shell, Widget, layout, mouse, renderer, widget::Tree},
};
use liquid_glass_scene::{GlassId, GlassMaterial, GlassNode, GlassShape, Rect};

/// A container that can later host any Iced widget tree.
#[derive(Clone, Debug)]
pub struct GlassContainer {
    node: GlassNode,
    chrome: GlassChrome,
    padding: f32,
    hovered: bool,
}

impl GlassContainer {
    #[must_use]
    pub fn new(id: GlassId, bounds: Rect) -> Self {
        Self {
            node: GlassNode::new(id, bounds),
            chrome: GlassChrome::default(),
            padding: 0.0,
            hovered: false,
        }
    }

    #[must_use]
    pub fn shape(mut self, shape: GlassShape) -> Self {
        self.node = self.node.shape(shape);
        self
    }

    #[must_use]
    pub fn material(mut self, material: GlassMaterial) -> Self {
        self.node = self.node.material(material);
        self
    }

    #[must_use]
    pub const fn chrome(mut self, chrome: GlassChrome) -> Self {
        self.chrome = chrome;
        self
    }

    #[must_use]
    pub const fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    #[must_use]
    pub const fn node(&self) -> &GlassNode {
        &self.node
    }

    #[must_use]
    pub const fn padding_value(&self) -> f32 {
        self.padding
    }

    /// Returns a scene node using the final bounds computed by Iced layout.
    #[must_use]
    pub fn scene_node_for(&self, bounds: Rectangle) -> GlassNode {
        GlassNode::new(self.node.id, Rect::new(bounds.x, bounds.y, bounds.width, bounds.height))
            .shape(self.node.shape.clone())
            .material(self.node.material)
    }

    /// Runs the widget's Iced layout contract and converts the result into a
    /// renderer-independent scene node.
    #[must_use]
    pub fn layout_scene_node(&mut self, viewport: Size) -> GlassNode {
        let widget: &dyn Widget<(), (), ()> = self;
        let mut tree = Tree::new(widget);
        let limits = layout::Limits::new(Size::ZERO, viewport);
        let layout = <Self as Widget<(), (), ()>>::layout(self, &mut tree, &(), &limits);
        let size = layout.size();
        self.scene_node_for(Rectangle {
            x: self.node.bounds.x,
            y: self.node.bounds.y,
            width: size.width,
            height: size.height,
        })
    }

    /// Converts this widget into an Iced element for any compatible renderer.
    #[must_use]
    pub fn into_element<Message, Theme, Renderer>(
        self,
    ) -> iced::Element<'static, Message, Theme, Renderer>
    where
        Message: 'static,
        Theme: 'static,
        Renderer: advanced::Renderer + 'static,
    {
        iced::Element::new(self)
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for GlassContainer
where
    Renderer: advanced::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.node.bounds.width), Length::Fixed(self.node.bounds.height))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(
            limits,
            Length::Fixed(self.node.bounds.width),
            Length::Fixed(self.node.bounds.height),
        )
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.intersection(viewport).is_none() {
            return;
        }

        let tint = self.node.material.tint;
        let fill =
            if self.hovered { iced_color(self.chrome.hover_overlay) } else { iced_color(tint) };
        let border_color =
            iced_color(if self.hovered { self.chrome.hover_border } else { self.chrome.border });
        let radius = shape_radius(&self.node);

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border::default().rounded(radius).width(1.0).color(border_color),
                shadow: Shadow {
                    color: iced_color(self.chrome.shadow),
                    offset: Vector::new(0.0, self.chrome.shadow_offset_y),
                    blur_radius: self.chrome.shadow_blur,
                },
                snap: true,
            },
            Background::Color(fill),
        );
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        _event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let hovered = cursor.is_over(layout.bounds());
        if hovered != self.hovered {
            self.hovered = hovered;
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn shape_radius(node: &GlassNode) -> f32 {
    match node.shape {
        GlassShape::RoundedRect { radius } => radius,
        GlassShape::Superellipse { .. } => node.bounds.width.min(node.bounds.height) * 0.2,
        GlassShape::Capsule => node.bounds.height * 0.5,
        GlassShape::Circle => node.bounds.width.min(node.bounds.height) * 0.5,
        GlassShape::Ellipse => node.bounds.width.min(node.bounds.height) * 0.25,
    }
}

/// The first interactive component contract.
#[derive(Clone, Debug)]
pub struct GlassButton {
    label: String,
    node: GlassNode,
    chrome: GlassChrome,
}

impl GlassButton {
    #[must_use]
    pub fn new(id: GlassId, label: impl Into<String>, bounds: Rect) -> Self {
        Self {
            label: label.into(),
            node: GlassNode::new(id, bounds)
                .shape(GlassShape::Capsule)
                .material(GlassMaterial::interactive()),
            chrome: GlassChrome::default(),
        }
    }

    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Replaces the material used by the Iced fallback paint and compositor
    /// scene node.
    #[must_use]
    pub fn material(mut self, material: GlassMaterial) -> Self {
        self.node = self.node.material(material);
        self
    }

    #[must_use]
    pub const fn chrome(mut self, chrome: GlassChrome) -> Self {
        self.chrome = chrome;
        self
    }

    #[must_use]
    pub const fn node(&self) -> &GlassNode {
        &self.node
    }

    /// Converts this button into an Iced element that publishes `on_press`
    /// when the left mouse button is released inside its bounds.
    #[must_use]
    pub fn into_element<Message, Theme, Renderer>(
        self,
        on_press: Message,
    ) -> iced::Element<'static, Message, Theme, Renderer>
    where
        Message: Clone + 'static,
        Theme: 'static,
        Renderer: advanced::Renderer + TextRenderer + 'static,
    {
        iced::Element::new(GlassButtonWidget {
            button: self,
            on_press,
            hovered: false,
            pressed: false,
        })
    }
}

struct GlassButtonWidget<Message> {
    button: GlassButton,
    on_press: Message,
    hovered: bool,
    pressed: bool,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for GlassButtonWidget<Message>
where
    Message: Clone,
    Renderer: advanced::Renderer + TextRenderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fixed(self.button.node.bounds.width),
            Length::Fixed(self.button.node.bounds.height),
        )
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(
            limits,
            Length::Fixed(self.button.node.bounds.width),
            Length::Fixed(self.button.node.bounds.height),
        )
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.intersection(viewport).is_none() {
            return;
        }

        let tint = self.button.node.material.tint;
        let fill = if self.pressed {
            iced_color(self.button.chrome.pressed_overlay)
        } else if self.hovered {
            iced_color(self.button.chrome.hover_overlay)
        } else {
            iced_color(tint)
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border::default()
                    .rounded(shape_radius(&self.button.node))
                    .width(1.0)
                    .color(iced_color(if self.hovered {
                        self.button.chrome.hover_border
                    } else {
                        self.button.chrome.border
                    })),
                shadow: Shadow {
                    color: iced_color(self.button.chrome.shadow),
                    offset: Vector::new(0.0, self.button.chrome.shadow_offset_y),
                    blur_radius: self.button.chrome.shadow_blur,
                },
                snap: true,
            },
            Background::Color(fill),
        );
        renderer.fill_text(
            advanced::Text {
                content: self.button.label.clone(),
                bounds: bounds.size(),
                size: Pixels(16.0),
                line_height: advanced::text::LineHeight::default(),
                font: renderer.default_font(),
                align_x: advanced::text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                shaping: advanced::text::Shaping::Auto,
                wrapping: advanced::text::Wrapping::None,
            },
            Point::new(bounds.x, bounds.y),
            iced_color(self.button.chrome.text),
            *viewport,
        );
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let hovered = cursor.is_over(layout.bounds());
        let was_hovered = self.hovered;
        let was_pressed = self.pressed;
        self.hovered = hovered;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if hovered => {
                self.pressed = true;
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if self.pressed => {
                self.pressed = false;
                shell.capture_event();
                if hovered {
                    shell.publish(self.on_press.clone());
                }
            }
            _ => {}
        }

        if was_hovered != self.hovered || was_pressed != self.pressed {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn iced_color(color: liquid_glass_scene::Color) -> IcedColor {
    IcedColor::from_rgba(color.r, color.g, color.b, color.a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_bridge_uses_widget_layout_size_and_origin() {
        let mut container = GlassContainer::new(GlassId(7), Rect::new(40.0, 24.0, 320.0, 180.0));

        let node = container.layout_scene_node(Size::new(960.0, 640.0));

        assert_eq!(node.id, GlassId(7));
        assert_eq!(node.bounds, Rect::new(40.0, 24.0, 320.0, 180.0));
    }

    #[test]
    fn button_publishes_message_on_click_release() {
        let mut button = GlassButtonWidget {
            button: GlassButton::new(GlassId(8), "Apply", Rect::new(0.0, 0.0, 120.0, 48.0)),
            on_press: 42_u8,
            hovered: false,
            pressed: false,
        };
        let mut tree = Tree::new(&button as &dyn Widget<u8, (), ()>);
        let limits = layout::Limits::new(Size::ZERO, Size::new(200.0, 100.0));
        let layout_node = <GlassButtonWidget<u8> as Widget<u8, (), ()>>::layout(
            &mut button,
            &mut tree,
            &(),
            &limits,
        );
        let layout = Layout::new(&layout_node);
        let viewport = Rectangle { x: 0.0, y: 0.0, width: 200.0, height: 100.0 };
        let cursor = mouse::Cursor::Available(Point::new(12.0, 12.0));
        let mut messages = Vec::new();
        {
            let mut shell = Shell::new(&mut messages);
            let mut clipboard = iced::advanced::clipboard::Null;
            let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
            <GlassButtonWidget<u8> as Widget<u8, (), ()>>::update(
                &mut button,
                &mut tree,
                &press,
                layout,
                cursor,
                &(),
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            let release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
            <GlassButtonWidget<u8> as Widget<u8, (), ()>>::update(
                &mut button,
                &mut tree,
                &release,
                layout,
                cursor,
                &(),
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }

        assert_eq!(messages, vec![42]);
        assert!(!button.pressed);
    }
}
