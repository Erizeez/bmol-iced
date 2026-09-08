//! Standalone showcase for authentic Apple-style Liquid Glass Context Menus.
//!
//! Fully powered by `bmol-window-shell`:
//! - Complete frameless window shell integration with physical non-client rim
//! - Authentic macOS traffic lights with dynamic symmetric margins (`margin_left == margin_top`)
//! - Interactive 8-direction border resize handles and loyal titlebar drag bar
//! - Native macOS squircle corner clipping and Stage Manager guard
//! - Dual Appearance: Side-by-side demonstration of both Light and Dark physical glass menus
//! - Ultra-Heavy Backdrop Blur (64pt Dual-Kawase equivalent) completely dissolving high-frequency details
//! - Continuous 12.0 pt squircle container curvature matching Apple HIG Liquid Glass specifications
//! - 1px fine edge highlight rim with deep elevation drop shadows (36pt blur)
//! - Interactive right-click popup at cursor location with outside-click dismissal

#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::struct_excessive_bools,
    clippy::many_single_char_names,
    clippy::excessive_precision,
    clippy::unreadable_literal,
    clippy::doc_markdown
)]

use std::time::Instant;

use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Rectangle,
    Shadow, Size, Subscription, Task, Theme, Vector,
    font::Weight,
    mouse,
    widget::{
        button,
        canvas::{self, Canvas, Frame, Geometry, Path},
        column, container, row, space, stack, text,
    },
    window,
};
use bmol_designs::{
    menu_metrics,
    popover_metrics::{PopoverArrowConfig, PopoverArrowEdge, PopoverArrowPreset},
};
use bmol_window_shell::{
    WindowChromeConfig, WindowShellController, is_system_dark_mode, traffic_lights, window_metrics,
};
use liquid_glass::{
    ContextMenu, ControlAction, MenuItem, TrafficLightsState, UiColorScheme, UiIcon, UiTheme,
    geometry::{
        squircle_path_commands, PathCommand, SquircleParams, APPLE_CORNER_SMOOTHING,
    },
    ui::font,
};
use vibrancy_rs::KawasePassPlan;

/// Television color block backgrounds and calibration test patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallpaperStyle {
    #[default]
    TvColorBars,
    TvSmpteSplit,
    TvColorGrid,
    PureWhite,
    PureBlack,
}

impl WallpaperStyle {
    pub const ALL: [Self; 5] = [
        Self::TvColorBars,
        Self::TvSmpteSplit,
        Self::TvColorGrid,
        Self::PureWhite,
        Self::PureBlack,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TvColorBars => "TV 彩条 (8色)",
            Self::TvSmpteSplit => "SMPTE 双层彩条",
            Self::TvColorGrid => "彩色网格色块",
            Self::PureWhite => "Pure White (255)",
            Self::PureBlack => "Pure Black (0)",
        }
    }
}

/// Blur strength presets demonstrating the dramatic difference of heavy frosted glass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlurPreset {
    #[default]
    UltraHeavy64,
    Heavy48,
    Medium32,
    Subtle16,
}

impl BlurPreset {
    pub const ALL: [Self; 4] = [Self::UltraHeavy64, Self::Heavy48, Self::Medium32, Self::Subtle16];

    #[must_use]
    pub const fn radius(self) -> f32 {
        match self {
            Self::UltraHeavy64 => 64.0,
            Self::Heavy48 => 48.0,
            Self::Medium32 => 32.0,
            Self::Subtle16 => 16.0,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::UltraHeavy64 => "64pt (Ultra-Heavy HIG)",
            Self::Heavy48 => "48pt (Heavy)",
            Self::Medium32 => "32pt (Medium)",
            Self::Subtle16 => "16pt (Subtle)",
        }
    }
}

/// Appearance mode for the floating context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FloatingAppearance {
    #[default]
    FollowTheme,
    ForceLight,
    ForceDark,
}

impl FloatingAppearance {
    pub const ALL: [Self; 3] = [Self::FollowTheme, Self::ForceLight, Self::ForceDark];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::FollowTheme => "Auto (System)",
            Self::ForceLight => "Always Light",
            Self::ForceDark => "Always Dark",
        }
    }

    #[must_use]
    pub const fn resolve(self, global_scheme: UiColorScheme) -> UiColorScheme {
        match self {
            Self::FollowTheme => global_scheme,
            Self::ForceLight => UiColorScheme::Light,
            Self::ForceDark => UiColorScheme::Dark,
        }
    }
}

/// Actions emitted by the demo context menus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    NewWindow,
    QuickLook,
    Cut,
    Copy,
    Paste,
    ToggleStatusBar,
    ToggleLineNumbers,
    ToggleWordWrap,
    OpenSettings,
    SearchHelp,
    AboutApp,
    DeleteDestructive,
}

impl MenuAction {
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::NewWindow => "New Window (⌘N)",
            Self::QuickLook => "Quick Look (Space)",
            Self::Cut => "Cut (⌘X)",
            Self::Copy => "Copy (⌘C)",
            Self::Paste => "Paste (⌘V)",
            Self::ToggleStatusBar => "Toggle Status Bar",
            Self::ToggleLineNumbers => "Toggle Line Numbers",
            Self::ToggleWordWrap => "Toggle Word Wrap",
            Self::OpenSettings => "Open Settings (⌘,)",
            Self::SearchHelp => "Search Help (⇧⌘F)",
            Self::AboutApp => "About This Application",
            Self::DeleteDestructive => "Move to Trash (⌫ - Destructive)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragTarget {
    LightCard,
    DarkCard,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Window Shell integration messages
    WindowOpened(window::Id),
    WindowResized(Size),
    WindowEvent((window::Id, window::Event)),
    DragWindow,
    ToggleMaximize,
    ResizeWindow(window::Direction),
    WindowControl(ControlAction),
    TrafficLightsHover(bool),
    TrafficLightsPressStart(usize),
    TrafficLightsPressCancel(usize),
    TrafficLightsPressEnd(usize),
    AnimationFrame(Instant),

    // Context menu and playground messages
    RightClicked,
    CursorMoved(Point),
    DismissFloatingMenu,
    OpenFloatingMenuAt(Point),
    TriggerAction(MenuAction),
    SetWallpaper(WallpaperStyle),
    SetBlurPreset(BlurPreset),
    SetFloatingAppearance(FloatingAppearance),
    ToggleColorScheme,
    ToggleCalibrationGrid,
    SetPopoverArrow(PopoverArrowEdge),
    ToggleMenuPreset,
    CycleArrowPreset,

    // Draggable menu cards
    StartDragCard(DragTarget),
    EndDragCard,
    ResetCardPositions,
}

/// Menu layout preset: full showcase vs. 1:1 pixel-perfect user reference dock menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuContentPreset {
    FullShowcase,
    #[default]
    DockReference1To1,
}

impl MenuContentPreset {
    #[must_use]
    pub const fn menu_size(self) -> (f32, f32) {
        match self {
            Self::FullShowcase => (demo_metrics::MENU_WIDTH, demo_metrics::MENU_HEIGHT),
            Self::DockReference1To1 => (demo_metrics::DOCK_REF_WIDTH, demo_metrics::DOCK_REF_HEIGHT),
        }
    }
}

/// Layout metrics strictly governing the context menu cards and floating popups in the demo.
pub mod demo_metrics {
    use bmol_designs::menu_metrics;

    /// Drag header grip pill fixed height (38.0 pt).
    pub const DRAG_HEADER_HEIGHT: f32 = 38.0;

    /// Gap between drag header and the context menu container (6.0 pt).
    pub const HEADER_MENU_GAP: f32 = 6.0;

    /// Vertical distance from card position (top-left of drag handle) to the menu container.
    pub const MENU_Y_INSET: f32 = DRAG_HEADER_HEIGHT + HEADER_MENU_GAP;

    /// Fixed width of the full showcase context menu container (220.0 pt).
    pub const MENU_WIDTH: f32 = menu_metrics::DEFAULT_WIDTH;

    /// Corner radius of the context menu container (strictly 12.0 pt continuous squircle).
    pub const MENU_CORNER_RADIUS: f32 = menu_metrics::CONTAINER_CORNER_RADIUS;

    /// Full showcase context menu exact physical height:
    /// - 5 Section headers: 5 * 18.0 = 90.0 pt
    /// - 13 Action/Check/Submenu items: 13 * 24.0 = 312.0 pt
    /// - 4 Separators: 4 * (1.0 + 5.0 * 2) = 44.0 pt
    /// - 21 column item gaps: 21 * 1.0 = 21.0 pt
    /// - Container top & bottom padding: 5.0 * 2 = 10.0 pt
    /// - Total exact height: 90 + 312 + 44 + 21 + 10 = 477.0 pt.
    pub const MENU_HEIGHT: f32 = 477.0;

    /// 1:1 macOS Dock reference menu width matching user screenshot: 154.0 pt (308 px @2x).
    pub const DOCK_REF_WIDTH: f32 = 154.0;

    /// 1:1 macOS Dock reference menu exact physical height matching ContextMenu layout geometry (121.0 pt):
    /// - 4 Items (选项, 显示所有窗口, 隐藏, 退出): 4 * 24.0 = 96.0 pt
    /// - 1 Separator: 1.0 + 5.0 * 2 = 11.0 pt
    /// - 4 item gaps (Column spacing 1.0): 4 * 1.0 = 4.0 pt
    /// - Container top & bottom padding: 5.0 * 2 = 10.0 pt
    /// - Total exact height: 96 + 11 + 4 + 10 = 121.0 pt.
    pub const DOCK_REF_HEIGHT: f32 = 121.0;

    /// 1:1 macOS Dock reference arrow left anchor offset: 27.0 pt / 154.0 pt ≈ 0.175.
    pub const DOCK_REF_ARROW_OFFSET: f32 = 27.0 / 154.0;
}

/// An occlusion region where a context menu card or popup overlays the wallpaper,
/// requiring genuine continuous backdrop blur spatial convolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuOcclusion {
    pub bounds: Rectangle,
    pub corner_radius: f32,
    pub arrow: PopoverArrowConfig,
    pub is_dark: bool,
}

