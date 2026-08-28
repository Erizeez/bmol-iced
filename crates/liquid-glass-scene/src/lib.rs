//! Renderer-independent data model for Liquid Glass.
//!
//! This crate deliberately does not depend on `wgpu` or Iced. It is the
//! contract between the UI adapter and the compositor.

#![deny(unsafe_code)]

use std::fmt;

/// A stable identifier for a glass element.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GlassId(pub u64);

/// A logical-pixel rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    #[must_use]
    pub fn expand(self, amount: f32) -> Self {
        Self {
            x: self.x - amount,
            y: self.y - amount,
            width: self.width + amount * 2.0,
            height: self.height + amount * 2.0,
        }
    }
}

/// An RGBA color in the 0..=1 range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    #[must_use]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[must_use]
    pub const fn white() -> Self {
        Self::rgba(1.0, 1.0, 1.0, 1.0)
    }

    #[must_use]
    pub const fn transparent() -> Self {
        Self::rgba(0.0, 0.0, 0.0, 0.0)
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::transparent()
    }
}

/// The curvature model used by radius-based rounded rectangles.
/// Capsules, circles, and ellipses preserve their exact circular geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CornerCurve {
    /// A quarter-circle corner with a curvature discontinuity at the join.
    Circular,
    /// A continuous superellipse corner. Exponents around 5 match the
    /// continuous visual language used by Apple system controls.
    Continuous { exponent: f32 },
}

impl CornerCurve {
    pub const DEFAULT_CONTINUOUS_EXPONENT: f32 = 5.0;

    #[must_use]
    pub const fn continuous() -> Self {
        Self::Continuous { exponent: Self::DEFAULT_CONTINUOUS_EXPONENT }
    }

    #[must_use]
    pub const fn continuous_with_exponent(exponent: f32) -> Self {
        Self::Continuous { exponent }
    }

    #[must_use]
    pub const fn exponent(self) -> f32 {
        match self {
            Self::Circular => 2.0,
            Self::Continuous { exponent } => {
                if exponent >= 2.0 {
                    exponent
                } else {
                    2.0
                }
            }
        }
    }
}

impl Default for CornerCurve {
    fn default() -> Self {
        Self::continuous()
    }
}

/// A shape evaluated by the SDF shader.
#[derive(Clone, Debug, PartialEq)]
pub enum GlassShape {
    RoundedRect { radius: f32 },
    Superellipse { exponent: f32 },
    Capsule,
    Circle,
    Ellipse,
}

impl Default for GlassShape {
    fn default() -> Self {
        Self::RoundedRect { radius: 16.0 }
    }
}

/// Blur configuration used by a glass material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlurStyle {
    pub radius: f32,
    pub edge_blur: bool,
}

/// Refraction configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RefractionStyle {
    pub thickness: f32,
    pub index: f32,
    pub strength: f32,
}

/// Chromatic dispersion configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DispersionStyle {
    pub strength: f32,
    pub spread: f32,
}

/// Fresnel highlight configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FresnelStyle {
    pub range: f32,
    pub hardness: f32,
    pub strength: f32,
}

/// A complete, shape-independent glass material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassMaterial {
    pub blur: BlurStyle,
    pub tint: Color,
    pub refraction: RefractionStyle,
    pub dispersion: DispersionStyle,
    pub fresnel: FresnelStyle,
    pub opacity: f32,
}

impl GlassMaterial {
    #[must_use]
    pub const fn clear() -> Self {
        Self {
            blur: BlurStyle { radius: 10.0, edge_blur: true },
            tint: Color::rgba(1.0, 1.0, 1.0, 0.12),
            refraction: RefractionStyle { thickness: 0.18, index: 1.45, strength: 0.35 },
            dispersion: DispersionStyle { strength: 0.08, spread: 0.02 },
            fresnel: FresnelStyle { range: 0.75, hardness: 0.6, strength: 0.35 },
            opacity: 0.92,
        }
    }

    #[must_use]
    pub const fn regular() -> Self {
        Self { blur: BlurStyle { radius: 20.0, edge_blur: true }, ..Self::clear() }
    }

    #[must_use]
    pub const fn thick() -> Self {
        Self { blur: BlurStyle { radius: 34.0, edge_blur: true }, ..Self::regular() }
    }

    #[must_use]
    pub const fn interactive() -> Self {
        Self {
            refraction: RefractionStyle { strength: 0.5, ..Self::regular().refraction },
            ..Self::regular()
        }
    }
}

impl Default for GlassMaterial {
    fn default() -> Self {
        Self::regular()
    }
}

/// The portion of the already-rendered scene needed by a glass node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropRegion {
    pub bounds: Rect,
    pub padding: f32,
    pub blur_radius: f32,
}

impl BackdropRegion {
    #[must_use]
    pub fn capture_bounds(self) -> Rect {
        self.bounds.expand(self.padding + self.blur_radius)
    }
}

