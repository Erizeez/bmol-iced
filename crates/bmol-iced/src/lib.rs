//! BMOL Iced: Modern Apple-style Liquid Glass UI framework and window integration for Iced.

#![deny(unsafe_code)]

pub use liquid_rs as render_core;
pub use liquid_glass_animation as animation;
pub use liquid_glass_geometry as geometry;
pub use liquid_glass_render as render;
pub use liquid_glass_scene as scene;
pub use liquid_glass_ui as ui;

pub use bmol_designs as designs;
pub use bmol_window_shell as window;
pub use bmol_window_platform as platform;
pub use bmol_window_native as native;

pub use animation::{Spring, SpringValue};
pub use geometry::{
    APPLE_CORNER_SMOOTHING, CornerRadii, CornerSegment, CubicBezier, Point, ProcessedCorner,
    SquircleParams, corner_lead_distance, generate_squircle_svg_path, glsl_squircle_sdf_source,
    sd_squircle, squircle_alpha, squircle_border_coverage, squircle_path_commands,
    wgsl_squircle_sdf_source,
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
    TrafficLightStyle,
};
pub use ui::{
    ControlAction, ControlGroup, DEFAULT_WINDOW_CORNER_RADIUS, GlassButton, GlassButtonIcon,
    GlassChrome, GlassContainer, GlassForeground, GlassForegroundRenderer, GlassNavigationControl,
    GlassRole, GlassSegment, GlassSegmentContent, GlassSegmentedControl, IcedWindowController,
    IcedWindowPolicy, ScrollbarConfig, SpringScrollState, SpringScrollView, TrafficLightsAction,
    TrafficLightsState, UiColorScheme, UiCornerStyle, UiIcon, UiPalette, UiTheme,
    WINDOW_CONTROL_GAP, WINDOW_CONTROL_LARGE_GAP, WINDOW_CONTROL_LARGE_SIZE,
    WINDOW_CONTROL_NATIVE_IDS, WINDOW_CONTROL_NATIVE_SIZE, WindowCommand, WindowDragArea,
    WindowExpandBehavior, control_group, positioned_control_group, spring_scroll_view,
    spring_scroll_view_with_config, traffic_lights, view_traffic_lights, window_control,
};

pub use bmol_window_shell::{
    NativeWindowOptions, WindowAppearance, WindowChromeConfig, WindowChromeMetrics, WindowRimConfig,
    WindowShellController, is_system_dark_mode, loyal_drag_bar, setup_native_window,
    wrap_border_resizer, wrap_window_rim,
};