/// High-precision approximation of the error function erf(x).
/// Maximum error < 1.5e-7 (Abramowitz and Stegun formula 7.1.26).
#[inline]
fn approx_erf(x: f32) -> f32 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let abs_x = x.abs();

    if abs_x > 4.0 {
        return sign;
    }

    let p = 0.3275911;
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;

    let t = 1.0 / (1.0 + p * abs_x);
    let poly = ((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t;
    sign * (1.0 - poly * (-abs_x * abs_x).exp())
}

/// Standard normal cumulative distribution function Phi(z) = P(Z <= z).
#[inline]
fn normal_cdf(z: f32) -> f32 {
    0.5 * (1.0 + approx_erf(z * std::f32::consts::FRAC_1_SQRT_2))
}

/// Evaluates genuine continuous Gaussian / Dual-Kawase convolution for 1D horizontal segments.
///
/// For an interval [x_left, x_right] with uniform color, the Gaussian convolution integral at x is:
/// weight = Phi((x_right - x) / sigma) - Phi((x_left - x) / sigma).
///
/// This provides 100% artifact-free, aliasing-free continuous blur without sparse sampling spikes.
#[inline]
fn segment_gaussian_weight(x: f32, x_left: f32, x_right: f32, inv_sigma: f32) -> f32 {
    let cdf_right = normal_cdf((x_right - x) * inv_sigma);
    let cdf_left = normal_cdf((x_left - x) * inv_sigma);
    (cdf_right - cdf_left).max(0.0)
}

/// Continuous analytical Gaussian convolution for 8 TV color bars.
fn sample_blurred_tv_bars(x: f32, bounds_width: f32, blur_radius: f32) -> [f32; 3] {
    const TV_BARS: [[f32; 3]; 8] = [
        [1.0, 1.0, 1.0], // 0: 白 (255, 255, 255)
        [1.0, 1.0, 0.0], // 1: 黄 (255, 255, 0)
        [0.0, 1.0, 1.0], // 2: 青 (0, 255, 255)
        [0.0, 1.0, 0.0], // 3: 绿 (0, 255, 0)
        [1.0, 0.0, 1.0], // 4: 洋红 (255, 0, 255)
        [1.0, 0.0, 0.0], // 5: 红 (255, 0, 0)
        [0.0, 0.0, 1.0], // 6: 蓝 (0, 0, 255)
        [0.0, 0.0, 0.0], // 7: 黑 (0, 0, 0)
    ];

    let n = 8.0f32;
    let bar_w = bounds_width / n;
    let sigma = (blur_radius * 0.40).max(1.0);
    let inv_sigma = 1.0 / sigma;

    let mut r = 0.0f32;
    let mut g = 0.0f32;
    let mut b = 0.0f32;

    for (i, &color) in TV_BARS.iter().enumerate() {
        let left = i as f32 * bar_w;
        let right = (i + 1) as f32 * bar_w;
        let w = segment_gaussian_weight(x, left, right, inv_sigma);
        r += color[0] * w;
        g += color[1] * w;
        b += color[2] * w;
    }

    [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
}

/// Continuous analytical Gaussian convolution for two-tier SMPTE bars.
fn sample_blurred_tv_smpte(x: f32, y: f32, bounds: Size, blur_radius: f32) -> [f32; 3] {
    const TOP_SMPTE: [[f32; 3]; 7] = [
        [1.0, 1.0, 1.0], // 白
        [1.0, 1.0, 0.0], // 黄
        [0.0, 1.0, 1.0], // 青
        [0.0, 1.0, 0.0], // 绿
        [1.0, 0.0, 1.0], // 洋红
        [1.0, 0.0, 0.0], // 红
        [0.0, 0.0, 1.0], // 蓝
    ];

    const BOT_SMPTE: [[f32; 3]; 8] = [
        [0.0, 0.0, 1.0], // 蓝
        [0.0, 0.0, 0.0], // 黑
        [1.0, 0.0, 1.0], // 洋红
        [0.0, 0.0, 0.0], // 黑
        [0.0, 1.0, 1.0], // 青
        [0.0, 0.0, 0.0], // 黑
        [1.0, 1.0, 1.0], // 白
        [0.0, 0.0, 0.0], // 黑
    ];

    let top_h = bounds.height * 0.70;
    let sigma = (blur_radius * 0.40).max(1.0);
    let inv_sigma = 1.0 / sigma;

    // Vertical blending between top 70% and bottom 30%
    let top_vertical_w = segment_gaussian_weight(y, -2.0 * sigma, top_h, inv_sigma);
    let bot_vertical_w = segment_gaussian_weight(y, top_h, bounds.height + 2.0 * sigma, inv_sigma);
    let v_sum = (top_vertical_w + bot_vertical_w).max(1e-5);
    let norm_top_w = top_vertical_w / v_sum;
    let norm_bot_w = bot_vertical_w / v_sum;

    let top_bar_w = bounds.width / 7.0;
    let mut top_r = 0.0f32;
    let mut top_g = 0.0f32;
    let mut top_b = 0.0f32;
    for (i, &color) in TOP_SMPTE.iter().enumerate() {
        let left = i as f32 * top_bar_w;
        let right = (i + 1) as f32 * top_bar_w;
        let w = segment_gaussian_weight(x, left, right, inv_sigma);
        top_r += color[0] * w;
        top_g += color[1] * w;
        top_b += color[2] * w;
    }

    let bot_bar_w = bounds.width / 8.0;
    let mut bot_r = 0.0f32;
    let mut bot_g = 0.0f32;
    let mut bot_b = 0.0f32;
    for (i, &color) in BOT_SMPTE.iter().enumerate() {
        let left = i as f32 * bot_bar_w;
        let right = (i + 1) as f32 * bot_bar_w;
        let w = segment_gaussian_weight(x, left, right, inv_sigma);
        bot_r += color[0] * w;
        bot_g += color[1] * w;
        bot_b += color[2] * w;
    }

    [
        (top_r * norm_top_w + bot_r * norm_bot_w).clamp(0.0, 1.0),
        (top_g * norm_top_w + bot_g * norm_bot_w).clamp(0.0, 1.0),
        (top_b * norm_top_w + bot_b * norm_bot_w).clamp(0.0, 1.0),
    ]
}

/// Continuous analytical 2D Gaussian convolution for 4x3 color grid.
fn sample_blurred_tv_grid(x: f32, y: f32, bounds: Size, blur_radius: f32) -> [f32; 3] {
    const GRID_PALETTE: [[f32; 3]; 12] = [
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.5, 0.0],
        [0.5, 0.0, 0.5],
        [0.0, 0.5, 0.5],
        [0.5, 0.5, 0.5],
    ];

    let cols = 4.0f32;
    let rows = 3.0f32;
    let cell_w = bounds.width / cols;
    let cell_h = bounds.height / rows;
    let sigma = (blur_radius * 0.40).max(1.0);
    let inv_sigma = 1.0 / sigma;

    let mut r = 0.0f32;
    let mut g = 0.0f32;
    let mut b = 0.0f32;

    for row_idx in 0..3 {
        let top = row_idx as f32 * cell_h;
        let bot = (row_idx + 1) as f32 * cell_h;
        let v_weight = segment_gaussian_weight(y, top, bot, inv_sigma);

        for col_idx in 0..4 {
            let left = col_idx as f32 * cell_w;
            let right = (col_idx + 1) as f32 * cell_w;
            let h_weight = segment_gaussian_weight(x, left, right, inv_sigma);
            let total_w = v_weight * h_weight;

            let color = GRID_PALETTE[row_idx * 4 + col_idx];
            r += color[0] * total_w;
            g += color[1] * total_w;
            b += color[2] * total_w;
        }
    }

    [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
}

/// Continuous analytical blurred wallpaper color at (x, y).
fn sample_analytical_blurred_wallpaper(
    style: WallpaperStyle,
    x: f32,
    y: f32,
    bounds: Size,
    blur_radius: f32,
) -> Color {
    match style {
        WallpaperStyle::PureWhite => Color::WHITE,
        WallpaperStyle::PureBlack => Color::BLACK,
        WallpaperStyle::TvColorBars => {
            let rgb = sample_blurred_tv_bars(x, bounds.width, blur_radius);
            Color::from_rgb(rgb[0], rgb[1], rgb[2])
        }
        WallpaperStyle::TvSmpteSplit => {
            let rgb = sample_blurred_tv_smpte(x, y, bounds, blur_radius);
            Color::from_rgb(rgb[0], rgb[1], rgb[2])
        }
        WallpaperStyle::TvColorGrid => {
            let rgb = sample_blurred_tv_grid(x, y, bounds, blur_radius);
            Color::from_rgb(rgb[0], rgb[1], rgb[2])
        }
    }
}

/// Builds an authentic Apple continuous curvature squircle path (G2 continuity) for a rectangle
/// using our dedicated `squircle-rs` (`liquid_glass::geometry`) library.
fn build_squircle_path(rect: Rectangle, radius: f32) -> Path {
    let r = radius.min(rect.width * 0.5).min(rect.height * 0.5);
    let params = SquircleParams::new(rect.width, rect.height, r)
        .with_smoothing(APPLE_CORNER_SMOOTHING);
    let commands = squircle_path_commands(&params);

    Path::new(move |b| {
        for cmd in &commands {
            match *cmd {
                PathCommand::MoveTo(pt) => b.move_to(Point::new(rect.x + pt.x, rect.y + pt.y)),
                PathCommand::LineTo(pt) => b.line_to(Point::new(rect.x + pt.x, rect.y + pt.y)),
                PathCommand::CubicTo { c0, c1, to } => b.bezier_curve_to(
                    Point::new(rect.x + c0.x, rect.y + c0.y),
                    Point::new(rect.x + c1.x, rect.y + c1.y),
                    Point::new(rect.x + to.x, rect.y + to.y),
                ),
                PathCommand::Close => b.close(),
            }
        }
    })
}

/// Builds an authentic Apple continuous curvature squircle path with an integrated smooth
/// popover arrow / beak (触角) on the designated edge.
///
/// Builds an authentic Apple continuous curvature squircle path with an integrated smooth
/// popover arrow / beak (触角) on the designated edge.
///
/// If `arrow.edge == PopoverArrowEdge::None` or `!arrow.is_visible()`, strictly defaults
/// to standard `build_squircle_path`.
fn build_popover_squircle_path(
    rect: Rectangle,
    radius: f32,
    arrow: PopoverArrowConfig,
) -> Path {
    if !arrow.is_visible() {
        return build_squircle_path(rect, radius);
    }

    let r = radius.min(rect.width * 0.5).min(rect.height * 0.5);
    let params = SquircleParams::new(rect.width, rect.height, r)
        .with_smoothing(APPLE_CORNER_SMOOTHING);
    let commands = squircle_path_commands(&params);

    let w = rect.width;
    let h = rect.height;
    let rx = rect.x;
    let ry = rect.y;

    let bw = arrow.base_width.min(w * 0.45).min(h * 0.45);
    let ha = arrow.height;
    let wb = bw * 0.5;
    let rt = arrow.tip_radius.clamp(0.5, wb * 0.6);
    let rf = arrow.base_fillet.clamp(0.5, wb * 0.6);

    // Parametric C1 Bezier control nodes dynamically adapting to tip_radius and base_fillet:
    let u_apex_ctrl = rt * 0.4;
    let v_inflect = ha * 0.58;
    let u_inflect = (rt * 0.7 + wb * 0.25).min(wb * 0.65);
    let u_base_ctrl = wb - rf * 0.45;
    let u_lower_slope = u_inflect + (wb - u_inflect) * 0.35;
    let v_lower_slope = v_inflect * 0.58;
    let u_upper_slope = u_inflect * 0.65;
    let v_upper_slope = v_inflect + (ha - v_inflect) * 0.6;

    Path::new(move |b| {
        let draw_arrow = |b: &mut iced::widget::canvas::path::Builder, map: &dyn Fn(f32, f32) -> Point| {
            // 1. Line to start of arrow at baseline (-wb, 0.0)
            b.line_to(map(-wb, 0.0));
            // 2. Base entry fillet: horizontal tangent from straight edge into lower flank
            b.bezier_curve_to(
                map(-u_base_ctrl, 0.0),
                map(-u_lower_slope, v_lower_slope),
                map(-u_inflect, v_inflect),
            );
            // 3. Upper flank into apex dome: smoothly curves towards broad horizontal crest
            b.bezier_curve_to(
                map(-u_upper_slope, v_upper_slope),
                map(-u_apex_ctrl, ha),
                map(0.0, ha),
            );
            // 4. Crest descent into downward flank: perfectly horizontal tangent at apex (0, ha)
            b.bezier_curve_to(
                map(u_apex_ctrl, ha),
                map(u_upper_slope, v_upper_slope),
                map(u_inflect, v_inflect),
            );
            // 5. Base exit fillet: smooth C1 tangent transition back to card baseline
            b.bezier_curve_to(
                map(u_lower_slope, v_lower_slope),
                map(u_base_ctrl, 0.0),
                map(wb, 0.0),
            );
        };

        match arrow.edge {
            PopoverArrowEdge::Top => {
                let xc = (rx + w * arrow.offset).clamp(rx + r + wb, rx + w - r - wb);
                let map = |u: f32, v: f32| Point::new(xc + u, ry - v);
                for (i, cmd) in commands.iter().enumerate() {
                    if i == commands.len() - 1 {
                        draw_arrow(b, &map);
                        if let PathCommand::MoveTo(pt) = commands[0] {
                            b.line_to(Point::new(rx + pt.x, ry + pt.y));
                        }
                        b.close();
                    } else {
                        emit_cmd(b, rx, ry, cmd);
                    }
                }
            }
            PopoverArrowEdge::Bottom => {
                let xc = (rx + w * arrow.offset).clamp(rx + r + wb, rx + w - r - wb);
                let map = |u: f32, v: f32| Point::new(xc - u, ry + h + v);
                for cmd in &commands {
                    if let PathCommand::LineTo(pt) = *cmd {
                        if (pt.y - h).abs() < 0.1 && pt.x < w * 0.5 {
                            draw_arrow(b, &map);
                            b.line_to(Point::new(rx + pt.x, ry + pt.y));
                            continue;
                        }
                    }
                    emit_cmd(b, rx, ry, cmd);
                }
            }
            PopoverArrowEdge::Left => {
                let yc = (ry + h * arrow.offset).clamp(ry + r + wb, ry + h - r - wb);
                let map = |u: f32, v: f32| Point::new(rx - v, yc - u);
                for cmd in &commands {
                    if let PathCommand::LineTo(pt) = *cmd {
                        if pt.x.abs() < 0.1 && pt.y < h * 0.5 {
                            draw_arrow(b, &map);
                            b.line_to(Point::new(rx + pt.x, ry + pt.y));
                            continue;
                        }
                    }
                    emit_cmd(b, rx, ry, cmd);
                }
            }
            PopoverArrowEdge::Right => {
                let yc = (ry + h * arrow.offset).clamp(ry + r + wb, ry + h - r - wb);
                let map = |u: f32, v: f32| Point::new(rx + w + v, yc + u);
                for cmd in &commands {
                    if let PathCommand::LineTo(pt) = *cmd {
                        if (pt.x - w).abs() < 0.1 && pt.y > h * 0.5 {
                            draw_arrow(b, &map);
                            b.line_to(Point::new(rx + pt.x, ry + pt.y));
                            continue;
                        }
                    }
                    emit_cmd(b, rx, ry, cmd);
                }
            }
            PopoverArrowEdge::None => {
                for cmd in &commands {
                    emit_cmd(b, rx, ry, cmd);
                }
            }
        }
    })
}

#[inline]
fn emit_cmd(b: &mut iced::widget::canvas::path::Builder, rx: f32, ry: f32, cmd: &PathCommand) {
    match *cmd {
        PathCommand::MoveTo(pt) => b.move_to(Point::new(rx + pt.x, ry + pt.y)),
        PathCommand::LineTo(pt) => b.line_to(Point::new(rx + pt.x, ry + pt.y)),
        PathCommand::CubicTo { c0, c1, to } => b.bezier_curve_to(
            Point::new(rx + c0.x, ry + c0.y),
            Point::new(rx + c1.x, ry + c1.y),
            Point::new(rx + to.x, ry + to.y),
        ),
        PathCommand::Close => b.close(),
    }
}

/// Fills an authentic Apple squircle/popover on the frame using continuous curvature.
fn fill_popover(
    frame: &mut Frame,
    rect: Rectangle,
    radius: f32,
    arrow: PopoverArrowConfig,
    color: Color,
) {
    if rect.width <= 0.0 || rect.height <= 0.0 || color.a <= 0.001 {
        return;
    }
    let path = build_popover_squircle_path(rect, radius, arrow);
    frame.fill(&path, color);
}

/// Strokes a continuous 1px fine edge highlight around the entire popover squircle + arrow rim.
fn stroke_popover_rim(
    frame: &mut Frame,
    rect: Rectangle,
    radius: f32,
    arrow: PopoverArrowConfig,
    color: Color,
    width: f32,
) {
    if rect.width <= 0.0 || rect.height <= 0.0 || color.a <= 0.001 {
        return;
    }
    let path = build_popover_squircle_path(rect, radius, arrow);
    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_color(color)
            .with_width(width),
    );
}

/// Renders multi-layer soft drop shadows matching macOS Popover Window shadow geometry,
/// continuously wrapping both the squircle menu body and the popover arrow (触角).
fn render_soft_menu_shadow(
    frame: &mut Frame,
    occ: &MenuOcclusion,
) {
    let rect = occ.bounds;
    let corner_radius = occ.corner_radius;
    let arrow = occ.arrow;
    let is_dark = occ.is_dark;

    // 1. Ambient Contact Shadow: Tight, soft ground contact (macOS HIG elevation = 4pt)
    let ambient_tiers = 8;
    let ambient_max_spread = 10.0f32;
    let base_ambient_alpha = if is_dark { 0.048 } else { 0.038 };

    for i in (0..ambient_tiers).rev() {
        let t = (i + 1) as f32 / ambient_tiers as f32;
        let spread = ambient_max_spread * t;
        let alpha = base_ambient_alpha * (1.0 - t).powi(2);
        let shadow_rect = Rectangle {
            x: rect.x - spread * 0.5,
            y: rect.y - spread * 0.25,
            width: rect.width + spread,
            height: rect.height + spread * 1.1,
        };
        let shadow_arrow = if arrow.is_visible() {
            arrow.with_size(
                arrow.base_width + spread * 1.2,
                arrow.height + spread * 0.8,
            )
        } else {
            arrow
        };
        fill_popover(
            frame,
            shadow_rect,
            corner_radius + spread,
            shadow_arrow,
            Color::from_rgba(0.0, 0.0, 0.0, alpha),
        );
    }

    // 2. Key Elevation Drop Shadow: Deep spatial projection (macOS HIG elevation = 16pt)
    let key_tiers = 14;
    let key_max_spread = 30.0f32;
    let key_offset_y = 14.0f32;
    let base_key_alpha = if is_dark { 0.036 } else { 0.026 };

    for i in (0..key_tiers).rev() {
        let t = (i + 1) as f32 / key_tiers as f32;
        let spread = key_max_spread * t;
        let offset_y = key_offset_y * t;
        let alpha = base_key_alpha * (1.0 - t).powi(2);
        let shadow_rect = Rectangle {
            x: rect.x - spread * 0.75,
            y: rect.y + offset_y - spread * 0.35,
            width: rect.width + spread * 1.5,
            height: rect.height + spread * 1.35,
        };
        let shadow_arrow = if arrow.is_visible() {
            arrow.with_size(
                arrow.base_width + spread * 1.2,
                arrow.height + spread * 0.8,
            )
        } else {
            arrow
        };
        fill_popover(
            frame,
            shadow_rect,
            corner_radius + spread,
            shadow_arrow,
            Color::from_rgba(0.0, 0.0, 0.0, alpha),
        );
    }
}

/// Renders a continuous, artifact-free blurred occlusion clipped to continuous squircle/rounded corners
/// and integrated popover arrow (触角).
fn render_blurred_occlusion(
    frame: &mut Frame,
    style: WallpaperStyle,
    occ: &MenuOcclusion,
    blur_radius: f32,
    bounds: Size,
) {
    let rect = occ.bounds;
    let corner_radius = occ.corner_radius;
    let arrow = occ.arrow;
    let is_dark = occ.is_dark;

    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }

    if matches!(style, WallpaperStyle::PureWhite) {
        fill_popover(frame, rect, corner_radius, arrow, Color::WHITE);
        return;
    }
    if matches!(style, WallpaperStyle::PureBlack) {
        fill_popover(frame, rect, corner_radius, arrow, Color::BLACK);
        return;
    }

    let slice_w = 2.0f32;
    let r = corner_radius.min(rect.width * 0.5).min(rect.height * 0.5);

    // Continuous Apple squircle analytical curvature clipping (exponent n = 3.32)
    let exp_n = 2.0 + (4.2 - 2.0) * APPLE_CORNER_SMOOTHING;
    let inv_exp_n = 1.0 / exp_n;
    let r_pow_n = r.powf(exp_n);

    let extra_left = if arrow.edge == PopoverArrowEdge::Left { arrow.height } else { 0.0 };
    let extra_right = if arrow.edge == PopoverArrowEdge::Right { arrow.height } else { 0.0 };

    let mut curr_x = rect.x - extra_left;
    let end_x = rect.x + rect.width + extra_right;

    let arrow_center_x = rect.x + rect.width * arrow.offset;
    let arrow_half_w = arrow.base_width * 0.5;

    while curr_x < end_x {
        let actual_w = (end_x - curr_x).min(slice_w);
        let sample_x = curr_x + actual_w * 0.5;

        let local_x = sample_x - rect.x;
        let inset_y = if local_x >= 0.0 && local_x < rect.width {
            if local_x < r {
                let dx = r - local_x;
                let dy = (r_pow_n - dx.powf(exp_n)).max(0.0).powf(inv_exp_n);
                r - dy
            } else if local_x > rect.width - r {
                let dx = local_x - (rect.width - r);
                let dy = (r_pow_n - dx.powf(exp_n)).max(0.0).powf(inv_exp_n);
                r - dy
            } else {
                0.0
            }
        } else {
            0.0
        };

        let top_extension = if arrow.edge == PopoverArrowEdge::Top && (sample_x - arrow_center_x).abs() < arrow_half_w {
            let dist = ((sample_x - arrow_center_x).abs() / arrow_half_w).clamp(0.0, 1.0);
            0.5 * (1.0 + (dist * std::f32::consts::PI).cos()) * arrow.height
        } else {
            0.0
        };

        let bottom_extension = if arrow.edge == PopoverArrowEdge::Bottom && (sample_x - arrow_center_x).abs() < arrow_half_w {
            let dist = ((sample_x - arrow_center_x).abs() / arrow_half_w).clamp(0.0, 1.0);
            0.5 * (1.0 + (dist * std::f32::consts::PI).cos()) * arrow.height
        } else {
            0.0
        };

        let is_in_side_arrow = (arrow.edge == PopoverArrowEdge::Left && sample_x < rect.x)
            || (arrow.edge == PopoverArrowEdge::Right && sample_x > rect.x + rect.width);

        let (slice_top, slice_height) = if is_in_side_arrow {
            let arrow_center_y = rect.y + rect.height * arrow.offset;
            let dist_x = if arrow.edge == PopoverArrowEdge::Left {
                (rect.x - sample_x) / arrow.height
            } else {
                (sample_x - (rect.x + rect.width)) / arrow.height
            };
            let bell = 0.5 * (1.0 + (dist_x.clamp(0.0, 1.0) * std::f32::consts::PI).cos());
            let current_half_w = arrow_half_w * bell;
            (arrow_center_y - current_half_w, current_half_w * 2.0)
        } else {
            let top = rect.y + inset_y - top_extension;
            let height = (rect.height - inset_y * 2.0 + top_extension + bottom_extension).max(0.0);
            (top, height)
        };

        if slice_height > 0.0 {
            let color = sample_analytical_blurred_wallpaper(
                style,
                sample_x,
                rect.y + rect.height * 0.5,
                bounds,
                blur_radius,
            );
            frame.fill_rectangle(
                Point::new(curr_x, slice_top),
                Size::new(actual_w.ceil(), slice_height),
                color,
            );
        }

        curr_x += actual_w;
    }

    // Physical glass base tint overlay over the entire unified squircle+arrow
    let (base_r, base_g, base_b, base_a) = if is_dark {
        menu_metrics::DARK_MENU_BASE_RGBA_F32
    } else {
        menu_metrics::LIGHT_MENU_BASE_RGBA_F32
    };
    fill_popover(frame, rect, corner_radius, arrow, Color::from_rgba(base_r, base_g, base_b, base_a));

    // 1px fine rim highlight tracing the complete unified silhouette
    let rim_color = if is_dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.18)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.12)
    };
    stroke_popover_rim(frame, rect, corner_radius, arrow, rim_color, 1.0);
}

