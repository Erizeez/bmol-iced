//! BMOL Iced: Modern Apple-style Liquid Glass UI framework and window integration for Iced.

#![deny(unsafe_code)]

pub use liquid_glass_animation as animation;
pub use liquid_glass_render as render;
pub use liquid_glass_scene as scene;
pub use liquid_glass_ui as ui;
pub use liquid_rs as render_core;
pub use squircle_rs as geometry;

pub use bmol_designs as designs;
pub use bmol_window_native as native;
pub use bmol_window_platform as platform;
pub use bmol_window_shell as window;

pub use animation::{Spring, SpringValue};
pub use geometry::{
    APPLE_CORNER_SMOOTHING, CornerRadii, CornerSegment, CubicBezier, PathCommand, Point,
    PopoverArrowParams, PopoverArrowPreset, PopoverArrowSide, PopoverSpline, ProcessedCorner,
    SquircleParams, corner_lead_distance, generate_squircle_svg_path, glsl_squircle_sdf_source,
    sd_squircle, squircle_alpha, squircle_border_coverage, squircle_path_commands,
    squircle_popover_path_commands, wgsl_squircle_sdf_source,
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
/// Application-level composition that is not a reusable glass widget: the Dock
/// shelf and the iced-facing window integration.
pub mod dock;
pub mod windowing;

pub use ui::{
    ClarityPolicy, ContextMenu, GlassButton, GlassButtonIcon, GlassChrome, GlassContainer,
    GlassForeground, GlassForegroundRenderer, GlassNavigationControl, GlassPanel, GlassRole,
    GlassSegment, GlassSegmentContent, GlassSegmentedControl, MenuItem, ScrollbarConfig,
    SpringScrollState, SpringScrollView, UiColorScheme, UiCornerStyle, UiIcon, UiPalette, UiTheme,
    glass_panel, next_dynamic_glass_id, popover, spring_scroll_view,
    spring_scroll_view_with_config, view_context_menu,
};
pub use windowing::{
    DEFAULT_WINDOW_CORNER_RADIUS, IcedWindowController, IcedWindowPolicy, WindowCommand,
    WindowDragArea, WindowExpandBehavior,
};

pub use bmol_window_shell::{
    ControlAction, NativeWindowOptions, TrafficLightsEvent, TrafficLightsState, WindowAppearance,
    WindowChromeConfig, WindowChromeMetrics, WindowControlAction, WindowRimConfig,
    WindowShellController, is_system_dark_mode, loyal_drag_bar, setup_native_window,
    traffic_lights, view_traffic_lights_all_inclusive, wrap_border_resizer, wrap_window_rim,
};
