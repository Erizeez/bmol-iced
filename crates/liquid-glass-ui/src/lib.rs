//! UI-facing component descriptions.
//!
//! Iced widget integration is intentionally the next milestone. These
//! builders establish the component-to-scene contract first.

#![deny(unsafe_code)]

use liquid_glass_scene::{GlassId, GlassMaterial, GlassNode, GlassShape, Rect};

/// A container that can later host any Iced widget tree.
#[derive(Clone, Debug)]
pub struct GlassContainer {
    node: GlassNode,
    padding: f32,
}

impl GlassContainer {
    #[must_use]
    pub fn new(id: GlassId, bounds: Rect) -> Self {
        Self { node: GlassNode::new(id, bounds), padding: 0.0 }
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