/// Canvas program rendering television color test blocks with continuous analytical backdrop blur occlusions.
struct WallpaperCanvas {
    style: WallpaperStyle,
    blur_preset: BlurPreset,
    occlusions: Vec<MenuOcclusion>,
    show_calibration_grid: bool,
}

impl<Message> canvas::Program<Message> for WallpaperCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // 1. Draw base sharp television wallpaper
        match self.style {
            WallpaperStyle::TvColorBars => {
                // 8 standard TV primary & secondary color bars across screen
                let colors = [
                    Color::WHITE,                   // 0: 白 (255, 255, 255)
                    Color::from_rgb(1.0, 1.0, 0.0), // 1: 黄 (255, 255, 0)
                    Color::from_rgb(0.0, 1.0, 1.0), // 2: 青 (0, 255, 255)
                    Color::from_rgb(0.0, 1.0, 0.0), // 3: 绿 (0, 255, 0)
                    Color::from_rgb(1.0, 0.0, 1.0), // 4: 洋红 (255, 0, 255)
                    Color::from_rgb(1.0, 0.0, 0.0), // 5: 红 (255, 0, 0)
                    Color::from_rgb(0.0, 0.0, 1.0), // 6: 蓝 (0, 0, 255)
                    Color::BLACK,                   // 7: 黑 (0, 0, 0)
                ];
                let n = colors.len() as f32;
                let bar_w = bounds.width / n;
                for (i, &c) in colors.iter().enumerate() {
                    let x = i as f32 * bar_w;
                    let rect_w = if i == colors.len() - 1 {
                        bounds.width - x
                    } else {
                        bar_w.ceil()
                    };
                    frame.fill_rectangle(
                        Point::new(x, 0.0),
                        Size::new(rect_w, bounds.height),
                        c,
                    );
                }
            }
            WallpaperStyle::TvSmpteSplit => {
                let top_h = bounds.height * 0.70;
                let bot_h = bounds.height - top_h;

                let top_colors = [
                    Color::WHITE,                   // 白
                    Color::from_rgb(1.0, 1.0, 0.0), // 黄
                    Color::from_rgb(0.0, 1.0, 1.0), // 青
                    Color::from_rgb(0.0, 1.0, 0.0), // 绿
                    Color::from_rgb(1.0, 0.0, 1.0), // 洋红
                    Color::from_rgb(1.0, 0.0, 0.0), // 红
                    Color::from_rgb(0.0, 0.0, 1.0), // 蓝
                ];
                let top_n = top_colors.len() as f32;
                let top_bar_w = bounds.width / top_n;
                for (i, &c) in top_colors.iter().enumerate() {
                    let x = i as f32 * top_bar_w;
                    frame.fill_rectangle(
                        Point::new(x, 0.0),
                        Size::new(top_bar_w.ceil(), top_h),
                        c,
                    );
                }

                let bot_colors = [
                    Color::from_rgb(0.0, 0.0, 1.0), // 蓝
                    Color::BLACK,                   // 黑
                    Color::from_rgb(1.0, 0.0, 1.0), // 洋红
                    Color::BLACK,                   // 黑
                    Color::from_rgb(0.0, 1.0, 1.0), // 青
                    Color::BLACK,                   // 黑
                    Color::WHITE,                   // 白
                    Color::BLACK,                   // 黑
                ];
                let bot_n = bot_colors.len() as f32;
                let bot_bar_w = bounds.width / bot_n;
                for (i, &c) in bot_colors.iter().enumerate() {
                    let x = i as f32 * bot_bar_w;
                    frame.fill_rectangle(
                        Point::new(x, top_h),
                        Size::new(bot_bar_w.ceil(), bot_h),
                        c,
                    );
                }
            }
            WallpaperStyle::TvColorGrid => {
                let cols = 4;
                let rows = 3;
                let cell_w = bounds.width / cols as f32;
                let cell_h = bounds.height / rows as f32;
                let palette = [
                    Color::WHITE,
                    Color::from_rgb(1.0, 1.0, 0.0),
                    Color::from_rgb(0.0, 1.0, 1.0),
                    Color::from_rgb(0.0, 1.0, 0.0),
                    Color::from_rgb(1.0, 0.0, 1.0),
                    Color::from_rgb(1.0, 0.0, 0.0),
                    Color::from_rgb(0.0, 0.0, 1.0),
                    Color::BLACK,
                    Color::from_rgb(1.0, 0.5, 0.0),
                    Color::from_rgb(0.5, 0.0, 0.5),
                    Color::from_rgb(0.0, 0.5, 0.5),
                    Color::from_rgb(0.5, 0.5, 0.5),
                ];
                for r in 0..rows {
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let x = c as f32 * cell_w;
                        let y = r as f32 * cell_h;
                        frame.fill_rectangle(
                            Point::new(x, y),
                            Size::new(cell_w.ceil(), cell_h.ceil()),
                            palette[idx % palette.len()],
                        );
                    }
                }
            }
            WallpaperStyle::PureWhite => {
                frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::WHITE);
            }
            WallpaperStyle::PureBlack => {
                frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::BLACK);
            }
        }

        // 2. Draw high-frequency test patterns & calibration grid on sharp background
        if self.show_calibration_grid
            && !matches!(self.style, WallpaperStyle::PureWhite | WallpaperStyle::PureBlack)
        {
            let grid_step = 36.0f32;
            let line_color = Color::from_rgba(1.0, 1.0, 1.0, 0.16);
            let dark_line_color = Color::from_rgba(0.0, 0.0, 0.0, 0.18);

            // Vertical fine grid lines
            let mut x = grid_step;
            while x < bounds.width {
                frame.fill_rectangle(Point::new(x, 0.0), Size::new(1.0, bounds.height), line_color);
                x += grid_step;
            }

            // Horizontal fine grid lines
            let mut y = grid_step;
            while y < bounds.height {
                frame.fill_rectangle(Point::new(0.0, y), Size::new(bounds.width, 1.0), dark_line_color);
                y += grid_step;
            }

            // Central crosshair & calibration test target
            let cx = (bounds.width * 0.5).round();
            let cy = (bounds.height * 0.5).round();
            frame.fill_rectangle(Point::new(cx - 60.0, cy - 1.0), Size::new(120.0, 2.0), Color::WHITE);
            frame.fill_rectangle(Point::new(cx - 1.0, cy - 60.0), Size::new(2.0, 120.0), Color::WHITE);
        }

        // 3. Render authentic Apple multi-tier soft drop shadows and continuous analytical backdrop blur
        if !self.occlusions.is_empty() {
            let blur_radius = self.blur_preset.radius();

            // Step 3a: Soft ambient contact shadow & elevation drop shadow behind each menu card
            for occ in &self.occlusions {
                render_soft_menu_shadow(&mut frame, occ);
            }

            // Step 3b: Continuous analytical backdrop blur inside each menu container squircle + arrow
            for occ in &self.occlusions {
                render_blurred_occlusion(
                    &mut frame,
                    self.style,
                    occ,
                    blur_radius,
                    bounds.size(),
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

#[derive(Debug)]
pub struct State {
    pub controller: WindowShellController,
    pub traffic_lights: TrafficLightsState,
    pub theme: Theme,
    pub palette: liquid_glass::UiPalette,
    pub wallpaper: WallpaperStyle,
    pub blur_preset: BlurPreset,
    pub show_calibration_grid: bool,
    pub floating_appearance: FloatingAppearance,
    pub show_status_bar: bool,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub cursor_pos: Point,
    pub floating_menu: Option<Point>,
    pub last_action: Option<String>,
    pub light_menu: ContextMenu<Message>,
    pub dark_menu: ContextMenu<Message>,
    pub floating_menu_cached: ContextMenu<Message>,

    // Draggable card positions
    pub light_pos: Point,
    pub dark_pos: Point,
    pub active_drag: Option<(DragTarget, Point)>,
    pub top_card: DragTarget,
    pub popover_arrow_edge: PopoverArrowEdge,
    pub arrow_preset: PopoverArrowPreset,
    pub arrow_offset: f32,
    pub menu_preset: MenuContentPreset,
}

impl Default for State {
    fn default() -> Self {
        let is_dark = is_system_dark_mode();
        let config = WindowChromeConfig::unified_header(window_metrics::FUSED_HEADER_HEIGHT);
        let controller = WindowShellController::new(config, is_dark);
        let scheme = if is_dark {
            UiColorScheme::Dark
        } else {
            UiColorScheme::Light
        };
        let theme = UiTheme::new(scheme).iced_theme();
        let palette = UiTheme::new(scheme).palette();

        Self {
            controller,
            traffic_lights: TrafficLightsState::new(),
            theme,
            palette,
            wallpaper: WallpaperStyle::TvColorBars,
            blur_preset: BlurPreset::UltraHeavy64,
            show_calibration_grid: true,
            floating_appearance: FloatingAppearance::FollowTheme,
            show_status_bar: true,
            show_line_numbers: true,
            word_wrap: false,
            cursor_pos: Point::new(320.0, 240.0),
            floating_menu: None,
            last_action: None,
            light_menu: ContextMenu::new(),
            dark_menu: ContextMenu::new(),
            floating_menu_cached: ContextMenu::new(),
            light_pos: Point::new(70.0, 50.0),
            dark_pos: Point::new(450.0, 50.0),
            active_drag: None,
            top_card: DragTarget::DarkCard,
            popover_arrow_edge: PopoverArrowEdge::Bottom,
            arrow_preset: PopoverArrowPreset::MenuWide,
            arrow_offset: demo_metrics::DOCK_REF_ARROW_OFFSET,
            menu_preset: MenuContentPreset::DockReference1To1,
        }
    }
}

impl State {
    #[must_use]
    pub fn current_arrow_config(&self) -> PopoverArrowConfig {
        PopoverArrowConfig::from_preset(self.popover_arrow_edge, self.arrow_preset)
            .with_offset(self.arrow_offset)
    }
    #[must_use]
    pub fn scheme(&self) -> UiColorScheme {
        if self.controller.is_dark {
            UiColorScheme::Dark
        } else {
            UiColorScheme::Light
        }
    }

    pub fn rebuild_menus(&mut self) {
        let scheme = self.scheme();
        self.theme = UiTheme::new(scheme).iced_theme();
        self.palette = UiTheme::new(scheme).palette();
        self.light_menu = build_demo_menu(self).with_scheme(UiColorScheme::Light);
        self.dark_menu = build_demo_menu(self).with_scheme(UiColorScheme::Dark);
        let float_scheme = self.resolved_floating_scheme();
        self.floating_menu_cached = build_demo_menu(self).with_scheme(float_scheme);
    }

    #[must_use]
    pub fn resolved_floating_scheme(&self) -> UiColorScheme {
        self.floating_appearance.resolve(self.scheme())
    }
}

pub fn boot() -> (State, Task<Message>) {
    let mut state = State::default();
    state.rebuild_menus();
    (state, Task::none())
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::WindowOpened(id) => {
            state.controller.set_window_id(id);
            let is_dark = state.controller.is_dark;
            window::run(id, move |w| {
                if let Ok(handle) = w.window_handle() {
                    let appearance = if is_dark {
                        bmol_window_shell::native::WindowAppearance::Dark
                    } else {
                        bmol_window_shell::native::WindowAppearance::Light
                    };
                    let _ = bmol_window_shell::setup_native_window(
                        handle.as_raw(),
                        bmol_window_shell::NativeWindowOptions::new()
                            .with_appearance(appearance)
                            .with_corner_radius(f64::from(window_metrics::DEFAULT_CORNER_RADIUS)),
                    );
                }
            })
            .discard()
        }
        Message::WindowResized(size) => {
            state.controller.handle_resized(size.width, size.height);
            Task::none()
        }
        Message::WindowEvent((_id, event)) => {
            if let Some(shell_event) = state.controller.handle_window_event(&event) {
                match shell_event {
                    bmol_window_shell::ShellEvent::Focused
                    | bmol_window_shell::ShellEvent::Unfocused => {
                        state.traffic_lights.on_group_hover(false);
                        state.active_drag = None;
                    }
                    bmol_window_shell::ShellEvent::CloseRequested => {
                        if let Some(id) = state.controller.window_id {
                            return window::close(id);
                        }
                    }
                    _ => {}
                }
            }
            Task::none()
        }
        Message::DragWindow => {
            if let Some(id) = state.controller.window_id {
                window::drag(id)
            } else {
                Task::none()
            }
        }
        Message::ToggleMaximize => {
            if let Some(id) = state.controller.window_id {
                window::toggle_maximize(id)
            } else {
                Task::none()
            }
        }
        Message::ResizeWindow(direction) => {
            if let Some(id) = state.controller.window_id {
                window::drag_resize(id, direction)
            } else {
                Task::none()
            }
        }
        Message::WindowControl(action) => match action {
            ControlAction::Close => {
                if let Some(id) = state.controller.window_id {
                    window::close(id)
                } else {
                    Task::none()
                }
            }
            ControlAction::Minimize => {
                if let Some(id) = state.controller.window_id {
                    window::minimize(id, true)
                } else {
                    Task::none()
                }
            }
            ControlAction::Expand => {
                if let Some(id) = state.controller.window_id {
                    window::toggle_maximize(id)
                } else {
                    Task::none()
                }
            }
        },
        Message::TrafficLightsHover(hovered) => {
            state.traffic_lights.on_group_hover(hovered);
            Task::none()
        }
        Message::TrafficLightsPressStart(idx) => {
            state.traffic_lights.on_press_start(idx);
            Task::none()
        }
        Message::TrafficLightsPressCancel(idx) => {
            state.traffic_lights.on_press_cancel(idx);
            Task::none()
        }
        Message::TrafficLightsPressEnd(idx) => {
            state.traffic_lights.on_press_end(idx);
            Task::none()
        }
        Message::AnimationFrame(now) => {
            state.traffic_lights.step(now);
            Task::none()
        }
        Message::RightClicked => {
            state.floating_menu = Some(state.cursor_pos);
            state.last_action = Some(format!(
                "Menu Spawned at ({:.0}, {:.0})",
                state.cursor_pos.x, state.cursor_pos.y
            ));
            Task::none()
        }
        Message::CursorMoved(point) => {
            state.cursor_pos = point;
            if let Some((target, offset)) = state.active_drag {
                let new_x = (point.x - offset.x).max(10.0);
                let new_y = (point.y - offset.y).max(10.0);
                match target {
                    DragTarget::LightCard => {
                        state.light_pos = Point::new(new_x, new_y);
                    }
                    DragTarget::DarkCard => {
                        state.dark_pos = Point::new(new_x, new_y);
                    }
                }
            }
            Task::none()
        }
        Message::StartDragCard(target) => {
            let origin = match target {
                DragTarget::LightCard => state.light_pos,
                DragTarget::DarkCard => state.dark_pos,
            };
            let offset = Point::new(
                state.cursor_pos.x - origin.x,
                state.cursor_pos.y - origin.y,
            );
            state.active_drag = Some((target, offset));
            state.top_card = target;
            state.last_action = Some(format!(
                "拖拽中: {} (移动到各色彩条上方测色)",
                match target {
                    DragTarget::LightCard => "浅色模式菜单",
                    DragTarget::DarkCard => "深色模式菜单",
                }
            ));
            Task::none()
        }
        Message::EndDragCard => {
            if let Some((target, _)) = state.active_drag.take() {
                let pos = match target {
                    DragTarget::LightCard => state.light_pos,
                    DragTarget::DarkCard => state.dark_pos,
                };
                state.last_action = Some(format!(
                    "已固定 {} 至 ({:.0}, {:.0})",
                    match target {
                        DragTarget::LightCard => "浅色模式菜单",
                        DragTarget::DarkCard => "深色模式菜单",
                    },
                    pos.x, pos.y
                ));
            }
            Task::none()
        }
        Message::ResetCardPositions => {
            state.light_pos = Point::new(70.0, 50.0);
            state.dark_pos = Point::new(450.0, 50.0);
            state.active_drag = None;
            state.last_action = Some("已重置菜单示例位置".to_string());
            Task::none()
        }
        Message::DismissFloatingMenu => {
            if state.floating_menu.is_some() {
                state.floating_menu = None;
                state.last_action = Some("Menu Dismissed (Outside Click)".to_string());
            }
            Task::none()
        }
        Message::OpenFloatingMenuAt(pos) => {
            state.floating_menu = Some(pos);
            state.last_action = Some(format!("Menu Spawned at ({:.0}, {:.0})", pos.x, pos.y));
            Task::none()
        }
        Message::TriggerAction(action) => {
            match action {
                MenuAction::ToggleStatusBar => {
                    state.show_status_bar = !state.show_status_bar;
                    state.last_action = Some(format!(
                        "Toggled Status Bar -> {}",
                        if state.show_status_bar { "ON" } else { "OFF" }
                    ));
                    state.rebuild_menus();
                }
                MenuAction::ToggleLineNumbers => {
                    state.show_line_numbers = !state.show_line_numbers;
                    state.last_action = Some(format!(
                        "Toggled Line Numbers -> {}",
                        if state.show_line_numbers { "ON" } else { "OFF" }
                    ));
                    state.rebuild_menus();
                }
                MenuAction::ToggleWordWrap => {
                    state.word_wrap = !state.word_wrap;
                    state.last_action = Some(format!(
                        "Toggled Word Wrap -> {}",
                        if state.word_wrap { "ON" } else { "OFF" }
                    ));
                    state.rebuild_menus();
                }
                other => {
                    state.last_action = Some(format!("Triggered Action: {}", other.description()));
                }
            }
            state.floating_menu = None;
            Task::none()
        }
        Message::SetWallpaper(w) => {
            state.wallpaper = w;
            state.last_action = Some(format!("Switched Wallpaper: {}", w.label()));
            Task::none()
        }
        Message::SetBlurPreset(b) => {
            state.blur_preset = b;
            state.last_action = Some(format!("Blur Kernel: {}", b.label()));
            Task::none()
        }
        Message::SetFloatingAppearance(a) => {
            state.floating_appearance = a;
            state.rebuild_menus();
            state.last_action = Some(format!("Floating Menu Appearance: {}", a.label()));
            Task::none()
        }
        Message::ToggleColorScheme => {
            state.controller.set_dark_mode(!state.controller.is_dark);
            state.rebuild_menus();
            state.last_action = Some(format!("Window Theme: {:?}", state.scheme()));
            Task::none()
        }
        Message::ToggleCalibrationGrid => {
            state.show_calibration_grid = !state.show_calibration_grid;
            state.last_action = Some(format!(
                "高频测试网格: {}",
                if state.show_calibration_grid {
                    "开启 (1px细线在菜单外部锐利可见，在菜单遮罩下彻底被Dual-Kawase滤除消融)"
                } else {
                    "关闭"
                }
            ));
            Task::none()
        }
        Message::SetPopoverArrow(edge) => {
            state.popover_arrow_edge = edge;
            state.arrow_offset = if state.menu_preset == MenuContentPreset::DockReference1To1 && edge == PopoverArrowEdge::Bottom {
                demo_metrics::DOCK_REF_ARROW_OFFSET
            } else {
                0.5
            };
            state.last_action = Some(format!(
                "卡片触角形态: {}",
                match edge {
                    PopoverArrowEdge::None => "无触角",
                    PopoverArrowEdge::Top => "▲ 顶部触角 (Top)",
                    PopoverArrowEdge::Bottom => "▼ 底部触角 (Bottom)",
                    PopoverArrowEdge::Left => "◀ 左侧触角 (Left)",
                    PopoverArrowEdge::Right => "▶ 右侧触角 (Right)",
                }
            ));
            Task::none()
        }
        Message::ToggleMenuPreset => {
            state.menu_preset = match state.menu_preset {
                MenuContentPreset::FullShowcase => {
                    state.popover_arrow_edge = PopoverArrowEdge::Bottom;
                    state.arrow_offset = demo_metrics::DOCK_REF_ARROW_OFFSET;
                    state.last_action = Some("已切换为: 🍎 1:1 原生 macOS Dock 菜单模式 (偏左圆润触角)".into());
                    MenuContentPreset::DockReference1To1
                }
                MenuContentPreset::DockReference1To1 => {
                    state.arrow_offset = 0.5;
                    state.last_action = Some("已切换为: 📑 完整全功能展示菜单模式".into());
                    MenuContentPreset::FullShowcase
                }
            };
            state.rebuild_menus();
            Task::none()
        }
        Message::CycleArrowPreset => {
            state.arrow_preset = match state.arrow_preset {
                PopoverArrowPreset::MenuWide => {
                    state.last_action = Some("已切换触角预设: 🎯 细窄 Hover / Tooltip 气泡 (16×7 pt, R2.0 挺拔)".into());
                    PopoverArrowPreset::TooltipNarrow
                }
                PopoverArrowPreset::TooltipNarrow => {
                    state.last_action = Some("已切换触角预设: 📐 系统原生 AppKit NSPopover (27.5×13 pt, R5.0)".into());
                    PopoverArrowPreset::AppKitStandard
                }
                PopoverArrowPreset::AppKitStandard => {
                    state.last_action = Some("已切换触角预设: 🔹 微型提示指针 (12×5 pt, R1.5 超紧凑)".into());
                    PopoverArrowPreset::SubtleCompact
                }
                PopoverArrowPreset::SubtleCompact => {
                    state.last_action = Some("已切换触角预设: 🍎 宽型菜单穹顶气泡 (26×10 pt, R5.0 饱满圆润)".into());
                    PopoverArrowPreset::MenuWide
                }
            };
            Task::none()
        }
    }
}

pub fn subscription(state: &State) -> Subscription<Message> {
    let mut subscriptions = vec![
        window::open_events().map(Message::WindowOpened),
        window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
        iced::event::listen_with(|event, _status, id| match event {
            iced::Event::Window(w_event) => {
                Some(Message::WindowEvent((id, w_event)))
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                Some(Message::RightClicked)
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Some(Message::EndDragCard)
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Some(Message::CursorMoved(position))
            }
            _ => None,
        }),
    ];

    if state.traffic_lights.is_animating() {
        subscriptions.push(window::frames().map(Message::AnimationFrame));
    }

    Subscription::batch(subscriptions)
}

#[must_use]
pub fn app_theme(state: &State) -> Theme {
    state.theme.clone()
}

/// Builds the comprehensive Apple-style Context Menu.
fn build_demo_menu(state: &State) -> ContextMenu<Message> {
    if state.menu_preset == MenuContentPreset::DockReference1To1 {
        return ContextMenu::new()
            .width(demo_metrics::DOCK_REF_WIDTH)
            .with_border(false)
            .with_transparent_background(true)
            .item(
                MenuItem::submenu("选项")
                    .on_press(Message::TriggerAction(MenuAction::OpenSettings)),
            )
            .item(MenuItem::separator())
            .item(
                MenuItem::action("显示所有窗口")
                    .on_press(Message::TriggerAction(MenuAction::QuickLook)),
            )
            .item(
                MenuItem::action("隐藏")
                    .on_press(Message::TriggerAction(MenuAction::Cut)),
            )
            .item(
                MenuItem::action("退出")
                    .on_press(Message::WindowControl(ControlAction::Close)),
            );
    }

    ContextMenu::new()
        .width(menu_metrics::DEFAULT_WIDTH)
        .with_border(false)
        .with_transparent_background(true)
        .item(MenuItem::section("FILE"))
        .item(
            MenuItem::action("New Window")
                .with_shortcut("⌘N")
                .with_icon(UiIcon::Gear)
                .on_press(Message::TriggerAction(MenuAction::NewWindow)),
        )
        .item(
            MenuItem::action("Quick Look")
                .with_shortcut("Space")
                .on_press(Message::TriggerAction(MenuAction::QuickLook)),
        )
        .item(MenuItem::submenu("Open With..."))
        .item(MenuItem::separator())
        .item(MenuItem::section("EDIT"))
        .item(
            MenuItem::action("Cut")
                .with_shortcut("⌘X")
                .on_press(Message::TriggerAction(MenuAction::Cut)),
        )
        .item(
            MenuItem::action("Copy")
                .with_shortcut("⌘C")
                .on_press(Message::TriggerAction(MenuAction::Copy)),
        )
        .item(
            MenuItem::action("Paste")
                .with_shortcut("⌘V")
                .with_disabled(true)
                .on_press(Message::TriggerAction(MenuAction::Paste)),
        )
        .item(MenuItem::separator())
        .item(MenuItem::section("VIEW & OPTIONS"))
        .item(
            MenuItem::checkbox("Show Status Bar", state.show_status_bar)
                .on_toggle(Message::TriggerAction(MenuAction::ToggleStatusBar)),
        )
        .item(
            MenuItem::checkbox("Show Line Numbers", state.show_line_numbers)
                .on_toggle(Message::TriggerAction(MenuAction::ToggleLineNumbers)),
        )
        .item(
            MenuItem::checkbox("Word Wrap", state.word_wrap)
                .on_toggle(Message::TriggerAction(MenuAction::ToggleWordWrap)),
        )
        .item(MenuItem::separator())
        .item(MenuItem::section("SYSTEM"))
        .item(
            MenuItem::action("Settings...")
                .with_shortcut("⌘,")
                .with_icon(UiIcon::Gear)
                .on_press(Message::TriggerAction(MenuAction::OpenSettings)),
        )
        .item(
            MenuItem::action("Search Help")
                .with_shortcut("⇧⌘F")
                .with_icon(UiIcon::Search)
                .on_press(Message::TriggerAction(MenuAction::SearchHelp)),
        )
        .item(
            MenuItem::action("About This App")
                .with_icon(UiIcon::Info)
                .on_press(Message::TriggerAction(MenuAction::AboutApp)),
        )
        .item(MenuItem::separator())
        .item(MenuItem::section("DANGER ZONE"))
        .item(
            MenuItem::action("Move to Trash")
                .with_shortcut("⌫")
                .with_destructive(true)
                .on_press(Message::TriggerAction(MenuAction::DeleteDestructive)),
        )
}

/// Builds the authentic Apple Traffic Lights button row with dynamic symmetric margins.
fn view_traffic_lights_group(
    state: &State,
    symmetric_margin: f32,
) -> Element<'_, Message, Theme, iced::Renderer> {
    let is_focused = state.controller.is_focused;
    let hover_amount = state.traffic_lights.hover_progress;
    let is_dark = state.controller.is_dark;

    let build_btn = |action: ControlAction,
                     index: usize,
                     base_color: Color,
                     hover_color: Color,
                     press_color: Color,
                     border_color: Color,
                     glyph_char: &'static str| {
        let is_animating = state.traffic_lights.press_targets[index] > 0.0;
        let scale = state.traffic_lights.press_springs[index].value();
        let size = traffic_lights::DIAMETER * scale;

        let fill_color = if !is_focused && hover_amount < 0.05 {
            if is_dark {
                Color::from_rgb8(0x4C, 0x4C, 0x50)
            } else {
                Color::from_rgb8(0xD1, 0xD1, 0xD6)
            }
        } else if is_animating {
            press_color
        } else if hover_amount > 0.5 {
            hover_color
        } else {
            base_color
        };

        let glyph_text = text(glyph_char)
            .size(if action == ControlAction::Close { 8.0 } else { 7.0 })
            .font(font::ui_font(Weight::Bold))
            .color(Color {
                a: hover_amount * if is_dark { 0.85 } else { 0.75 },
                ..match action {
                    ControlAction::Close => Color::from_rgb8(0x4C, 0x00, 0x00),
                    ControlAction::Minimize => Color::from_rgb8(0x5A, 0x36, 0x00),
                    ControlAction::Expand => Color::from_rgb8(0x0A, 0x38, 0x00),
                }
            });

        let btn_content = container(glyph_text)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .center_x(Length::Fixed(size))
            .center_y(Length::Fixed(size));

        button(btn_content)
            .padding(0)
            .style(move |_theme, _status| button::Style {
                background: Some(Background::Color(fill_color)),
                border: Border::default()
                    .rounded(size * 0.5)
                    .width(0.5)
                    .color(border_color),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                    offset: Vector::new(0.0, 0.5),
                    blur_radius: 1.0,
                },
                ..button::Style::default()
            })
            .on_press(Message::WindowControl(action))
    };

    let red = build_btn(
        ControlAction::Close,
        0,
        Color::from_rgb8(0xFF, 0x5F, 0x56),
        Color::from_rgb8(0xFF, 0x6E, 0x67),
        Color::from_rgb8(0xD3, 0x3B, 0x36),
        Color::from_rgb8(0xE0, 0x44, 0x3E),
        "✕",
    );

    let yellow = build_btn(
        ControlAction::Minimize,
        1,
        Color::from_rgb8(0xFF, 0xBD, 0x2E),
        Color::from_rgb8(0xFF, 0xC8, 0x47),
        Color::from_rgb8(0xD7, 0x96, 0x1E),
        Color::from_rgb8(0xDE, 0xA1, 0x23),
        "─",
    );

    let green = build_btn(
        ControlAction::Expand,
        2,
        Color::from_rgb8(0x27, 0xC9, 0x3F),
        Color::from_rgb8(0x32, 0xD8, 0x4D),
        Color::from_rgb8(0x19, 0xA0, 0x23),
        Color::from_rgb8(0x1A, 0xAB, 0x29),
        "⤢",
    );

    let slop = traffic_lights::control_hover_slop(traffic_lights::DIAMETER);
    let controls_row = row![red, yellow, green]
        .spacing(traffic_lights::SPACING)
        .align_y(Alignment::Center);

    let tracking_area = container(controls_row).padding(Padding {
        top: slop,
        right: slop,
        bottom: slop,
        left: slop,
    });

    let interactive_group = iced::widget::mouse_area(tracking_area)
        .on_enter(Message::TrafficLightsHover(true))
        .on_exit(Message::TrafficLightsHover(false));

    let spacer_left = (symmetric_margin - slop).max(0.0);
    row![
        space().width(Length::Fixed(spacer_left)),
        interactive_group,
    ]
    .align_y(Alignment::Center)
    .into()
}

