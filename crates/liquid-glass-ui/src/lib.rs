//! UI-facing Liquid Glass components.
//!
//! The widgets implement Iced's advanced widget contract while keeping the
//! renderer-facing scene node independent from Iced.

#![deny(unsafe_code)]

use iced::{
    Background, Border, Color as IcedColor, Event, Length, Rectangle, Shadow, Size, Vector,
    advanced::{self, Clipboard, Layout, Shell, Widget, layout, mouse, renderer, widget::Tree},
};
use liquid_glass_scene::{GlassId, GlassMaterial, GlassNode, GlassShape, Rect};

/// A container that can later host any Iced widget tree.
#[derive(Clone, Debug)]
pub struct GlassContainer {
    node: GlassNode,
    padding: f32,
    hovered: bool,
}

impl GlassContainer {
    #[must_use]
    pub fn new(id: GlassId, bounds: Rect) -> Self {
        Self { node: GlassNode::new(id, bounds), padding: 0.0, hovered: false }
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

        let material = self.node.material;
        let tint = material.tint;
        let opacity = (tint.a + if self.hovered { 0.06 } else { 0.0 }).min(1.0);
        let border_color =
            IcedColor::from_rgba(1.0, 1.0, 1.0, if self.hovered { 0.34 } else { 0.20 });
        let radius = shape_radius(&self.node);

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border::default().rounded(radius).width(1.0).color(border_color),
                shadow: Shadow {
                    color: IcedColor::from_rgba(0.0, 0.0, 0.0, 0.24),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 18.0,
                },
                snap: true,
            },
            Background::Color(IcedColor::from_rgba(tint.r, tint.g, tint.b, opacity)),
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
}

impl GlassButton {
    #[must_use]
    pub fn new(id: GlassId, label: impl Into<String>, bounds: Rect) -> Self {
        Self {
            label: label.into(),
            node: GlassNode::new(id, bounds)
                .shape(GlassShape::Capsule)
                .material(GlassMaterial::interactive()),
        }
    }

    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[must_use]
    pub const fn node(&self) -> &GlassNode {
        &self.node
    }
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
}
