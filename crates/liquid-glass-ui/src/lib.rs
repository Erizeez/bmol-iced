//! UI-facing Liquid Glass components.
//!
//! The widgets implement Iced's advanced widget contract while keeping the
//! renderer-facing scene node independent from Iced.

#![deny(unsafe_code)]

mod theme;

pub use theme::{GlassChrome, GlassRole, UiColorScheme, UiPalette, UiTheme};

use iced::advanced::text::Renderer as TextRenderer;
use iced::{
    Background, Border, Color as IcedColor, Event, Length, Pixels, Rectangle, Shadow, Size, Vector,
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
        GlassShape::Superellipse { .. } => node.bounds.width.min(node.bounds.height) * 0.4,
        GlassShape::Capsule => node.bounds.height * 0.5,
        GlassShape::Circle => node.bounds.width.min(node.bounds.height) * 0.5,
        GlassShape::Ellipse => node.bounds.width.min(node.bounds.height) * 0.25,
    }
}

/// Optional icon content for compact glass buttons.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlassButtonIcon {
    Back,
    Forward,
}

/// The first interactive component contract.
#[derive(Clone, Debug)]
pub struct GlassButton {
    label: String,
    icon: Option<GlassButtonIcon>,
    node: GlassNode,
    chrome: GlassChrome,
}