/// Builds the wallpaper style picker.
fn view_wallpaper_picker<'a>(
    state: &'a State,
    palette: &'a liquid_glass::UiPalette,
    is_dark: bool,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let mut picker = row![].spacing(4.0).align_y(Alignment::Center);
    for &style in &WallpaperStyle::ALL {
        let is_selected = state.wallpaper == style;
        let btn = button(
            text(style.label())
                .size(11.0)
                .font(font::ui_font(if is_selected {
                    Weight::Semibold
                } else {
                    Weight::Normal
                }))
                .color(if is_selected {
                    Color::WHITE
                } else {
                    palette.text_secondary
                }),
        )
        .padding(Padding {
            top: 4.0,
            right: 10.0,
            bottom: 4.0,
            left: 10.0,
        })
        .style(move |_theme, _status| {
            if is_selected {
                button::Style {
                    background: Some(Background::Color(palette.accent)),
                    border: Border::default().rounded(6.0),
                    ..button::Style::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.06)
                    })),
                    border: Border::default().rounded(6.0),
                    ..button::Style::default()
                }
            }
        })
        .on_press(Message::SetWallpaper(style));
        picker = picker.push(btn);
    }
    picker.into()
}

/// Builds the blur strength selector.
fn view_blur_preset_picker<'a>(
    state: &'a State,
    palette: &'a liquid_glass::UiPalette,
    is_dark: bool,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let mut picker = row![].spacing(4.0).align_y(Alignment::Center);
    for &preset in &BlurPreset::ALL {
        let is_selected = state.blur_preset == preset;
        let btn = button(
            text(preset.label())
                .size(11.0)
                .font(font::ui_font(if is_selected {
                    Weight::Semibold
                } else {
                    Weight::Normal
                }))
                .color(if is_selected {
                    Color::WHITE
                } else {
                    palette.text_secondary
                }),
        )
        .padding(Padding {
            top: 4.0,
            right: 9.0,
            bottom: 4.0,
            left: 9.0,
        })
        .style(move |_theme, _status| {
            if is_selected {
                button::Style {
                    background: Some(Background::Color(palette.accent)),
                    border: Border::default().rounded(6.0),
                    ..button::Style::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.06)
                    })),
                    border: Border::default().rounded(6.0),
                    ..button::Style::default()
                }
            }
        })
        .on_press(Message::SetBlurPreset(preset));
        picker = picker.push(btn);
    }
    picker.into()
}