/// A renderer-facing glass node.
#[derive(Clone, Debug, PartialEq)]
pub struct GlassNode {
    pub id: GlassId,
    pub bounds: Rect,
    pub shape: GlassShape,
    pub corner_curve: CornerCurve,
    pub material: GlassMaterial,
    pub backdrop: BackdropRegion,
    pub z_index: i32,
}

impl GlassNode {
    #[must_use]
    pub fn new(id: GlassId, bounds: Rect) -> Self {
        let material = GlassMaterial::default();
        Self {
            id,
            bounds,
            shape: GlassShape::default(),
            corner_curve: CornerCurve::default(),
            backdrop: BackdropRegion { bounds, padding: 8.0, blur_radius: material.blur.radius },
            material,
            z_index: 0,
        }
    }

    #[must_use]
    pub fn shape(mut self, shape: GlassShape) -> Self {
        self.shape = shape;
        self
    }

    #[must_use]
    pub const fn corner_curve(mut self, corner_curve: CornerCurve) -> Self {
        self.corner_curve = corner_curve;
        self
    }

    #[must_use]
    pub fn material(mut self, material: GlassMaterial) -> Self {
        self.backdrop.blur_radius = material.blur.radius;
        self.material = material;
        self
    }
}

/// A scene collection consumed by the Liquid Glass compositor.
#[derive(Clone, Debug, Default)]
pub struct GlassScene {
    nodes: Vec<GlassNode>,
}

impl GlassScene {
    pub fn push(&mut self, node: GlassNode) {
        self.nodes.push(node);
    }

    #[must_use]
    pub fn nodes(&self) -> &[GlassNode] {
        &self.nodes
    }

    /// Returns mutable nodes for compositor adapters that apply a final
    /// viewport transform before rendering.
    pub fn nodes_mut(&mut self) -> &mut [GlassNode] {
        &mut self.nodes
    }

    /// Returns nodes in stable back-to-front order for compositor drawing.
    #[must_use]
    pub fn nodes_in_render_order(&self) -> Vec<&GlassNode> {
        let mut nodes: Vec<_> = self.nodes.iter().collect();
        nodes.sort_by_key(|node| node.z_index);
        nodes
    }

    #[must_use]
    pub fn capture_bounds(&self) -> Option<Rect> {
        self.nodes.iter().map(|node| node.backdrop.capture_bounds()).reduce(union)
    }
}

fn union(a: Rect, b: Rect) -> Rect {
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = (a.x + a.width).max(b.x + b.width);
    let bottom = (a.y + a.height).max(b.y + b.height);
    Rect::new(left, top, right - left, bottom - top)
}

impl fmt::Display for GlassId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "glass-{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_capture_bounds_include_backdrop_padding_and_blur() {
        let node = GlassNode::new(GlassId(1), Rect::new(10.0, 20.0, 100.0, 50.0))
            .material(GlassMaterial::regular());
        let expected = Rect::new(-18.0, -8.0, 156.0, 106.0);

        let mut scene = GlassScene::default();
        scene.push(node);

        assert_eq!(scene.capture_bounds(), Some(expected));
    }

    #[test]
    fn material_change_updates_backdrop_blur_radius() {
        let node = GlassNode::new(GlassId(1), Rect::new(0.0, 0.0, 10.0, 10.0))
            .material(GlassMaterial::thick());

        assert!(
            (node.backdrop.blur_radius - GlassMaterial::thick().blur.radius).abs() < f32::EPSILON
        );
    }

    #[test]
    fn render_order_is_sorted_by_z_index() {
        let mut front = GlassNode::new(GlassId(2), Rect::new(20.0, 20.0, 10.0, 10.0));
        front.z_index = 10;
        let mut back = GlassNode::new(GlassId(1), Rect::new(0.0, 0.0, 10.0, 10.0));
        back.z_index = -1;

        let mut scene = GlassScene::default();
        scene.push(front);
        scene.push(back);

        let order = scene.nodes_in_render_order();

        assert_eq!(order.iter().map(|node| node.id).collect::<Vec<_>>(), [GlassId(1), GlassId(2)]);
    }

    #[test]
    fn glass_nodes_default_to_continuous_corners() {
        let node = GlassNode::new(GlassId(3), Rect::new(0.0, 0.0, 72.0, 36.0));

        assert_eq!(node.corner_curve, CornerCurve::continuous());
        assert!(
            (node.corner_curve.exponent() - CornerCurve::DEFAULT_CONTINUOUS_EXPONENT).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn circular_and_invalid_curves_resolve_safely() {
        assert!((CornerCurve::Circular.exponent() - 2.0).abs() < f32::EPSILON);
        assert!(
            (CornerCurve::continuous_with_exponent(f32::NAN).exponent() - 2.0).abs() < f32::EPSILON
        );
    }
}
