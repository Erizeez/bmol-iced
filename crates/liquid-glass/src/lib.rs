//! A Rust Liquid Glass UI framework.
//!
//! The facade keeps application code independent from the internal split
//! between UI, scene, compositor, animation, and platform layers.

#![deny(unsafe_code)]

pub use liquid_glass_animation as animation;
pub use liquid_glass_platform as platform;
pub use liquid_glass_render as render;
pub use liquid_glass_scene as scene;
pub use liquid_glass_ui as ui;

pub use animation::{Spring, SpringValue};
pub use render::{
    GpuError, GpuRenderer, GpuSize, LiquidRenderer, RenderGraph, RenderPass, TexturePool,
};
pub use scene::{
    BackdropRegion, Color, CornerCurve, GlassId, GlassMaterial, GlassNode, GlassScene, GlassShape,
    Rect,
};
pub use ui::{
    GlassButton, GlassButtonIcon, GlassChrome, GlassContainer, GlassNavigationControl, GlassRole,
    GlassSegment, GlassSegmentContent, GlassSegmentedControl, UiColorScheme, UiPalette, UiTheme,
};