/// Builds the popover arrow / beak (触角) edge selector.
fn view_popover_arrow_picker<'a>(
    state: &'a State,
    palette: &'a liquid_glass::UiPalette,
    is_dark: bool,
) -> Element<'a, Message, Theme, iced::Renderer> {
    const EDGES: [(PopoverArrowEdge, &str); 5] = [
        (PopoverArrowEdge::None, "无触角"),
        (PopoverArrowEdge::Top, "▲ 顶"),
        (PopoverArrowEdge::Bottom, "▼ 底"),
        (PopoverArrowEdge::Left, "◀ 左"),
        (PopoverArrowEdge::Right, "▶ 右"),
    ];

    let mut picker = row![].spacing(3.0).align_y(Alignment::Center);
    for &(edge, label) in &EDGES {
        let is_selected = state.popover_arrow_edge == edge;
        let btn = button(
            text(label)
                .size(10.5)
                .font(font::ui_font(if is_selected {
                    Weight::Semibold
                } else {
                    Weight::Normal
                }))
                .color(if is_selected {
                    Color::WHITE
                } else {
                    palette.text_secondary
                }),
        )
        .padding(Padding {
            top: 3.5,
            right: 7.0,
            bottom: 3.5,
            left: 7.0,
        })
        .style(move |_theme, _status| {
            if is_selected {
                button::Style {
                    background: Some(Background::Color(palette.accent)),
                    border: Border::default().rounded(5.0),
                    ..button::Style::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.06)
                    })),
                    border: Border::default().rounded(5.0),
                    ..button::Style::default()
                }
            }
        })
        .on_press(Message::SetPopoverArrow(edge));
        picker = picker.push(btn);
    }
    picker.into()
}