impl GlassButton {
    #[must_use]
    pub fn new(id: GlassId, label: impl Into<String>, bounds: Rect) -> Self {
        Self {
            label: label.into(),
            icon: None,
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
    pub fn shape(mut self, shape: GlassShape) -> Self {
        self.node = self.node.shape(shape);
        self
    }

    #[must_use]
    pub const fn icon(mut self, icon: GlassButtonIcon) -> Self {
        self.icon = Some(icon);
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
        let (content, font, size) = match self.button.icon {
            Some(GlassButtonIcon::Back) => {
                (Renderer::SCROLL_LEFT_ICON.to_string(), Renderer::ICON_FONT, Pixels(14.0))
            }
            Some(GlassButtonIcon::Forward) => {
                (Renderer::SCROLL_RIGHT_ICON.to_string(), Renderer::ICON_FONT, Pixels(14.0))
            }
            None => (self.button.label.clone(), renderer.default_font(), Pixels(16.0)),
        };
        renderer.fill_text(
            advanced::Text {
                content,
                bounds: bounds.size(),
                size,
                line_height: advanced::text::LineHeight::default(),
                font,
                align_x: advanced::text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                shaping: advanced::text::Shaping::Auto,
                wrapping: advanced::text::Wrapping::None,
            },
            bounds.center(),
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

/// Content rendered in one equal-width glass control segment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlassSegmentContent {
    ChevronLeft,
    ChevronRight,
    Glyph(String),
}

impl GlassSegmentContent {
    #[must_use]
    pub fn glyph(value: impl Into<String>) -> Self {
        Self::Glyph(value.into())
    }
}

/// One independently enabled item in a [`GlassSegmentedControl`].
#[derive(Clone, Debug)]
pub struct GlassSegment<Message> {
    content: GlassSegmentContent,
    on_press: Message,
    enabled: bool,
}

impl<Message> GlassSegment<Message> {
    #[must_use]
    pub const fn new(content: GlassSegmentContent, on_press: Message) -> Self {
        Self { content, on_press, enabled: true }
    }

    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub const fn content(&self) -> &GlassSegmentContent {
        &self.content
    }
}

/// A reusable equal-width segmented control backed by one glass surface.
#[derive(Clone, Debug)]
pub struct GlassSegmentedControl<Message> {
    node: GlassNode,
    chrome: GlassChrome,
    segments: Vec<GlassSegment<Message>>,
}

impl<Message> GlassSegmentedControl<Message> {
    /// Builds a control from one or more equal-width segments.
    ///
    /// # Panics
    ///
    /// Panics when `segments` is empty or contains more than 65,535 items.
    #[must_use]
    pub fn new(
        id: GlassId,
        bounds: Rect,
        segments: impl IntoIterator<Item = GlassSegment<Message>>,
    ) -> Self {
        let segments = segments.into_iter().collect::<Vec<_>>();
        assert!(!segments.is_empty(), "a segmented control needs at least one segment");
        assert!(
            u16::try_from(segments.len()).is_ok(),
            "a segmented control supports at most 65535 segments"
        );
        Self {
            node: GlassNode::new(id, bounds)
                .shape(GlassShape::Capsule)
                .material(GlassMaterial::interactive()),
            chrome: GlassChrome::default(),
            segments,
        }
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
    pub const fn node(&self) -> &GlassNode {
        &self.node
    }

    #[must_use]
    pub fn segments(&self) -> &[GlassSegment<Message>] {
        &self.segments
    }

    #[must_use]
    pub fn into_element<Theme, Renderer>(self) -> iced::Element<'static, Message, Theme, Renderer>
    where
        Message: Clone + 'static,
        Theme: 'static,
        Renderer:
            advanced::Renderer + advanced::graphics::geometry::Renderer + TextRenderer + 'static,
    {
        iced::Element::new(GlassSegmentedWidget { control: self, hovered: None, pressed: None })
    }
}

/// Convenience wrapper for the common two-segment back/forward control.
#[derive(Clone, Debug)]
pub struct GlassNavigationControl {
    node: GlassNode,
    chrome: GlassChrome,
    back_enabled: bool,
    forward_enabled: bool,
}

impl GlassNavigationControl {
    #[must_use]
    pub fn new(id: GlassId, bounds: Rect) -> Self {
        Self {
            node: GlassNode::new(id, bounds)
                .shape(GlassShape::Capsule)
                .material(GlassMaterial::interactive()),
            chrome: GlassChrome::default(),
            back_enabled: true,
            forward_enabled: true,
        }
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
    pub const fn back_enabled(mut self, enabled: bool) -> Self {
        self.back_enabled = enabled;
        self
    }

    #[must_use]
    pub const fn forward_enabled(mut self, enabled: bool) -> Self {
        self.forward_enabled = enabled;
        self
    }

    #[must_use]
    pub const fn node(&self) -> &GlassNode {
        &self.node
    }

    #[must_use]
    pub fn into_element<Message, Theme, Renderer>(
        self,
        on_back: Message,
        on_forward: Message,
    ) -> iced::Element<'static, Message, Theme, Renderer>
    where
        Message: Clone + 'static,
        Theme: 'static,
        Renderer:
            advanced::Renderer + advanced::graphics::geometry::Renderer + TextRenderer + 'static,
    {
        GlassSegmentedControl {
            node: self.node,
            chrome: self.chrome,
            segments: vec![
                GlassSegment::new(GlassSegmentContent::ChevronLeft, on_back)
                    .enabled(self.back_enabled),
                GlassSegment::new(GlassSegmentContent::ChevronRight, on_forward)
                    .enabled(self.forward_enabled),
            ],
        }
        .into_element()
    }
}

struct GlassSegmentedWidget<Message> {
    control: GlassSegmentedControl<Message>,
    hovered: Option<usize>,
    pressed: Option<usize>,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for GlassSegmentedWidget<Message>
where
    Message: Clone,
    Renderer: advanced::Renderer + advanced::graphics::geometry::Renderer + TextRenderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fixed(self.control.node.bounds.width),
            Length::Fixed(self.control.node.bounds.height),
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
            Length::Fixed(self.control.node.bounds.width),
            Length::Fixed(self.control.node.bounds.height),
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

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border::default()
                    .rounded(shape_radius(&self.control.node))
                    .width(1.0)
                    .color(iced_color(self.control.chrome.border)),
                shadow: Shadow {
                    color: iced_color(self.control.chrome.shadow),
                    offset: Vector::new(0.0, self.control.chrome.shadow_offset_y),
                    blur_radius: self.control.chrome.shadow_blur,
                },
                snap: true,
            },
            Background::Color(iced_color(self.control.node.material.tint)),
        );

        let segment_count = self.control.segments.len();
        let segment_width = bounds.width / count_as_f32(segment_count);
        if let Some(index) = self.hovered {
            let hover_bounds = segment_hover_bounds(bounds, segment_count, index);
            let overlay = if self.pressed == Some(index) {
                self.control.chrome.pressed_overlay
            } else {
                self.control.chrome.hover_overlay
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: hover_bounds,
                    border: Border::default().rounded(hover_bounds.height * 0.5),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(iced_color(overlay)),
            );
        }

        for divider_index in 1..segment_count {
            if divider_touches_hovered(divider_index, self.hovered) {
                continue;
            }
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x + segment_width * count_as_f32(divider_index) - 0.5,
                        y: bounds.y + 8.0,
                        width: 1.0,
                        height: bounds.height - 16.0,
                    },
                    border: Border::default(),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(iced_color(self.control.chrome.divider)),
            );
        }

        draw_segment_content(
            renderer,
            &self.control.segments,
            self.control.chrome,
            bounds,
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
        let hovered = enabled_segment_at(&self.control.segments, layout.bounds(), cursor);
        let previous_hovered = self.hovered;
        let previous_pressed = self.pressed;
        self.hovered = hovered;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if hovered.is_some() => {
                self.pressed = hovered;
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.pressed.is_some() =>
            {
                let pressed = self.pressed.take();
                shell.capture_event();
                if pressed == hovered
                    && let Some(index) = pressed
                {
                    shell.publish(self.control.segments[index].on_press.clone());
                }
            }
            _ => {}
        }

        if previous_hovered != self.hovered || previous_pressed != self.pressed {
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
        if enabled_segment_at(&self.control.segments, layout.bounds(), cursor).is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn draw_segment_content<Message, Renderer>(
    renderer: &mut Renderer,
    segments: &[GlassSegment<Message>],
    chrome: GlassChrome,
    bounds: Rectangle,
    viewport: Rectangle,
) where
    Renderer: advanced::Renderer + advanced::graphics::geometry::Renderer + TextRenderer,
{
    use advanced::graphics::geometry::{Frame, LineCap, LineJoin, Path, Stroke};

    let segment_width = bounds.width / count_as_f32(segments.len());
    renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
        let mut frame = Frame::new(renderer, bounds.size());
        let center_y = bounds.height * 0.5;

        for (index, segment) in segments.iter().enumerate() {
            let center_x = segment_width * (count_as_f32(index) + 0.5);
            let icon_color = iced_color(segment_text_color(segment, chrome));
            let stroke = Stroke::default()
                .with_color(icon_color)
                .with_width(1.8)
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round);
            let path = match segment.content {
                GlassSegmentContent::ChevronLeft => Some(Path::new(|path| {
                    path.move_to(iced::Point::new(center_x + 2.5, center_y - 5.0));
                    path.line_to(iced::Point::new(center_x - 2.5, center_y));
                    path.line_to(iced::Point::new(center_x + 2.5, center_y + 5.0));
                })),
                GlassSegmentContent::ChevronRight => Some(Path::new(|path| {
                    path.move_to(iced::Point::new(center_x - 2.5, center_y - 5.0));
                    path.line_to(iced::Point::new(center_x + 2.5, center_y));
                    path.line_to(iced::Point::new(center_x - 2.5, center_y + 5.0));
                })),
                GlassSegmentContent::Glyph(_) => None,
            };
            if let Some(path) = path {
                frame.stroke(&path, stroke);
            }
        }
        renderer.draw_geometry(frame.into_geometry());
    });

    for (index, segment) in segments.iter().enumerate() {
        let GlassSegmentContent::Glyph(content) = &segment.content else {
            continue;
        };
        renderer.fill_text(
            advanced::Text {
                content: content.clone(),
                bounds: Size::new(segment_width, bounds.height),
                size: Pixels(14.0),
                line_height: advanced::text::LineHeight::default(),
                font: renderer.default_font(),
                align_x: advanced::text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                shaping: advanced::text::Shaping::Auto,
                wrapping: advanced::text::Wrapping::None,
            },
            iced::Point::new(
                bounds.x + segment_width * (count_as_f32(index) + 0.5),
                bounds.center_y(),
            ),
            iced_color(segment_text_color(segment, chrome)),
            viewport,
        );
    }
}

fn segment_text_color<Message>(
    segment: &GlassSegment<Message>,
    chrome: GlassChrome,
) -> liquid_glass_scene::Color {
    if segment.enabled { chrome.text } else { chrome.disabled_text }
}

fn count_as_f32(value: usize) -> f32 {
    f32::from(u16::try_from(value).expect("segment count was validated during construction"))
}

fn segment_index_at(
    bounds: Rectangle,
    segment_count: usize,
    cursor: mouse::Cursor,
) -> Option<usize> {
    let position = cursor.position_over(bounds)?;
    let segment_width = bounds.width / count_as_f32(segment_count);
    (0..segment_count)
        .find(|index| position.x < bounds.x + segment_width * count_as_f32(index.saturating_add(1)))
}

fn enabled_segment_at<Message>(
    segments: &[GlassSegment<Message>],
    bounds: Rectangle,
    cursor: mouse::Cursor,
) -> Option<usize> {
    segment_index_at(bounds, segments.len(), cursor).filter(|index| segments[*index].enabled)
}

fn divider_touches_hovered(divider_index: usize, hovered: Option<usize>) -> bool {
    hovered.is_some_and(|index| divider_index == index || divider_index == index.saturating_add(1))
}

const SEGMENT_HOVER_INSET: f32 = 3.0;

fn segment_hover_bounds(bounds: Rectangle, segment_count: usize, index: usize) -> Rectangle {
    let segment_width = bounds.width / count_as_f32(segment_count);
    let diameter = (segment_width.min(bounds.height) - SEGMENT_HOVER_INSET * 2.0).max(0.0);
    let segment_center_x = bounds.x + segment_width * (count_as_f32(index) + 0.5);
    Rectangle {
        x: segment_center_x - diameter * 0.5,
        y: bounds.center_y() - diameter * 0.5,
        width: diameter,
        height: diameter,
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
        let cursor = mouse::Cursor::Available(iced::Point::new(12.0, 12.0));
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

    #[test]
    fn compact_button_can_use_a_circle_and_builtin_icon() {
        let button = GlassButton::new(GlassId(9), "Back", Rect::new(0.0, 0.0, 36.0, 36.0))
            .shape(GlassShape::Circle)
            .icon(GlassButtonIcon::Back);

        assert_eq!(button.node().shape, GlassShape::Circle);
        assert_eq!(button.icon, Some(GlassButtonIcon::Back));
    }

    #[test]
    fn navigation_control_uses_one_capsule_glass_node() {
        let control = GlassNavigationControl::new(GlassId(12), Rect::new(0.0, 0.0, 72.0, 36.0));

        assert_eq!(control.node().id, GlassId(12));
        assert_eq!(control.node().shape, GlassShape::Capsule);
        assert!((control.node().bounds.width - 72.0).abs() < f32::EPSILON);
    }

    #[test]
    fn navigation_hover_feedback_is_a_centered_circle() {
        let bounds = Rectangle { x: 10.0, y: 20.0, width: 72.0, height: 36.0 };
        let hover = segment_hover_bounds(bounds, 2, 1);

        assert!((hover.width - hover.height).abs() < f32::EPSILON);
        assert!((hover.width - 30.0).abs() < f32::EPSILON);
        assert!((hover.y - bounds.y - SEGMENT_HOVER_INSET).abs() < f32::EPSILON);
        assert!((hover.center_x() - (bounds.x + bounds.width * 0.75)).abs() < f32::EPSILON);
        assert!((hover.center_y() - bounds.center_y()).abs() < f32::EPSILON);
    }

    #[test]
    fn segmented_control_accepts_more_than_two_items() {
        let control = GlassSegmentedControl::new(
            GlassId(13),
            Rect::new(0.0, 0.0, 108.0, 36.0),
            [
                GlassSegment::new(GlassSegmentContent::ChevronLeft, 1_u8),
                GlassSegment::new(GlassSegmentContent::glyph("⌂"), 2_u8),
                GlassSegment::new(GlassSegmentContent::ChevronRight, 3_u8),
            ],
        );

        assert_eq!(control.segments().len(), 3);
        assert_eq!(control.segments()[1].content(), &GlassSegmentContent::glyph("⌂"));
    }

    #[test]
    fn disabled_segment_does_not_hover_or_hide_dividers() {
        let segments = [
            GlassSegment::new(GlassSegmentContent::ChevronLeft, 1_u8),
            GlassSegment::new(GlassSegmentContent::ChevronRight, 2_u8).enabled(false),
        ];
        let bounds = Rectangle { x: 0.0, y: 0.0, width: 72.0, height: 36.0 };
        let cursor = mouse::Cursor::Available(iced::Point::new(54.0, 18.0));
        let hovered = enabled_segment_at(&segments, bounds, cursor);

        assert_eq!(hovered, None);
        assert!(!divider_touches_hovered(1, hovered));
    }
}
