//! A Rust Liquid Glass UI framework.
//!
//! The facade keeps application code independent from the internal split
//! between UI, scene, compositor, animation, and platform layers.

#![deny(unsafe_code)]

pub use liquid_glass_animation as animation;
pub use liquid_glass_geometry as geometry;
pub use liquid_glass_platform as platform;
pub use liquid_glass_render as render;
pub use liquid_glass_scene as scene;
pub use liquid_glass_ui as ui;

pub use animation::{Spring, SpringValue};
pub use geometry::{
    CapsuleAxis, CornerRadii, CubicBezier, G2Continuity, G2Profile, Path, PathSegment, Point,
    ResolvedCapsule,
};
pub use platform::{
    BackdropError, BackdropFrame, BackdropFrameError, BackdropRequest, BackdropSize,
    BackdropSource, DesktopBackdropProvider, DisplayScale, SidebarBackgroundConfig,
    SidebarBackgroundExtension, WindowConfig,
};
pub use render::{
    GpuError, GpuFrameBatch, GpuRenderer, GpuSize, LiquidRenderer, RenderGraph, RenderPass,
    ScrollEdgeStyle, TexturePool,
};
pub use scene::{
    AdaptiveStyle, BackdropRegion, Color, CornerCurve, GlareStyle, GlassAccessibility,
    GlassEffectContainer, GlassEnvironment, GlassId, GlassInteraction, GlassMaterial, GlassNode,
    GlassRenderOptions, GlassScene, GlassShape, GlassShapeLayer, GlassVariant, Rect, ShadowStyle,
};
pub use ui::{
    GlassButton, GlassButtonIcon, GlassChrome, GlassContainer, GlassForeground,
    GlassForegroundRenderer, GlassNavigationControl, GlassRole, GlassSegment, GlassSegmentContent,
    GlassSegmentedControl, UiColorScheme, UiIcon, UiPalette, UiTheme,
};