/// Builds the top fused header bar managed by `loyal_drag_bar`.
fn view_top_header<'a>(
    state: &'a State,
    palette: &'a liquid_glass::UiPalette,
    is_dark: bool,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let header_height = state.controller.metrics.header_rect.height;
    let symmetric_margin = ((header_height - traffic_lights::DIAMETER) * 0.5).max(4.0);

    let traffic_lights = view_traffic_lights_group(state, symmetric_margin);

    let title_text = row![
        space().width(Length::Fixed(traffic_lights::TITLE_CLEARANCE)),
        text("Liquid Glass Context Menu")
            .size(13.5)
            .font(font::ui_font(Weight::Semibold))
            .color(palette.text_primary),
    ]
    .align_y(Alignment::Center);

    let arrow_picker = view_popover_arrow_picker(state, palette, is_dark);
    let wallpaper_picker = view_wallpaper_picker(state, palette, is_dark);
    let blur_picker = view_blur_preset_picker(state, palette, is_dark);

    let preset_btn = button(
        text(match state.menu_preset {
            MenuContentPreset::DockReference1To1 => "🍎 1:1参考图模式",
            MenuContentPreset::FullShowcase => "📑 全功能模式",
        })
        .size(10.5)
        .font(font::ui_font(Weight::Semibold))
        .color(if state.menu_preset == MenuContentPreset::DockReference1To1 {
            Color::WHITE
        } else {
            palette.text_primary
        }),
    )
    .padding(Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 8.0,
    })
    .style(move |_theme, _status| {
        if state.menu_preset == MenuContentPreset::DockReference1To1 {
            button::Style {
                background: Some(Background::Color(Color::from_rgb(0.18, 0.55, 0.95))),
                border: Border::default().rounded(6.0),
                ..button::Style::default()
            }
        } else {
            button::Style {
                background: Some(Background::Color(if is_dark {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                } else {
                    Color::from_rgba(0.0, 0.0, 0.0, 0.06)
                })),
                border: Border::default().rounded(6.0),
                ..button::Style::default()
            }
        }
    })
    .on_press(Message::ToggleMenuPreset);

    let arrow_preset_text = match state.arrow_preset {
        PopoverArrowPreset::MenuWide => "▼ 宽型菜单 (26×10)",
        PopoverArrowPreset::TooltipNarrow => "▼ 细窄Hover (16×7)",
        PopoverArrowPreset::AppKitStandard => "▼ 原生NSPopover (27.5×13)",
        PopoverArrowPreset::SubtleCompact => "▼ 微型提示 (12×5)",
    };

    let arrow_preset_btn = button(
        text(arrow_preset_text)
            .size(10.5)
            .font(font::ui_font(Weight::Medium))
            .color(palette.text_primary),
    )
    .padding(Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 8.0,
    })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.08)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.06)
        })),
        border: Border::default().rounded(6.0),
        ..button::Style::default()
    })
    .on_press(Message::CycleArrowPreset);

    let theme_btn = button(
        text(if is_dark { "☀️ Light" } else { "🌙 Dark" })
            .size(11.0)
            .font(font::ui_font(Weight::Medium))
            .color(palette.text_primary),
    )
    .padding(Padding {
        top: 4.0,
        right: 10.0,
        bottom: 4.0,
        left: 10.0,
    })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.10)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.06)
        })),
        border: Border::default().rounded(6.0),
        ..button::Style::default()
    })
    .on_press(Message::ToggleColorScheme);

    let header_row = row![
        traffic_lights,
        title_text,
        space().width(Length::Fill),
        preset_btn,
        space().width(6.0),
        arrow_preset_btn,
        space().width(6.0),
        arrow_picker,
        space().width(8.0),
        wallpaper_picker,
        space().width(8.0),
        blur_picker,
        space().width(8.0),
        theme_btn,
        space().width(12.0),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(header_height))
    .width(Length::Fill);

    let header_container = container(header_row)
        .width(Length::Fill)
        .height(Length::Fixed(header_height))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.10, 0.10, 0.13, 0.72)
            } else {
                Color::from_rgba(0.96, 0.96, 0.98, 0.76)
            })),
            border: Border::default().width(0.5).color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.10)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.08)
            }),
            ..container::Style::default()
        });

    state.controller.loyal_drag_bar(
        header_height,
        header_container,
        Message::DragWindow,
        Some(Message::ToggleMaximize),
    )
}

/// Visual and metadata configuration for draggable showcase menu cards.
#[derive(Debug, Clone, Copy)]
struct MenuCardConfig {
    target: DragTarget,
    badge_title: &'static str,
    badge_sub: &'static str,
    is_dark_card: bool,
    card_width: f32,
    card_height: f32,
}

/// Builds a draggable menu showcase card with grip bar and transparent background.
fn view_menu_card<'a>(
    config: MenuCardConfig,
    menu: &'a ContextMenu<Message>,
    is_dragging: bool,
    theme: &'a Theme,
    palette: &'a liquid_glass::UiPalette,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let grip_pill = row![
        text("⠿")
            .size(14.0)
            .font(font::ui_font(Weight::Bold))
            .color(palette.accent),
        column![
            text(config.badge_title)
                .size(11.0)
                .font(font::ui_font(Weight::Bold))
                .color(palette.text_primary),
            text(config.badge_sub)
                .size(9.5)
                .font(font::ui_font(Weight::Normal))
                .color(palette.text_secondary),
        ]
        .spacing(1.0),
        space().width(Length::Fill),
        text(if is_dragging { "松开" } else { "拖拽" })
            .size(9.5)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dragging { palette.accent } else { palette.text_tertiary }),
    ]
    .spacing(6.0)
    .align_y(Alignment::Center);

    let drag_header = iced::widget::mouse_area(
        container(grip_pill)
            .width(Length::Fixed(config.card_width))
            .height(Length::Fixed(demo_metrics::DRAG_HEADER_HEIGHT))
            .padding(Padding {
                top: 4.0,
                right: 8.0,
                bottom: 4.0,
                left: 8.0,
            })
            .style(move |_theme| container::Style {
                background: Some(Background::Color(if config.is_dark_card {
                    Color::from_rgba(0.12, 0.12, 0.16, 0.90)
                } else {
                    Color::from_rgba(0.96, 0.96, 0.98, 0.92)
                })),
                border: Border::default()
                    .rounded(8.0)
                    .width(1.0)
                    .color(if is_dragging {
                        palette.accent
                    } else if config.is_dark_card {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.22)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.15)
                    }),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, if is_dragging { 0.35 } else { 0.18 }),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                },
                ..container::Style::default()
            }),
    )
    .interaction(if is_dragging {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    })
    .on_press(Message::StartDragCard(config.target));

    let menu_element = container(menu.view::<iced::Renderer>(theme))
        .width(Length::Fixed(config.card_width))
        .height(Length::Fixed(config.card_height));

    let card_box = column![drag_header, menu_element]
        .spacing(demo_metrics::HEADER_MENU_GAP)
        .width(Length::Fixed(config.card_width));

    container(card_box).into()
}

/// Builds the bottom status bar.
fn view_status_bar<'a>(
    state: &'a State,
    palette: &'a liquid_glass::UiPalette,
    is_dark: bool,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let status_text = state
        .last_action
        .as_deref()
        .unwrap_or("就绪。按住卡片顶部手柄 ⠿ 可拖曳至任意彩条边界，实时观测 Dual-Kawase 柔和空间扩散");

    let blur_radius = state.blur_preset.radius();
    let plan = KawasePassPlan::new(1280, 800, blur_radius);
    let deep_tier = if plan.use_deep_blur {
        "1/2→1/4→1/8"
    } else {
        "1/2→1/4"
    };

    let status_content = row![
        text("STATUS: ")
            .size(11.0)
            .font(font::ui_font(Weight::Bold))
            .color(palette.accent),
        text(status_text)
            .size(11.0)
            .font(font::ui_font(Weight::Normal))
            .color(palette.text_primary),
        space().width(Length::Fill),
        text(format!(
            "vibrancy-rs: Dual-Kawase R={blur_radius:.0}pt ({deep_tier}) · IGN Dither · BT.709 Sat+25% | 白底(深86/浅255) 黑底(深33/浅185)"
        ))
        .size(10.5)
        .font(font::ui_font(Weight::Medium))
        .color(palette.text_secondary),
    ]
    .align_y(Alignment::Center);

    container(status_content)
        .width(Length::Fill)
        .padding(Padding {
            top: 6.0,
            right: 16.0,
            bottom: 6.0,
            left: 16.0,
        })
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.08, 0.08, 0.10, 0.85)
            } else {
                Color::from_rgba(0.96, 0.96, 0.98, 0.90)
            })),
            border: Border::default().width(0.5).color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            }),
            ..container::Style::default()
        })
        .into()
}

#[must_use]
pub fn view(state: &State) -> Element<'_, Message, Theme, iced::Renderer> {
    let is_dark = state.controller.is_dark;
    let theme = &state.theme;
    let palette = &state.palette;

    // 1. Top fused header bar
    let draggable_header = view_top_header(state, palette, is_dark);

    // 2. Draggable Light & Dark menu cards over television color test blocks
    let is_light_dragging = matches!(state.active_drag, Some((DragTarget::LightCard, _)));
    let is_dark_dragging = matches!(state.active_drag, Some((DragTarget::DarkCard, _)));

    let (card_w, card_h) = state.menu_preset.menu_size();
    let (light_title, light_sub) = match state.menu_preset {
        MenuContentPreset::DockReference1To1 => ("☀️ 1:1 Dock 菜单 (浅色)", "参考截图 1:1 还原: 154×117 pt"),
        MenuContentPreset::FullShowcase => ("☀️ Light Mode Menu", "校准值: 白255 | 黑185 (α=72.5%)"),
    };
    let (dark_title, dark_sub) = match state.menu_preset {
        MenuContentPreset::DockReference1To1 => ("🌙 1:1 Dock 菜单 (深色)", "参考截图 1:1 还原: 154×117 pt"),
        MenuContentPreset::FullShowcase => ("🌙 Dark Mode Menu", "校准值: 白86 | 黑33 (α=79.2%)"),
    };

    let light_card = view_menu_card(
        MenuCardConfig {
            target: DragTarget::LightCard,
            badge_title: light_title,
            badge_sub: light_sub,
            is_dark_card: false,
            card_width: card_w,
            card_height: card_h,
        },
        &state.light_menu,
        is_light_dragging,
        theme,
        palette,
    );

    let dark_card = view_menu_card(
        MenuCardConfig {
            target: DragTarget::DarkCard,
            badge_title: dark_title,
            badge_sub: dark_sub,
            is_dark_card: true,
            card_width: card_w,
            card_height: card_h,
        },
        &state.dark_menu,
        is_dark_dragging,
        theme,
        palette,
    );

    let positioned_light = container(light_card)
        .padding(Padding {
            top: state.light_pos.y.max(10.0),
            left: state.light_pos.x.max(10.0),
            right: 0.0,
            bottom: 0.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

    let positioned_dark = container(dark_card)
        .padding(Padding {
            top: state.dark_pos.y.max(10.0),
            left: state.dark_pos.x.max(10.0),
            right: 0.0,
            bottom: 0.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

    let cards_stack = if state.top_card == DragTarget::LightCard {
        stack![positioned_dark, positioned_light]
    } else {
        stack![positioned_light, positioned_dark]
    }
    .width(Length::Fill)
    .height(Length::Fill);

    let header_height = state.controller.metrics.header_rect.height;
    let guide_height = 36.0;
    let top_offset = header_height + guide_height;

    let arrow_cfg = state.current_arrow_config();

    let mut occlusions = vec![
        MenuOcclusion {
            bounds: Rectangle {
                x: state.light_pos.x.max(10.0),
                y: state.light_pos.y.max(10.0) + demo_metrics::MENU_Y_INSET,
                width: card_w,
                height: card_h,
            },
            corner_radius: demo_metrics::MENU_CORNER_RADIUS,
            arrow: arrow_cfg,
            is_dark: false,
        },
        MenuOcclusion {
            bounds: Rectangle {
                x: state.dark_pos.x.max(10.0),
                y: state.dark_pos.y.max(10.0) + demo_metrics::MENU_Y_INSET,
                width: card_w,
                height: card_h,
            },
            corner_radius: demo_metrics::MENU_CORNER_RADIUS,
            arrow: arrow_cfg,
            is_dark: true,
        },
    ];

    if let Some(pos) = state.floating_menu {
        occlusions.push(MenuOcclusion {
            bounds: Rectangle {
                x: (pos.x - 10.0).max(10.0),
                y: (pos.y - 10.0 - top_offset).max(0.0),
                width: card_w,
                height: card_h,
            },
            corner_radius: demo_metrics::MENU_CORNER_RADIUS,
            arrow: arrow_cfg,
            is_dark: match state.resolved_floating_scheme() {
                UiColorScheme::Dark => true,
                UiColorScheme::Light => false,
            },
        });
    }

    let wallpaper_canvas = Canvas::new(WallpaperCanvas {
        style: state.wallpaper,
        blur_preset: state.blur_preset,
        occlusions,
        show_calibration_grid: state.show_calibration_grid,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    let wallpaper_layer = container(wallpaper_canvas)
        .width(Length::Fill)
        .height(Length::Fill);

    let stage_stack = stack![wallpaper_layer, cards_stack]
        .width(Length::Fill)
        .height(Length::Fill);

    // Calibration guide and reset toolbar
    let guide_bar = row![
        text("📺 电视彩色色块校准台:")
            .size(11.5)
            .font(font::ui_font(Weight::Bold))
            .color(palette.accent),
        text("按住菜单卡片顶栏 ⠿ 即可自由拖动，移至各色彩条/色块上方取色校验")
            .size(11.0)
            .font(font::ui_font(Weight::Normal))
            .color(palette.text_primary),
        space().width(Length::Fill),
        button(
            text(if state.show_calibration_grid {
                "📐 1px测试网格: 开启"
            } else {
                "📐 1px测试网格: 关闭"
            })
            .size(10.5)
            .font(font::ui_font(Weight::Medium))
            .color(palette.text_primary),
        )
        .padding(Padding {
            top: 3.0,
            right: 8.0,
            bottom: 3.0,
            left: 8.0,
        })
        .style(move |_theme, _status| button::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.10)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.06)
            })),
            border: Border::default().rounded(5.0),
            ..button::Style::default()
        })
        .on_press(Message::ToggleCalibrationGrid),
        button(
            text("↺ 重置位置")
                .size(10.5)
                .font(font::ui_font(Weight::Medium))
                .color(palette.text_primary)
        )
        .padding(Padding {
            top: 3.0,
            right: 8.0,
            bottom: 3.0,
            left: 8.0,
        })
        .style(move |_theme, _status| button::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.10)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.06)
            })),
            border: Border::default().rounded(5.0),
            ..button::Style::default()
        })
        .on_press(Message::ResetCardPositions),
    ]
    .spacing(8.0)
    .align_y(Alignment::Center)
    .padding(Padding {
        top: 4.0,
        right: 14.0,
        bottom: 4.0,
        left: 14.0,
    });

    let guide_container = container(guide_bar)
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.10, 0.10, 0.14, 0.85)
            } else {
                Color::from_rgba(0.96, 0.96, 0.98, 0.85)
            })),
            border: Border::default().width(0.5).color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            }),
            ..container::Style::default()
        });

    let page_content = if state.show_status_bar {
        column![
            draggable_header,
            guide_container,
            stage_stack,
            view_status_bar(state, palette, is_dark)
        ]
    } else {
        column![draggable_header, guide_container, stage_stack]
    }
    .width(Length::Fill)
    .height(Length::Fill);

    let mut layers: Vec<Element<'_, Message, Theme, iced::Renderer>> = vec![page_content.into()];

    // 3. Optional floating context menu at cursor
    if let Some(pos) = state.floating_menu {
        let dismiss_backdrop = iced::widget::mouse_area(
            container(space())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::DismissFloatingMenu);

        let floating_menu_widget = state.floating_menu_cached.view::<iced::Renderer>(theme);

        let positioned_menu = container(
            container(floating_menu_widget)
                .width(Length::Fixed(demo_metrics::MENU_WIDTH))
                .height(Length::Fixed(demo_metrics::MENU_HEIGHT)),
        )
        .padding(Padding {
            top: (pos.y - 10.0).max(10.0),
            left: (pos.x - 10.0).max(10.0),
            right: 0.0,
            bottom: 0.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        let overlay_stack = stack![dismiss_backdrop, positioned_menu]
            .width(Length::Fill)
            .height(Length::Fill);

        layers.push(overlay_stack.into());
    }

    let root_page = container(iced::widget::Stack::with_children(layers))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgb8(18, 18, 22)
            } else {
                Color::from_rgb8(248, 249, 251)
            })),
            border: Border {
                radius: window_metrics::DEFAULT_CORNER_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    // 4. Wrap with bmol-window-shell 8-direction resize handles and physical non-client rim
    state.controller.wrap_window_with_resizer(
        root_page,
        window_metrics::DEFAULT_CORNER_RADIUS,
        Message::ResizeWindow,
    )
}

fn main() -> iced::Result {
    let fonts = font::ui_fonts();
    let window_settings = window::Settings {
        size: Size::new(1180.0, 780.0),
        transparent: true,
        decorations: false,
        ..Default::default()
    };

    let mut app = iced::application::<State, Message, Theme, iced::Renderer>(boot, update, view)
        .title("Liquid Glass Context Menu - Window Shell Edition")
        .theme(app_theme)
        .subscription(subscription)
        .window(window_settings);

    for bytes in &fonts.bytes {
        app = app.font(bytes.clone());
    }
    if let Some(ui_font) = fonts.font() {
        app = app.default_font(ui_font);
    }
    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_defaults() {
        let state = State::default();
        assert_eq!(state.wallpaper, WallpaperStyle::TvColorBars);
        assert_eq!(state.blur_preset, BlurPreset::UltraHeavy64);
        assert_eq!(state.floating_appearance, FloatingAppearance::FollowTheme);
        assert!(state.show_status_bar);
        assert!(state.show_line_numbers);
        assert!(!state.word_wrap);
        assert_eq!(state.floating_menu, None);
        assert_eq!(state.controller.window_id, None);
        assert_eq!(state.light_pos, Point::new(70.0, 50.0));
        assert_eq!(state.dark_pos, Point::new(450.0, 50.0));
        assert_eq!(state.active_drag, None);
        assert_eq!(state.menu_preset, MenuContentPreset::DockReference1To1);
        assert_eq!(state.popover_arrow_edge, PopoverArrowEdge::Bottom);
        assert_eq!(state.arrow_preset, PopoverArrowPreset::MenuWide);
        assert_eq!(state.arrow_offset, demo_metrics::DOCK_REF_ARROW_OFFSET);
    }

    #[test]
    fn test_window_shell_controller_lifecycle() {
        let mut state = State::default();
        let test_id = window::Id::unique();

        // 1. WindowOpened associates window id
        let _ = update(&mut state, Message::WindowOpened(test_id));
        assert_eq!(state.controller.window_id, Some(test_id));

        // 2. WindowResized updates controller metrics
        let _ = update(&mut state, Message::WindowResized(Size::new(1200.0, 800.0)));
        assert_eq!(state.controller.window_size, (1200.0, 800.0));
        assert_eq!(state.controller.metrics.window_size, (1200.0, 800.0));

        // 3. Traffic lights hover interaction
        assert_eq!(state.traffic_lights.hover_target, 0.0);
        let _ = update(&mut state, Message::TrafficLightsHover(true));
        assert_eq!(state.traffic_lights.hover_target, 1.0);
        let _ = update(&mut state, Message::TrafficLightsHover(false));
        assert_eq!(state.traffic_lights.hover_target, 0.0);

        // 4. Focus/Unfocus handling
        let _ = update(
            &mut state,
            Message::WindowEvent((test_id, window::Event::Unfocused)),
        );
        assert!(!state.controller.is_focused);

        let _ = update(
            &mut state,
            Message::WindowEvent((test_id, window::Event::Focused)),
        );
        assert!(state.controller.is_focused);
    }

    #[test]
    fn test_toggle_actions_update_state() {
        let mut state = State::default();

        let _ = update(&mut state, Message::TriggerAction(MenuAction::ToggleLineNumbers));
        assert!(!state.show_line_numbers);

        let _ = update(&mut state, Message::TriggerAction(MenuAction::ToggleWordWrap));
        assert!(state.word_wrap);

        let _ = update(&mut state, Message::TriggerAction(MenuAction::ToggleStatusBar));
        assert!(!state.show_status_bar);
    }

    #[test]
    fn test_floating_menu_lifecycle() {
        let mut state = State::default();
        let target = Point::new(200.0, 150.0);

        let _ = update(&mut state, Message::OpenFloatingMenuAt(target));
        assert_eq!(state.floating_menu, Some(target));

        let _ = update(&mut state, Message::DismissFloatingMenu);
        assert_eq!(state.floating_menu, None);

        let _ = update(&mut state, Message::CursorMoved(Point::new(350.0, 400.0)));
        let _ = update(&mut state, Message::RightClicked);
        assert_eq!(state.floating_menu, Some(Point::new(350.0, 400.0)));
    }

    #[test]
    fn test_theme_and_wallpaper_switching() {
        let mut state = State::default();
        let orig_theme = state.controller.is_dark;

        let _ = update(&mut state, Message::ToggleColorScheme);
        assert_ne!(state.controller.is_dark, orig_theme);

        let _ = update(&mut state, Message::SetWallpaper(WallpaperStyle::TvSmpteSplit));
        assert_eq!(state.wallpaper, WallpaperStyle::TvSmpteSplit);

        let _ = update(&mut state, Message::SetBlurPreset(BlurPreset::Heavy48));
        assert_eq!(state.blur_preset, BlurPreset::Heavy48);
        assert_eq!(state.blur_preset.radius(), 48.0);
    }

    #[test]
    fn test_card_dragging_lifecycle() {
        let mut state = State::default();
        state.cursor_pos = Point::new(100.0, 80.0);

        // 1. Start dragging light card
        let _ = update(&mut state, Message::StartDragCard(DragTarget::LightCard));
        assert!(state.active_drag.is_some());
        assert_eq!(state.top_card, DragTarget::LightCard);

        // 2. Cursor moved updates position
        let _ = update(&mut state, Message::CursorMoved(Point::new(160.0, 140.0)));
        assert_eq!(state.light_pos, Point::new(130.0, 110.0));

        // 3. End drag fixes card in place
        let _ = update(&mut state, Message::EndDragCard);
        assert!(state.active_drag.is_none());
        assert_eq!(state.light_pos, Point::new(130.0, 110.0));

        // 4. Reset positions restores defaults
        let _ = update(&mut state, Message::ResetCardPositions);
        assert_eq!(state.light_pos, Point::new(70.0, 50.0));
        assert_eq!(state.dark_pos, Point::new(450.0, 50.0));
    }

    #[test]
    fn test_demo_menu_building_and_rendering() {
        let mut state = State::default();
        state.rebuild_menus();
        let menu_dock = build_demo_menu(&state);
        assert_eq!(menu_dock.len(), 5);

        let theme_dark = app_theme(&state);
        let elem_dark = menu_dock.view::<iced::Renderer>(&theme_dark);
        drop(elem_dark);

        // Switch to full showcase and verify rendering
        let _ = update(&mut state, Message::ToggleMenuPreset);
        state.rebuild_menus();
        let menu_full = build_demo_menu(&state);
        assert_eq!(menu_full.len(), 22);

        let mut light_state = State::default();
        light_state.controller.set_dark_mode(false);
        let _ = update(&mut light_state, Message::ToggleMenuPreset);
        light_state.rebuild_menus();
        let theme_light = app_theme(&light_state);
        let elem_light = menu_full.view::<iced::Renderer>(&theme_light);
        drop(elem_light);
    }

    #[test]
    fn test_measured_colorimetry_expectations() {
        let (dark_r, _, _, dark_a) = menu_metrics::DARK_MENU_BASE_RGBA_F32;
        let (light_r, _, _, light_a) = menu_metrics::LIGHT_MENU_BASE_RGBA_F32;

        // Dark on white: 86
        let dark_on_white = (dark_r * 255.0 * dark_a + 255.0 * (1.0 - dark_a)).round() as u8;
        assert_eq!(dark_on_white, 86);

        // Dark on black: 33
        let dark_on_black = (dark_r * 255.0 * dark_a).round() as u8;
        assert_eq!(dark_on_black, 33);

        // Light on white: 255
        let light_on_white = (light_r * 255.0 * light_a + 255.0 * (1.0 - light_a)).round() as u8;
        assert_eq!(light_on_white, 255);

        // Light on black: 185
        let light_on_black = (light_r * 255.0 * light_a).round() as u8;
        assert_eq!(light_on_black, 185);
    }

    #[test]
    fn test_tv_color_bars_multi_color_calibration() {
        let (dark_r, _, _, dark_a) = menu_metrics::DARK_MENU_BASE_RGBA_F32;
        let (light_r, _, _, light_a) = menu_metrics::LIGHT_MENU_BASE_RGBA_F32;

        // 8 TV Color Bars: (Name, [R, G, B])
        let tv_bars: &[(&str, [u8; 3], [u8; 3], [u8; 3])] = &[
            // (Bar, Background RGB, Expected Dark Mode RGB, Expected Light Mode RGB)
            ("Pure White", [255, 255, 255], [86, 86, 86], [255, 255, 255]),
            ("Yellow",     [255, 255,   0], [86, 86, 33], [255, 255, 185]),
            ("Cyan",       [  0, 255, 255], [33, 86, 86], [185, 255, 255]),
            ("Green",      [  0, 255,   0], [33, 86, 33], [185, 255, 185]),
            ("Magenta",    [255,   0, 255], [86, 33, 86], [255, 185, 255]),
            ("Red",        [255,   0,   0], [86, 33, 33], [255, 185, 185]),
            ("Blue",       [  0,   0, 255], [33, 33, 86], [185, 185, 255]),
            ("Pure Black", [  0,   0,   0], [33, 33, 33], [185, 185, 185]),
        ];

        for &(name, bg, exp_dark, exp_light) in tv_bars {
            let calc_dark_r = (dark_r * 255.0 * dark_a + f32::from(bg[0]) * (1.0 - dark_a)).round() as u8;
            let calc_dark_g = (dark_r * 255.0 * dark_a + f32::from(bg[1]) * (1.0 - dark_a)).round() as u8;
            let calc_dark_b = (dark_r * 255.0 * dark_a + f32::from(bg[2]) * (1.0 - dark_a)).round() as u8;
            assert_eq!([calc_dark_r, calc_dark_g, calc_dark_b], exp_dark, "Dark mode on {}", name);

            let calc_light_r = (light_r * 255.0 * light_a + f32::from(bg[0]) * (1.0 - light_a)).round() as u8;
            let calc_light_g = (light_r * 255.0 * light_a + f32::from(bg[1]) * (1.0 - light_a)).round() as u8;
            let calc_light_b = (light_r * 255.0 * light_a + f32::from(bg[2]) * (1.0 - light_a)).round() as u8;
            assert_eq!([calc_light_r, calc_light_g, calc_light_b], exp_light, "Light mode on {}", name);
        }
    }

    #[test]
    fn test_vibrancy_dual_kawase_sampling_and_grid_toggle() {
        let mut state = State::default();
        assert!(state.show_calibration_grid);

        // 1. Toggle high-frequency calibration grid
        let _ = update(&mut state, Message::ToggleCalibrationGrid);
        assert!(!state.show_calibration_grid);
        let _ = update(&mut state, Message::ToggleCalibrationGrid);
        assert!(state.show_calibration_grid);

        // 2. Vibrancy PassPlan metrics
        let blur_64 = BlurPreset::UltraHeavy64.radius();
        let plan_64 = KawasePassPlan::new(1280, 800, blur_64);
        assert!(plan_64.use_deep_blur, "64pt must trigger 1/8 deep blur tier");
        assert!(plan_64.offset >= 1.2 && plan_64.offset <= 2.5);

        // 3. Pure color consistency verification
        let white_color = sample_analytical_blurred_wallpaper(
            WallpaperStyle::PureWhite,
            100.0,
            100.0,
            Size::new(800.0, 600.0),
            blur_64,
        );
        assert_eq!(white_color, Color::WHITE);

        let black_color = sample_analytical_blurred_wallpaper(
            WallpaperStyle::PureBlack,
            100.0,
            100.0,
            Size::new(800.0, 600.0),
            blur_64,
        );
        assert_eq!(black_color, Color::BLACK);

        // 4. Color bar boundary dispersion (yellow x=100.0 to cyan x=200.0)
        // Bar 1 is yellow [1, 1, 0], Bar 2 is cyan [0, 1, 1] on an 800-wide viewport (100px per bar)
        let boundary_sample = sample_analytical_blurred_wallpaper(
            WallpaperStyle::TvColorBars,
            200.0, // Exactly at Yellow/Cyan boundary
            300.0,
            Size::new(800.0, 600.0),
            blur_64,
        );
        // At boundary, both red and blue channels are non-zero due to continuous Gaussian dispersion
        assert!(boundary_sample.r > 0.05, "Yellow's red channel dispersed across boundary");
        assert!(boundary_sample.b > 0.05, "Cyan's blue channel dispersed across boundary");
        assert!(boundary_sample.g > 0.8, "Green channel remains high for both yellow and cyan");
    }

    #[test]
    fn test_demo_menu_height_geometry_exactness() {
        // 1. Verify Dock 1:1 Reference Preset
        let state = State::default();
        let menu_dock = build_demo_menu(&state);
        let items_dock = menu_dock.items_slice();
        assert_eq!(items_dock.len(), 5, "Dock 1:1 reference menu must have exactly 5 items");

        let mut dock_actions = 0;
        let mut dock_separators = 0;
        for item in items_dock {
            match item {
                MenuItem::Separator => dock_separators += 1,
                MenuItem::Action { .. } | MenuItem::Submenu { .. } => dock_actions += 1,
                _ => {}
            }
        }
        assert_eq!(dock_actions, 4);
        assert_eq!(dock_separators, 1);

        let dock_items_height = dock_actions as f32 * menu_metrics::ITEM_HEIGHT;
        let dock_separators_height = dock_separators as f32 * (menu_metrics::SEPARATOR_HEIGHT + menu_metrics::SEPARATOR_MARGIN_V * 2.0);
        let dock_gaps = (items_dock.len() - 1) as f32 * 1.0;
        let dock_padding = menu_metrics::CONTAINER_PADDING * 2.0;
        let dock_total = dock_items_height + dock_separators_height + dock_gaps + dock_padding;
        assert_eq!(dock_total, demo_metrics::DOCK_REF_HEIGHT);

        // 2. Verify Full Showcase Preset
        let mut full_state = State::default();
        let _ = update(&mut full_state, Message::ToggleMenuPreset);
        assert_eq!(full_state.menu_preset, MenuContentPreset::FullShowcase);

        let menu_full = build_demo_menu(&full_state);
        let items_full = menu_full.items_slice();
        assert_eq!(items_full.len(), 22, "Full showcase menu must have exactly 22 items");

        let mut section_count = 0;
        let mut separator_count = 0;
        let mut action_count = 0;

        for item in items_full {
            match item {
                MenuItem::Section(_) => section_count += 1,
                MenuItem::Separator => separator_count += 1,
                MenuItem::Action { .. } | MenuItem::Checkbox { .. } | MenuItem::Submenu { .. } => {
                    action_count += 1;
                }
            }
        }

        assert_eq!(section_count, 5);
        assert_eq!(separator_count, 4);
        assert_eq!(action_count, 13);

        let expected_sections = section_count as f32 * menu_metrics::SECTION_HEADER_HEIGHT;
        let expected_items = action_count as f32 * menu_metrics::ITEM_HEIGHT;
        let expected_separators = separator_count as f32 * (menu_metrics::SEPARATOR_HEIGHT + menu_metrics::SEPARATOR_MARGIN_V * 2.0);
        let expected_gaps = (items_full.len() - 1) as f32 * 1.0;
        let container_padding = menu_metrics::CONTAINER_PADDING * 2.0;
        let total_exact = expected_sections + expected_items + expected_separators + expected_gaps + container_padding;

        assert_eq!(
            total_exact,
            demo_metrics::MENU_HEIGHT,
            "demo_metrics::MENU_HEIGHT must exactly equal total item stack height to avoid transparent occlusion bottom gaps"
        );
        assert_eq!(demo_metrics::MENU_HEIGHT, 477.0);
        assert_eq!(demo_metrics::MENU_Y_INSET, 44.0);
    }

    #[test]
    fn test_toggle_menu_preset() {
        let mut state = State::default();
        assert_eq!(state.menu_preset, MenuContentPreset::DockReference1To1);
        assert_eq!(state.popover_arrow_edge, PopoverArrowEdge::Bottom);
        assert_eq!(state.arrow_offset, demo_metrics::DOCK_REF_ARROW_OFFSET);

        // 1. Toggle to FullShowcase
        let _ = update(&mut state, Message::ToggleMenuPreset);
        assert_eq!(state.menu_preset, MenuContentPreset::FullShowcase);
        assert_eq!(state.arrow_offset, 0.5);

        // 2. Toggle back to DockReference1To1
        let _ = update(&mut state, Message::ToggleMenuPreset);
        assert_eq!(state.menu_preset, MenuContentPreset::DockReference1To1);
        assert_eq!(state.popover_arrow_edge, PopoverArrowEdge::Bottom);
        assert_eq!(state.arrow_offset, demo_metrics::DOCK_REF_ARROW_OFFSET);
    }

    #[test]
    fn test_cycle_arrow_preset_lifecycle() {
        let mut state = State::default();
        assert_eq!(state.arrow_preset, PopoverArrowPreset::MenuWide);

        // 1. MenuWide -> TooltipNarrow
        let _ = update(&mut state, Message::CycleArrowPreset);
        assert_eq!(state.arrow_preset, PopoverArrowPreset::TooltipNarrow);
        let cfg_narrow = state.current_arrow_config();
        assert_eq!(cfg_narrow.base_width, 16.0);
        assert_eq!(cfg_narrow.height, 7.0);
        assert_eq!(cfg_narrow.tip_radius, 2.0);

        // 2. TooltipNarrow -> AppKitStandard
        let _ = update(&mut state, Message::CycleArrowPreset);
        assert_eq!(state.arrow_preset, PopoverArrowPreset::AppKitStandard);
        let cfg_std = state.current_arrow_config();
        assert_eq!(cfg_std.base_width, 27.5);
        assert_eq!(cfg_std.height, 13.0);
        assert_eq!(cfg_std.tip_radius, 5.0);

        // 3. AppKitStandard -> SubtleCompact
        let _ = update(&mut state, Message::CycleArrowPreset);
        assert_eq!(state.arrow_preset, PopoverArrowPreset::SubtleCompact);
        let cfg_subtle = state.current_arrow_config();
        assert_eq!(cfg_subtle.base_width, 12.0);
        assert_eq!(cfg_subtle.height, 5.0);

        // 4. SubtleCompact -> MenuWide
        let _ = update(&mut state, Message::CycleArrowPreset);
        assert_eq!(state.arrow_preset, PopoverArrowPreset::MenuWide);
        let cfg_wide = state.current_arrow_config();
        assert_eq!(cfg_wide.base_width, 26.0);
        assert_eq!(cfg_wide.height, 10.0);
        assert_eq!(cfg_wide.tip_radius, 5.0);
    }

    #[test]
    fn test_squircle_continuous_curvature_integration() {
        let rect = Rectangle {
            x: 50.0,
            y: 50.0,
            width: demo_metrics::MENU_WIDTH,
            height: demo_metrics::MENU_HEIGHT,
        };
        let r = demo_metrics::MENU_CORNER_RADIUS;

        // 1. Verify SquircleParams generates non-empty Apple continuous commands
        let params = SquircleParams::new(rect.width, rect.height, r)
            .with_smoothing(APPLE_CORNER_SMOOTHING);
        let commands = squircle_path_commands(&params);
        assert!(!commands.is_empty(), "Squircle commands must not be empty");

        // 2. Verify Apple smoothing exponent matches tagged library default (3.32)
        let exp_n = 2.0 + (4.2 - 2.0) * APPLE_CORNER_SMOOTHING;
        assert!((exp_n - 3.32).abs() < 1e-4);

        // 3. Verify corner curvature inset smoothly vanishes towards center
        let inv_exp_n = 1.0 / exp_n;
        let r_pow_n = r.powf(exp_n);

        // At corner tip (dx = r)
        let dy_tip = (r_pow_n - r.powf(exp_n)).max(0.0).powf(inv_exp_n);
        let inset_tip = r - dy_tip;
        assert!((inset_tip - r).abs() < 1e-4, "Corner tip inset must equal full radius");

        // Midway through corner (dx = r * 0.5)
        let dx_mid = r * 0.5;
        let dy_mid = (r_pow_n - dx_mid.powf(exp_n)).max(0.0).powf(inv_exp_n);
        let inset_mid = r - dy_mid;
        // In G2 squircle, inset_mid is significantly smoother than circle
        assert!(inset_mid > 0.0 && inset_mid < r * 0.5);
    }

    #[test]
    fn test_set_popover_arrow_message_handling() {
        let mut state = State::default();
        assert_eq!(state.popover_arrow_edge, PopoverArrowEdge::Bottom);

        let edges = [
            (PopoverArrowEdge::Top, "▲ 顶部触角 (Top)"),
            (PopoverArrowEdge::Left, "◀ 左侧触角 (Left)"),
            (PopoverArrowEdge::Right, "▶ 右侧触角 (Right)"),
            (PopoverArrowEdge::None, "无触角"),
            (PopoverArrowEdge::Bottom, "▼ 底部触角 (Bottom)"),
        ];

        for (edge, expected_text) in edges {
            let _ = update(&mut state, Message::SetPopoverArrow(edge));
            assert_eq!(state.popover_arrow_edge, edge);
            assert!(
                state.last_action.as_ref().unwrap().contains(expected_text),
                "Status should describe {:?}",
                edge
            );
        }
    }

    #[test]
    fn test_popover_arrow_path_geometry() {
        let rect = Rectangle {
            x: 100.0,
            y: 100.0,
            width: demo_metrics::MENU_WIDTH,
            height: demo_metrics::MENU_HEIGHT,
        };
        let r = demo_metrics::MENU_CORNER_RADIUS;

        let all_edges = [
            PopoverArrowEdge::None,
            PopoverArrowEdge::Top,
            PopoverArrowEdge::Bottom,
            PopoverArrowEdge::Left,
            PopoverArrowEdge::Right,
        ];

        for edge in all_edges {
            let config = PopoverArrowConfig::new(edge);
            if edge == PopoverArrowEdge::None {
                assert!(!config.is_visible());
            } else {
                assert!(config.is_visible());
            }

            let path = build_popover_squircle_path(rect, r, config);
            drop(path);

            // Verify with modified shadow spread size
            let shadow_config = config.with_size(config.base_width + 10.0, config.height + 5.0);
            assert_eq!(shadow_config.base_width, config.base_width + 10.0);
            assert_eq!(shadow_config.height, config.height + 5.0);
            let shadow_path = build_popover_squircle_path(rect, r + 5.0, shadow_config);
            drop(shadow_path);
        }
    }
}

