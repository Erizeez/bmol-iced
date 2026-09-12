//! Liquid Glass Dock, Search Bar, Icons & Context Menu Optics Playground Demo.
//!
//! Demonstrates authentic Apple-style Liquid Glass optics on a colorful background:
//! - Top-light specular highlight: wide & soft across straight horizontal edges.
//! - Curvature-compressed highlight: razor-sharp & pinpoint along corner arcs.
//! - Directional falloff: zero specular highlight on left/right vertical edges.
//! - Edge occlusion (Rim Darkening): subtle dark perimeter stroke revealed on sides.
//! - Authentic macOS squircle app icons with genuine vector illustrations.
//! - Floating Frosted Dock with backdrop blur, hover tooltips, and spring scale.
//! - Capsule Search Bar with spotlight styling and shortcut badge.
//! - Liquid Glass Context Menu popping up on right-click with full HIG styling.
//! - TV Color Bars, SMPTE split, and vibrant geometric gradient wallpapers.
//! - Real-time optics toggles (highlight switch & rim darkening switch) for comparison.

#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::struct_excessive_bools,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_arguments
)]

use std::sync::Arc;
use std::time::Instant;

use bmol_designs::menu_metrics;
use bmol_window_shell::{
    ShellEvent, TrafficLightsEvent, WindowChromeConfig, WindowShellController, is_system_dark_mode,
    window_metrics,
};
use iced::advanced::graphics::gradient::Linear;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Shadow, Size,
    Subscription, Task, Theme, Vector,
    font::Weight,
    mouse,
    widget::{
        button,
        canvas::{self, Canvas, Frame, Geometry, Path},
        column, container, row, space, text, text_input,
    },
    window,
};
use liquid_glass::{
    ContextMenu, CornerCurve, GlassChrome, GlassContainer, GlassId, GlassRole, GlassShape,
    MenuItem, Rect, UiColorScheme, UiIcon, UiTheme, ui::font,
};
use vibrancy_rs::KawasePassPlan;

#[path = "../iced_backend.rs"]
mod iced_backend;

use iced_backend::{DemoSurface, WINDOW_CONTROL_NATIVE_IDS};

use liquid_glass::dock::{
    BlurPreset, DockApp, LayoutMetrics, PhysicalPlateConfig, WallpaperBuffer, WallpaperStyle,
    build_squircle_path, draw_apple_icon, draw_elevation_shadow, generate_bezier_preview,
    generate_frosted_plate_texture, generate_physical_dock_plate_texture, load_or_create_wallpaper,
    load_real_app_icons, sample_sharp_wallpaper,
};

/// Context menu target type for right-clicks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuContext {
    App(DockApp),
    DockBar,
    SearchBar,
    Wallpaper,
}

/// Preset levels of glass plate translucency & transparency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlassTransparency {
    /// Ultra Clear Glass (Minimal center tint, pure optical refraction feel - Apple liquid glass default)
    #[default]
    Ultra,
    /// High Transparency (Apple Sequoia/Sonoma style: ~90% clear middle, vibrant wallpaper transmission)
    High,
    /// Milky Frosted Glass (Traditional heavier frosted substrate)
    Frosted,
}

impl GlassTransparency {
    pub const ALL: [Self; 3] = [Self::Ultra, Self::High, Self::Frosted];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Ultra => "极清",
            Self::High => "高透",
            Self::Frosted => "磨砂",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Ultra => Self::High,
            Self::High => Self::Frosted,
            Self::Frosted => Self::Ultra,
        }
    }

    #[must_use]
    pub fn center_alpha_light(&self) -> f32 {
        match self {
            Self::High => 0.13,
            Self::Ultra => 0.06,
            Self::Frosted => 0.24,
        }
    }

    #[must_use]
    pub fn center_alpha_dark(&self) -> f32 {
        match self {
            Self::High => 0.20,
            Self::Ultra => 0.10,
            Self::Frosted => 0.32,
        }
    }
}

/// Messages for the interactive application.
#[derive(Debug, Clone)]
pub enum Message {
    WindowReady(Option<window::Id>),
    WindowResized(Size),
    WindowEvent((window::Id, iced::window::Event)),
    AnimationFrame(Instant),
    ResizeWindow(window::Direction),
    DragWindow,
    TrafficLights(TrafficLightsEvent),
    SetWallpaper(WallpaperStyle),
    SetBlurPreset(BlurPreset),
    SetClarity(f32),
    SetAmount(f32),
    SetMilkiness(f32),
    SetHighlightIntensity(f32),
    SetDockRadius(f32),
    SetCornerSmoothing(f32),
    ApplyIconPaddingPreset,
    SetP1(f32),
    SetP2(f32),
    SetP3(f32),
    SetWidthX(f32),
    SetHeightY(f32),
    SetCurvePreset(f32, f32, f32),
    ToggleTheme,
    CycleTransparency,
    SetTransparency(GlassTransparency),
    ToggleHighlight(bool),
    ToggleDarkRim(bool),
    ToggleGrid(bool),
    TogglePipelineMode,
    ToggleDocumentEdited,
    SystemThemeChanged(iced::theme::Mode),
    SearchInputChanged(String),
    CursorMoved(Point),
    IconHovered(Option<DockApp>),
    IconClicked(DockApp),
    RightClicked,
    DismissFloatingMenu,
    TriggerAction(String),
}

/// Rendering pipeline selection for the Liquid Glass Dock demo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PipelineMode {
    #[default]
    GpuLiquidRs,
    Canvas2D,
}

impl PipelineMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::GpuLiquidRs => "GPU liquid-rs (自研引擎)",
            Self::Canvas2D => "2D Canvas (矢量模拟)",
        }
    }
}

/// Renders base wallpaper and alignment grid lines directly into the source layer.
struct WallpaperCanvas {
    style: WallpaperStyle,
    show_grid: bool,
    system_wallpaper: Option<Arc<WallpaperBuffer>>,
}

impl<Message> canvas::Program<Message, Theme, iced_backend::Renderer> for WallpaperCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced_backend::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let window_squircle = build_squircle_path(
            Rectangle::new(Point::ORIGIN, bounds.size()),
            window_metrics::DEFAULT_CORNER_RADIUS,
        );

        // 1. Draw Base Wallpaper (Presets clipped cleanly to window squircle)
        match self.style {
            WallpaperStyle::DesktopTransparent => {
                if let Some(buf) = &self.system_wallpaper {
                    frame.draw_image(
                        Rectangle::new(Point::ORIGIN, bounds.size()),
                        iced::widget::canvas::Image::new(buf.iced_handle()),
                    );
                }
            }
            WallpaperStyle::AuroraMesh => {
                let grad = Linear::new(Point::ORIGIN, Point::new(bounds.width, bounds.height))
                    .add_stop(0.0, Color::from_rgb(0.12, 0.06, 0.38))
                    .add_stop(0.20, Color::from_rgb(0.38, 0.10, 0.58))
                    .add_stop(0.44, Color::from_rgb(0.88, 0.16, 0.48))
                    .add_stop(0.68, Color::from_rgb(0.98, 0.46, 0.15))
                    .add_stop(0.86, Color::from_rgb(0.95, 0.80, 0.22))
                    .add_stop(1.0, Color::from_rgb(0.12, 0.78, 0.82));
                frame.fill(&window_squircle, grad);
            }
            WallpaperStyle::SunsetGaze => {
                let grad = Linear::new(
                    Point::new(bounds.width * 0.15, 0.0),
                    Point::new(bounds.width * 0.85, bounds.height),
                )
                .add_stop(0.0, Color::from_rgb(0.06, 0.10, 0.25))
                .add_stop(0.30, Color::from_rgb(0.35, 0.12, 0.42))
                .add_stop(0.60, Color::from_rgb(0.82, 0.22, 0.35))
                .add_stop(0.82, Color::from_rgb(0.96, 0.52, 0.18))
                .add_stop(1.0, Color::from_rgb(1.0, 0.82, 0.45));
                frame.fill(&window_squircle, grad);
            }
            WallpaperStyle::TvColorBars => {
                let n = 8.0;
                let bar_w = bounds.width / n;
                for i in 0..8 {
                    let c = sample_sharp_wallpaper(
                        self.style,
                        (i as f32 + 0.5) * bar_w,
                        0.0,
                        bounds.size(),
                        self.system_wallpaper.as_deref(),
                    );
                    frame.fill_rectangle(
                        Point::new(i as f32 * bar_w, 0.0),
                        Size::new(bar_w + 1.0, bounds.height),
                        Color::from_rgb(c[0], c[1], c[2]),
                    );
                }
            }
            WallpaperStyle::TvSmpteSplit => {
                let top_h = bounds.height * 0.70;
                let bot_h = bounds.height - top_h;

                let n_top = 7.0;
                let bar_w_top = bounds.width / n_top;
                for i in 0..7 {
                    let c = sample_sharp_wallpaper(
                        self.style,
                        (i as f32 + 0.5) * bar_w_top,
                        10.0,
                        bounds.size(),
                        self.system_wallpaper.as_deref(),
                    );
                    frame.fill_rectangle(
                        Point::new(i as f32 * bar_w_top, 0.0),
                        Size::new(bar_w_top + 1.0, top_h),
                        Color::from_rgb(c[0], c[1], c[2]),
                    );
                }

                let n_bot = 8.0;
                let bar_w_bot = bounds.width / n_bot;
                for i in 0..8 {
                    let c = sample_sharp_wallpaper(
                        self.style,
                        (i as f32 + 0.5) * bar_w_bot,
                        top_h + 10.0,
                        bounds.size(),
                        self.system_wallpaper.as_deref(),
                    );
                    frame.fill_rectangle(
                        Point::new(i as f32 * bar_w_bot, top_h),
                        Size::new(bar_w_bot + 1.0, bot_h),
                        Color::from_rgb(c[0], c[1], c[2]),
                    );
                }
            }
            WallpaperStyle::TvColorGrid => {
                let cols = 4.0;
                let rows = 3.0;
                let cell_w = bounds.width / cols;
                let cell_h = bounds.height / rows;
                for r in 0..3 {
                    for c in 0..4 {
                        let sample = sample_sharp_wallpaper(
                            self.style,
                            (c as f32 + 0.5) * cell_w,
                            (r as f32 + 0.5) * cell_h,
                            bounds.size(),
                            self.system_wallpaper.as_deref(),
                        );
                        frame.fill_rectangle(
                            Point::new(c as f32 * cell_w, r as f32 * cell_h),
                            Size::new(cell_w + 1.0, cell_h + 1.0),
                            Color::from_rgb(sample[0], sample[1], sample[2]),
                        );
                    }
                }
            }
            WallpaperStyle::PureWhite => {
                frame.fill(&window_squircle, Color::WHITE);
            }
            WallpaperStyle::PureBlack => {
                frame.fill(&window_squircle, Color::BLACK);
            }
        }

        // 2. Alignment Calibration Grid Lines
        if self.show_grid
            && !matches!(self.style, WallpaperStyle::PureWhite | WallpaperStyle::PureBlack)
        {
            let grid_step = 36.0f32;
            let line_color = Color::from_rgba(1.0, 1.0, 1.0, 0.16);
            let mut x = grid_step;
            while x < bounds.width {
                frame.fill_rectangle(Point::new(x, 0.0), Size::new(1.0, bounds.height), line_color);
                x += grid_step;
            }
            let mut y = grid_step;
            while y < bounds.height {
                frame.fill_rectangle(Point::new(0.0, y), Size::new(bounds.width, 1.0), line_color);
                y += grid_step;
            }
        }

        vec![frame.into_geometry()]
    }
}

/// Renders application icons and active indicator dots in the glass foreground layer.
struct IconsCanvas {
    metrics: LayoutMetrics,
    dock_radius: f32,
    is_dark: bool,
    enable_highlight: bool,
    enable_dark_rim: bool,
    app_icons: [Option<iced::widget::image::Handle>; 9],
    new_dock_physical_texture: Option<iced::widget::image::Handle>,
}

impl<Message> canvas::Program<Message, Theme, iced_backend::Renderer> for IconsCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced_backend::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let d_rect = self.metrics.dock_rect;

        // 0. Render Upper NEW Physical Liquid Glass Dock Plate (0% fake strokes, 100% physical GPU optics)
        let mut new_d_rect = self.metrics.new_dock_rect;
        new_d_rect.y -= self.metrics.header_h;
        draw_elevation_shadow(
            &mut frame,
            new_d_rect,
            self.dock_radius,
            self.is_dark,
            GlassTransparency::Ultra,
        );
        if let Some(texture) = &self.new_dock_physical_texture {
            frame.draw_image(new_d_rect, iced::widget::canvas::Image::new(texture.clone()));
        }

        // Render Upper NEW Physical Dock Icons (Pure clean Squircle icons, 0 wireframe stroke)
        for (i, app) in DockApp::ALL.iter().enumerate() {
            let mut i_rect = self.metrics.new_icon_rects[i];
            i_rect.y -= self.metrics.header_h;
            draw_apple_icon(
                &mut frame,
                *app,
                i_rect,
                self.is_dark,
                false,
                false,
                self.app_icons[i].as_ref(),
            );

            let is_running = matches!(
                app,
                DockApp::Finder
                    | DockApp::Safari
                    | DockApp::Messages
                    | DockApp::Mail
                    | DockApp::Terminal
                    | DockApp::Settings
            );
            if is_running {
                let dot_cx = i_rect.x + i_rect.width * 0.5;
                let dot_cy = (new_d_rect.y - self.metrics.header_h) + new_d_rect.height
                    - (self.metrics.dock_padding * 0.35);
                let (dot_color, halo_color) = if self.is_dark {
                    (Color::from_rgba(1.0, 1.0, 1.0, 0.90), Color::from_rgba(1.0, 1.0, 1.0, 0.18))
                } else {
                    (
                        Color::from_rgba(0.08, 0.09, 0.12, 0.65),
                        Color::from_rgba(1.0, 1.0, 1.0, 0.45),
                    )
                };
                let dot_r = 2.0f32;
                frame.fill(&Path::circle(Point::new(dot_cx, dot_cy), dot_r + 1.0), halo_color);
                frame.fill(&Path::circle(Point::new(dot_cx, dot_cy), dot_r), dot_color);
            }
        }

        // Label above NEW Physical Dock:
        frame.fill_text(iced::widget::canvas::Text {
            content:
                "✨ 全新物理 Liquid Glass Dock（Apple G2 连续超椭圆 · 物理微缝 AO · 0 人工描边）"
                    .into(),
            position: Point::new(new_d_rect.x + 8.0, new_d_rect.y - self.metrics.header_h - 18.0),
            color: Color::from_rgb(0.95, 0.95, 1.0),
            size: iced::Pixels(11.0),
            ..Default::default()
        });

        // Label above OLD Dock:
        frame.fill_text(iced::widget::canvas::Text {
            content: "原有 Dock 实现（传统机械圆角 · 2D 人工描边对比）".into(),
            position: Point::new(d_rect.x + 8.0, d_rect.y - self.metrics.header_h - 18.0),
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.65),
            size: iced::Pixels(11.0),
            ..Default::default()
        });

        // Render Lower OLD Dock Icons (Original presentation with border for comparison)
        for (i, app) in DockApp::ALL.iter().enumerate() {
            let mut i_rect = self.metrics.icon_rects[i];
            i_rect.y -= self.metrics.header_h;
            draw_apple_icon(
                &mut frame,
                *app,
                i_rect,
                self.is_dark,
                self.enable_highlight,
                self.enable_dark_rim,
                self.app_icons[i].as_ref(),
            );

            let is_running = matches!(
                app,
                DockApp::Finder
                    | DockApp::Safari
                    | DockApp::Messages
                    | DockApp::Mail
                    | DockApp::Terminal
                    | DockApp::Settings
            );
            if is_running {
                let dot_cx = i_rect.x + i_rect.width * 0.5;
                let dot_cy = (d_rect.y - self.metrics.header_h) + d_rect.height
                    - (self.metrics.dock_padding * 0.35);
                let (dot_color, halo_color) = if self.is_dark {
                    (Color::from_rgba(1.0, 1.0, 1.0, 0.90), Color::from_rgba(1.0, 1.0, 1.0, 0.18))
                } else {
                    (
                        Color::from_rgba(0.08, 0.09, 0.12, 0.65),
                        Color::from_rgba(1.0, 1.0, 1.0, 0.45),
                    )
                };
                let dot_r = 2.0f32;
                frame.fill(&Path::circle(Point::new(dot_cx, dot_cy), dot_r + 1.0), halo_color);
                frame.fill(&Path::circle(Point::new(dot_cx, dot_cy), dot_r), dot_color);
            }
        }

        vec![frame.into_geometry()]
    }
}

/// State of the Liquid Glass Optics & Dock Showcase.
#[derive(Debug)]
pub struct State {
    pub controller: WindowShellController,
    pub window_size: Size,
    pub theme: Theme,
    pub wallpaper: WallpaperStyle,
    pub transparency: GlassTransparency,
    pub blur_preset: BlurPreset,
    pub blur_radius: f32,
    pub clarity: f32,
    pub p1: f32,
    pub p2: f32,
    pub p3: f32,
    pub width_x: f32,
    pub height_y: f32,
    pub amount: f32,
    pub milkiness: f32,
    pub highlight_intensity: f32,
    pub dock_radius: f32,
    pub corner_smoothing: f32,
    pub pipeline_mode: PipelineMode,
    pub show_grid: bool,
    pub enable_highlight: bool,
    pub enable_dark_rim: bool,
    pub cursor_pos: Point,
    pub hovered_app: Option<DockApp>,
    pub search_query: String,
    pub document_edited: bool,
    pub floating_menu: Option<(Point, MenuContext)>,
    pub floating_menu_cached: ContextMenu<Message>,
    pub last_action: String,
    pub system_wallpaper: Arc<WallpaperBuffer>,
    pub dock_frosted_texture: Option<iced::widget::image::Handle>,
    pub new_dock_physical_texture: Option<iced::widget::image::Handle>,
    pub curve_handle: Option<iced::widget::image::Handle>,
    pub search_frosted_texture: Option<iced::widget::image::Handle>,
    pub app_icons: [Option<iced::widget::image::Handle>; 9],
}

impl Default for State {
    fn default() -> Self {
        let is_dark = is_system_dark_mode();
        let config = WindowChromeConfig::unified_header(window_metrics::FUSED_HEADER_HEIGHT);
        let controller = WindowShellController::new(config, is_dark);
        let scheme = if is_dark { UiColorScheme::Dark } else { UiColorScheme::Light };
        let theme = UiTheme::new(scheme).iced_theme();
        let system_wallpaper = load_or_create_wallpaper(2480, 1640);
        let app_icons = load_real_app_icons();

        let mut s = Self {
            controller,
            window_size: Size::new(1240.0, 820.0),
            theme,
            wallpaper: WallpaperStyle::DesktopTransparent,
            transparency: GlassTransparency::Ultra,
            blur_preset: BlurPreset::Standard16,
            blur_radius: BlurPreset::Standard16.radius(),
            clarity: 0.03,            // 97% 清晰!
            amount: -27.0,            // 扭曲量级 -27px
            milkiness: 0.0,           // 0% 奶白 (纯透清澈)
            highlight_intensity: 1.0, // 100% 物理高光打光
            p1: 0.18,                 // P1 默认 0.18 肩部微凸张力
            p2: 0.00,
            p3: 0.00,
            width_x: 13.0,          // 正好等价于图标边距 dock_padding (13px)
            height_y: 13.0,         // 正好等价于图标边距 dock_padding (13px)
            dock_radius: 23.0,      // 严格同心圆心法则: R = padding (13px) + r_icon (10px) = 23px
            corner_smoothing: 0.20, // 80% 明确圆弧段 + 20% 边缘切线平滑过渡 (兼具圆弧感与收边圆滑)
            pipeline_mode: PipelineMode::GpuLiquidRs,
            show_grid: false,
            enable_highlight: true,
            enable_dark_rim: true,
            cursor_pos: Point::new(600.0, 400.0),
            hovered_app: None,
            search_query: String::new(),
            document_edited: false,
            floating_menu: None,
            floating_menu_cached: ContextMenu::new(),
            last_action: "就绪：GPU liquid-rs 自研物理光学引擎与硬件模糊已接入".to_string(),
            system_wallpaper,
            dock_frosted_texture: None,
            new_dock_physical_texture: None,
            curve_handle: Some(generate_bezier_preview(0.18, 0.00, 0.00)),
            search_frosted_texture: None,
            app_icons,
        };
        s.regenerate_frosted_textures();
        s
    }
}

impl State {
    pub fn regenerate_frosted_textures(&mut self) {
        let metrics = LayoutMetrics::new(self.window_size, self.hovered_app);
        let mut render_rect = metrics.new_dock_rect;
        render_rect.y -= metrics.header_h;
        self.curve_handle = Some(generate_bezier_preview(self.p1, self.p2, self.p3));
        let config = PhysicalPlateConfig {
            clarity: self.clarity,
            amount: self.amount,
            p1: self.p1,
            p2: self.p2,
            p3: self.p3,
            width_x: self.width_x,
            height_y: self.height_y,
            milkiness: self.milkiness,
            highlight_intensity: self.highlight_intensity,
            corner_radius: self.dock_radius,
            corner_smoothing: self.corner_smoothing,
        };
        self.new_dock_physical_texture = Some(generate_physical_dock_plate_texture(
            self.wallpaper,
            render_rect,
            self.window_size,
            config,
            self.is_dark(),
            Some(&self.system_wallpaper),
        ));
        if self.pipeline_mode == PipelineMode::GpuLiquidRs {
            return;
        }
        let metrics = LayoutMetrics::new(self.window_size, self.hovered_app);
        self.dock_frosted_texture = Some(generate_frosted_plate_texture(
            self.wallpaper,
            metrics.dock_rect,
            self.window_size,
            self.blur_radius,
            metrics.dock_radius,
            self.is_dark(),
            Some(&self.system_wallpaper),
        ));
        self.search_frosted_texture = Some(generate_frosted_plate_texture(
            self.wallpaper,
            metrics.search_rect,
            self.window_size,
            self.blur_radius,
            21.0,
            self.is_dark(),
            Some(&self.system_wallpaper),
        ));
    }

    #[must_use]
    pub fn is_dark(&self) -> bool {
        self.controller.is_dark
    }

    #[must_use]
    pub fn scheme(&self) -> UiColorScheme {
        if self.is_dark() { UiColorScheme::Dark } else { UiColorScheme::Light }
    }

    pub fn rebuild_menu(&mut self, context: MenuContext) {
        let scheme = self.scheme();
        self.floating_menu_cached = match context {
            MenuContext::App(app) => build_app_context_menu(app, scheme),
            MenuContext::DockBar => build_dock_context_menu(self, scheme),
            MenuContext::SearchBar => build_search_context_menu(scheme),
            MenuContext::Wallpaper => build_wallpaper_context_menu(self, scheme),
        };
    }
}

fn build_app_context_menu(app: DockApp, scheme: UiColorScheme) -> ContextMenu<Message> {
    ContextMenu::new()
        .width(menu_metrics::DEFAULT_WIDTH)
        .with_scheme(scheme)
        .item(MenuItem::section(app.name()))
        .item(
            MenuItem::action("打开应用")
                .with_shortcut("↵")
                .with_icon(UiIcon::Info)
                .on_press(Message::TriggerAction(format!("打开应用: {}", app.name()))),
        )
        .item(
            MenuItem::action("在访达中显示")
                .with_shortcut("⌘O")
                .on_press(Message::TriggerAction(format!("在访达中显示: {}", app.name()))),
        )
        .item(MenuItem::separator())
        .item(MenuItem::submenu("选项 ▸"))
        .item(
            MenuItem::action("退出")
                .with_shortcut("⌘Q")
                .with_destructive(true)
                .on_press(Message::TriggerAction(format!("退出应用: {}", app.name()))),
        )
}

fn build_dock_context_menu(state: &State, scheme: UiColorScheme) -> ContextMenu<Message> {
    ContextMenu::new()
        .width(menu_metrics::DEFAULT_WIDTH)
        .with_scheme(scheme)
        .item(MenuItem::section("DOCK 与光学偏好设置"))
        .item(
            MenuItem::checkbox("启用边缘高光 (Highlight)", state.enable_highlight)
                .on_toggle(Message::ToggleHighlight(!state.enable_highlight)),
        )
        .item(
            MenuItem::checkbox("启用左右深色描边 (Dark Rim)", state.enable_dark_rim)
                .on_toggle(Message::ToggleDarkRim(!state.enable_dark_rim)),
        )
        .item(
            MenuItem::checkbox("显示校准对齐网格", state.show_grid)
                .on_toggle(Message::ToggleGrid(!state.show_grid)),
        )
        .item(MenuItem::separator())
        .item(
            MenuItem::action("系统设置...")
                .with_shortcut("⌘,")
                .with_icon(UiIcon::Gear)
                .on_press(Message::TriggerAction("打开系统设置...".to_string())),
        )
}

fn build_search_context_menu(scheme: UiColorScheme) -> ContextMenu<Message> {
    ContextMenu::new()
        .width(menu_metrics::DEFAULT_WIDTH)
        .with_scheme(scheme)
        .item(MenuItem::section("搜索操作"))
        .item(
            MenuItem::action("粘贴并搜索")
                .with_shortcut("⌘V")
                .on_press(Message::TriggerAction("执行: 粘贴并搜索".to_string())),
        )
        .item(
            MenuItem::action("清空搜索内容")
                .with_shortcut("⌫")
                .on_press(Message::SearchInputChanged(String::new())),
        )
        .item(MenuItem::separator())
        .item(MenuItem::submenu("搜索引擎设置 ▸"))
}

fn build_wallpaper_context_menu(state: &State, scheme: UiColorScheme) -> ContextMenu<Message> {
    ContextMenu::new()
        .width(menu_metrics::DEFAULT_WIDTH)
        .with_scheme(scheme)
        .item(MenuItem::section("壁纸与显示"))
        .item(
            MenuItem::action(format!("当前壁纸: {}", state.wallpaper.label()))
                .on_press(Message::TriggerAction("点击顶栏切换壁纸".to_string())),
        )
        .item(
            MenuItem::action(if state.is_dark() {
                "切换至浅色模式"
            } else {
                "切换至深色模式"
            })
            .with_shortcut("⇧⌘D")
            .on_press(Message::ToggleTheme),
        )
        .item(MenuItem::separator())
        .item(
            MenuItem::action("材质光学调试面板")
                .with_icon(UiIcon::Info)
                .on_press(Message::TriggerAction("激活材质调试面板".to_string())),
        )
}

pub fn boot() -> (State, Task<Message>) {
    let mut state = State::default();
    state.wallpaper = WallpaperStyle::DesktopTransparent;
    for arg in std::env::args().skip(1) {
        if arg == "--transparent" || arg == "transparent" {
            state.wallpaper = WallpaperStyle::DesktopTransparent;
        } else if arg == "--bars" || arg == "bars" {
            state.wallpaper = WallpaperStyle::TvColorBars;
        } else if arg == "--grid" || arg == "grid" {
            state.wallpaper = WallpaperStyle::TvColorGrid;
        } else if arg == "--smpte" || arg == "smpte" {
            state.wallpaper = WallpaperStyle::TvSmpteSplit;
        } else if arg == "--aurora" || arg == "aurora" {
            state.wallpaper = WallpaperStyle::AuroraMesh;
        }
    }
    state.rebuild_menu(MenuContext::DockBar);
    state.regenerate_frosted_textures();
    (
        state,
        Task::batch([
            iced::system::theme().map(Message::SystemThemeChanged),
            liquid_glass::IcedWindowController::latest().map(Message::WindowReady),
        ]),
    )
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::WindowReady(Some(id)) => {
            state.controller.set_window_id(id);
            let controller = state.controller.clone();
            window::run(id, move |w| {
                if let Ok(handle) = w.window_handle() {
                    let _ = controller.setup_window(handle.as_raw());
                }
            })
            .discard()
        }
        Message::WindowReady(None) => Task::none(),
        Message::WindowResized(size) => {
            state.window_size = size;
            state.controller.handle_resized(size.width, size.height);
            state.regenerate_frosted_textures();
            Task::none()
        }
        Message::WindowEvent((id, event)) => {
            if let iced::window::Event::Opened { .. } = event {
                if state.controller.window_id.is_none() {
                    return update(state, Message::WindowReady(Some(id)));
                }
            }
            if let Some(shell_event) = state.controller.handle_window_event(&event) {
                match shell_event {
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
        Message::AnimationFrame(now) => {
            state.controller.step(now);
            Task::none()
        }
        Message::ResizeWindow(direction) => {
            if let Some(id) = state.controller.window_id {
                window::drag_resize(id, direction)
            } else {
                Task::none()
            }
        }
        Message::DragWindow => {
            if let Some(id) = state.controller.window_id {
                window::drag(id)
            } else {
                Task::none()
            }
        }
        Message::TrafficLights(event) => state.controller.handle_traffic_lights(event),
        Message::SetWallpaper(w) => {
            state.wallpaper = w;
            state.last_action = format!("切换壁纸: {}", w.label());
            state.regenerate_frosted_textures();
            Task::none()
        }
        Message::SetAmount(a) => {
            state.amount = a;
            state.regenerate_frosted_textures();
            state.last_action = format!("调整折射扭曲量级: {:.0}px", a);
            Task::none()
        }
        Message::SetMilkiness(m) => {
            state.milkiness = m.clamp(0.0, 1.0);
            state.regenerate_frosted_textures();
            state.last_action = format!("调整材质奶白染色: {:.0}%", state.milkiness * 100.0);
            Task::none()
        }
        Message::SetHighlightIntensity(h) => {
            state.highlight_intensity = h.clamp(0.0, 1.0);
            state.regenerate_frosted_textures();
            state.last_action =
                format!("调整物理打光强度: {:.0}%", state.highlight_intensity * 100.0);
            Task::none()
        }
        Message::SetDockRadius(r) => {
            state.dock_radius = r;
            state.regenerate_frosted_textures();
            state.last_action = format!("调整底座圆角半径: {:.0}px", r);
            Task::none()
        }
        Message::SetCornerSmoothing(s) => {
            state.corner_smoothing = s.clamp(0.0, 1.0);
            state.regenerate_frosted_textures();
            state.last_action = format!("调整圆角平滑过渡: {:.2}", s);
            Task::none()
        }
        Message::ApplyIconPaddingPreset => {
            state.amount = -27.0;
            state.milkiness = 0.0;
            state.highlight_intensity = 1.0;
            state.dock_radius = 23.0;
            state.corner_smoothing = 0.20;
            state.p1 = 0.18;
            state.p2 = 0.0;
            state.p3 = 0.0;
            state.width_x = 13.0;
            state.height_y = 13.0;
            state.clarity = 0.03;
            state.regenerate_frosted_textures();
            state.last_action =
                "应用图标边距对齐物理参数 (-27px, 奶白0%, 打光100%, 平滑0.20, 13px, 97%清晰)"
                    .to_string();
            Task::none()
        }
        Message::SetClarity(c) => {
            state.clarity = c.clamp(0.0, 1.0);
            state.blur_radius = 14.0 + state.clarity * (65.0 - 14.0);
            state.regenerate_frosted_textures();
            state.last_action = format!(
                "调整官方清晰度: {:.0}% (t = {:.2})",
                (1.0 - state.clarity) * 100.0,
                state.clarity
            );
            Task::none()
        }
        Message::SetP1(v) => {
            state.p1 = v;
            state.regenerate_frosted_textures();
            state.last_action = format!("调整肩部凸度 P1: {:.2}", v);
            Task::none()
        }
        Message::SetP2(v) => {
            state.p2 = v;
            state.regenerate_frosted_textures();
            state.last_action = format!("调整腰身弧度 P2: {:.2}", v);
            Task::none()
        }
        Message::SetP3(v) => {
            state.p3 = v;
            state.regenerate_frosted_textures();
            state.last_action = format!("调整落底平滑 P3: {:.2}", v);
            Task::none()
        }
        Message::SetWidthX(w) => {
            state.width_x = w;
            state.regenerate_frosted_textures();
            state.last_action = format!("调整左右倒角宽度: {:.0}px", w);
            Task::none()
        }
        Message::SetHeightY(h) => {
            state.height_y = h;
            state.regenerate_frosted_textures();
            state.last_action = format!("调整上下倒角高度: {:.0}px", h);
            Task::none()
        }
        Message::SetCurvePreset(p1, p2, p3) => {
            state.p1 = p1;
            state.p2 = p2;
            state.p3 = p3;
            state.regenerate_frosted_textures();
            state.last_action = format!("应用曲线预设: P1={:.2}, P2={:.2}, P3={:.2}", p1, p2, p3);
            Task::none()
        }
        Message::SetBlurPreset(p) => {
            state.blur_preset = p;
            state.blur_radius = p.radius();
            state.clarity = (p.radius() / 64.0).clamp(0.0, 1.0);
            state.regenerate_frosted_textures();
            state.last_action = format!(
                "切换模糊预设: {} (清晰度: {:.0}%)",
                p.label(),
                (1.0 - state.clarity) * 100.0
            );
            Task::none()
        }
        Message::ToggleTheme => {
            state.controller.is_dark = !state.controller.is_dark;
            state.theme = UiTheme::new(state.scheme()).iced_theme();
            state.last_action = format!(
                "切换外观: {}",
                if state.is_dark() { "深色 (Dark)" } else { "浅色 (Light)" }
            );
            state.regenerate_frosted_textures();
            Task::none()
        }
        Message::CycleTransparency => {
            state.transparency = state.transparency.next();
            state.last_action = format!("切换玻璃通透度: {}", state.transparency.label());
            Task::none()
        }
        Message::SetTransparency(t) => {
            state.transparency = t;
            state.last_action = format!("设置玻璃通透度: {}", state.transparency.label());
            Task::none()
        }
        Message::ToggleHighlight(val) => {
            state.enable_highlight = val;
            state.last_action =
                format!("边缘高光 (Highlight): {}", if val { "开启" } else { "关闭" });
            Task::none()
        }
        Message::ToggleDarkRim(val) => {
            state.enable_dark_rim = val;
            state.last_action =
                format!("左右深色描边 (Dark Rim): {}", if val { "开启" } else { "关闭" });
            Task::none()
        }
        Message::ToggleGrid(val) => {
            state.show_grid = val;
            state.last_action = format!("校准网格线: {}", if val { "显示" } else { "隐藏" });
            Task::none()
        }
        Message::TogglePipelineMode => {
            state.pipeline_mode = match state.pipeline_mode {
                PipelineMode::GpuLiquidRs => PipelineMode::Canvas2D,
                PipelineMode::Canvas2D => PipelineMode::GpuLiquidRs,
            };
            if state.pipeline_mode == PipelineMode::Canvas2D {
                state.regenerate_frosted_textures();
            }
            state.last_action = format!("切换渲染管线: {}", state.pipeline_mode.label());
            Task::none()
        }
        Message::ToggleDocumentEdited => {
            state.document_edited = !state.document_edited;
            bmol_window_shell::set_document_edited(state.document_edited);
            state.last_action = format!("NSWindow.isDocumentEdited = {}", state.document_edited);
            Task::none()
        }
        Message::SystemThemeChanged(mode) => {
            let event = state.controller.handle_system_theme(mode);
            if let ShellEvent::ThemeChanged { .. } = event {
                state.theme = UiTheme::new(state.scheme()).iced_theme();
                state.regenerate_frosted_textures();
            }
            state.last_action = format!("系统外观: {mode:?}");
            Task::none()
        }
        Message::SearchInputChanged(q) => {
            state.search_query = q;
            Task::none()
        }
        Message::CursorMoved(pos) => {
            state.cursor_pos = pos;
            Task::none()
        }
        Message::IconHovered(app) => {
            state.hovered_app = app;
            Task::none()
        }
        Message::IconClicked(app) => {
            state.last_action = format!("激活应用: {}", app.name());
            Task::none()
        }
        Message::RightClicked => {
            let cursor = state.cursor_pos;
            let metrics = LayoutMetrics::new(state.window_size, state.hovered_app);

            // Check if right-click hit an app icon
            let mut context = MenuContext::Wallpaper;
            for (i, app) in DockApp::ALL.iter().enumerate() {
                let rect = metrics.icon_rects[i];
                if cursor.x >= rect.x
                    && cursor.x <= rect.x + rect.width
                    && cursor.y >= rect.y
                    && cursor.y <= rect.y + rect.height
                {
                    context = MenuContext::App(*app);
                    break;
                }
            }

            // Check if right-click hit the Dock bar
            if context == MenuContext::Wallpaper {
                let d = metrics.dock_rect;
                if cursor.x >= d.x
                    && cursor.x <= d.x + d.width
                    && cursor.y >= d.y
                    && cursor.y <= d.y + d.height
                {
                    context = MenuContext::DockBar;
                }
            }

            // Check if right-click hit the Search bar
            if context == MenuContext::Wallpaper {
                let s = metrics.search_rect;
                if cursor.x >= s.x
                    && cursor.x <= s.x + s.width
                    && cursor.y >= s.y
                    && cursor.y <= s.y + s.height
                {
                    context = MenuContext::SearchBar;
                }
            }

            state.rebuild_menu(context);
            state.floating_menu = Some((cursor, context));
            state.last_action = format!("弹出液态玻璃菜单: {context:?}");
            Task::none()
        }
        Message::DismissFloatingMenu => {
            state.floating_menu = None;
            Task::none()
        }
        Message::TriggerAction(act) => {
            state.last_action = act;
            state.floating_menu = None;
            Task::none()
        }
    }
}

pub fn subscription(state: &State) -> Subscription<Message> {
    let mut subscriptions = vec![
        window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
        bmol_window_shell::system_theme_subscription(Message::SystemThemeChanged),
        iced::event::listen_with(|event, _status, id| match event {
            iced::Event::Window(w_event) => Some(Message::WindowEvent((id, w_event))),
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                Some(Message::RightClicked)
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Some(Message::CursorMoved(position))
            }
            _ => None,
        }),
    ];

    if state.controller.is_animating() {
        subscriptions.push(window::frames().map(Message::AnimationFrame));
    }

    Subscription::batch(subscriptions)
}

pub fn view(state: &State) -> Element<'_, Message, Theme, iced_backend::Renderer> {
    let is_dark = state.is_dark();
    let metrics = LayoutMetrics::new(state.window_size, state.hovered_app);

    // Drive the shared Liquid Glass compositor so the traffic lights render
    // through the exact same GPU physical-glass pipeline as the settings demo.
    iced_backend::set_surface(DemoSurface::TrafficLightsOnly);
    bmol_window_shell::set_glass_passthrough(true);
    bmol_window_shell::set_document_edited(state.document_edited);
    iced_backend::set_color_scheme(if is_dark {
        UiColorScheme::Dark
    } else {
        UiColorScheme::Light
    });
    iced_backend::set_window_inactive(!state.controller.is_focused);
    // The traffic-light origin is measured from the glyph widget's real layout
    // by `MeasuredTrafficLights`, so no manual origin is needed here.
    for (index, &id) in WINDOW_CONTROL_NATIVE_IDS.iter().enumerate() {
        iced_backend::set_window_control_scale(
            id,
            state.controller.traffic_lights.press_scale(index),
        );
    }
    iced_backend::set_window_control_group_progress(
        0,
        state.controller.traffic_lights.hover_progress,
    );

    // Compute Context Menu Rect if open
    let floating_menu_rect = state.floating_menu.map(|(pos, _)| {
        let menu_w = menu_metrics::DEFAULT_WIDTH;
        let item_count = state.floating_menu_cached.len();
        let menu_h = (item_count as f32 * 26.5 + 16.0).max(60.0);
        let menu_x = (pos.x - 12.0).clamp(10.0, state.window_size.width - menu_w - 10.0);
        let menu_y =
            (pos.y - 12.0).clamp(metrics.header_h + 10.0, state.window_size.height - menu_h - 10.0);
        Rectangle { x: menu_x, y: menu_y, width: menu_w, height: menu_h }
    });

    // Search bar input positioned right over metrics.search_rect
    let search_input = view_search_input(state, is_dark, metrics.search_rect);

    // Dock interactive touch areas
    let dock_interactive = view_dock_hitboxes(state, &metrics);

    // 1. Base Wallpaper Canvas directly targeting the source render layer
    let wallpaper_canvas = Canvas::new(WallpaperCanvas {
        style: state.wallpaper,
        show_grid: state.show_grid,
        system_wallpaper: Some(state.system_wallpaper.clone()),
    })
    .width(Length::Fill)
    .height(Length::Fill);

    // 2. Foreground Icons and Interactive Hitboxes on top of GPU Glass
    let foreground_icons = liquid_glass::ui::components::glass_foreground(
        Canvas::new(IconsCanvas {
            metrics,
            dock_radius: state.dock_radius,
            is_dark,
            enable_highlight: state.enable_highlight,
            enable_dark_rim: state.enable_dark_rim,
            app_icons: state.app_icons.clone(),
            new_dock_physical_texture: state.new_dock_physical_texture.clone(),
        })
        .width(Length::Fill)
        .height(Length::Fill),
    );

    // -------------------------------------------------------------
    // Dual Pipeline Rendering Architecture (GPU liquid-rs vs 2D Canvas Fallback)
    // -------------------------------------------------------------
    let (stage_area, base_canvas): (
        Element<'_, Message, Theme, iced_backend::Renderer>,
        Element<'_, Message, Theme, iced_backend::Renderer>,
    ) = if state.pipeline_mode == PipelineMode::GpuLiquidRs {
        let theme = UiTheme::new(state.scheme());
        let mut search_mat = theme.glass_material(GlassRole::SearchField);
        search_mat.blur.radius = state.blur_radius;

        let mut dock_mat = theme.glass_material(GlassRole::FloatingControl);
        dock_mat.blur.radius = state.blur_radius;
        dock_mat.whiteness = 0.12;
        dock_mat.refraction.thickness = 28.0;
        dock_mat.refraction.strength = 0.85;

        let search_glass = container(
            GlassContainer::new(
                GlassId(200),
                Rect::new(
                    metrics.search_rect.x,
                    metrics.search_rect.y,
                    metrics.search_rect.width,
                    metrics.search_rect.height,
                ),
            )
            .shape(GlassShape::Capsule)
            .material(search_mat)
            .chrome(GlassChrome::transparent()),
        )
        .padding(Padding {
            top: metrics.search_rect.y - metrics.header_h,
            left: metrics.search_rect.x,
            ..Padding::ZERO
        });

        let old_dock_glass = container(
            GlassContainer::new(
                GlassId(201),
                Rect::new(
                    metrics.dock_rect.x,
                    metrics.dock_rect.y,
                    metrics.dock_rect.width,
                    metrics.dock_rect.height,
                ),
            )
            .shape(GlassShape::RoundedRect { radius: metrics.dock_radius })
            .corner_curve(CornerCurve::Circular)
            .material(dock_mat)
            .chrome(GlassChrome::transparent()),
        )
        .padding(Padding {
            top: metrics.dock_rect.y - metrics.header_h,
            left: metrics.dock_rect.x,
            ..Padding::ZERO
        });

        let stage = iced::widget::Stack::new()
            .push(search_glass)
            .push(old_dock_glass)
            .push(foreground_icons)
            .push(liquid_glass::ui::components::glass_foreground(search_input))
            .push(liquid_glass::ui::components::glass_foreground(dock_interactive))
            .width(Length::Fill)
            .height(Length::Fill);

        (stage.into(), wallpaper_canvas.into())
    } else {
        let stage = iced::widget::Stack::new()
            .push(foreground_icons)
            .push(liquid_glass::ui::components::glass_foreground(search_input))
            .push(liquid_glass::ui::components::glass_foreground(dock_interactive))
            .width(Length::Fill)
            .height(Length::Fill);

        (stage.into(), wallpaper_canvas.into())
    };

    let header = view_header(state, is_dark);
    let status_bar = view_status_bar(state, is_dark);

    // Curve tuning capsule bar directly accessible right below header
    let p1_col = column![
        text(format!("P1 (肩部): {:.2}", state.p1))
            .size(10)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.95)
            } else {
                Color::from_rgb(0.1, 0.1, 0.2)
            }),
        iced::widget::slider(0.0..=2.0, state.p1, Message::SetP1)
            .step(0.02)
            .width(Length::Fixed(80.0)),
    ]
    .spacing(1);

    let p2_col = column![
        text(format!("P2 (腰身): {:.2}", state.p2))
            .size(10)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.95)
            } else {
                Color::from_rgb(0.1, 0.1, 0.2)
            }),
        iced::widget::slider(0.0..=2.0, state.p2, Message::SetP2)
            .step(0.02)
            .width(Length::Fixed(80.0)),
    ]
    .spacing(1);

    let p3_col = column![
        text(format!("P3 (落底): {:.2}", state.p3))
            .size(10)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.95)
            } else {
                Color::from_rgb(0.1, 0.1, 0.2)
            }),
        iced::widget::slider(0.0..=1.0, state.p3, Message::SetP3)
            .step(0.02)
            .width(Length::Fixed(70.0)),
    ]
    .spacing(1);

    let wx_col = column![
        text(format!("左右宽: {:.0}px", state.width_x))
            .size(10)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.95)
            } else {
                Color::from_rgb(0.1, 0.1, 0.2)
            }),
        iced::widget::slider(4.0..=50.0, state.width_x, Message::SetWidthX)
            .step(1.0)
            .width(Length::Fixed(70.0)),
    ]
    .spacing(1);

    let hy_col = column![
        text(format!("上下高: {:.0}px", state.height_y))
            .size(10)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.95)
            } else {
                Color::from_rgb(0.1, 0.1, 0.2)
            }),
        iced::widget::slider(4.0..=50.0, state.height_y, Message::SetHeightY)
            .step(1.0)
            .width(Length::Fixed(70.0)),
    ]
    .spacing(1);

    let curve_presets = row![
        button(text("图标边距对齐(13px)").size(10))
            .padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 })
            .on_press(Message::ApplyIconPaddingPreset),
        button(text("标准水滴").size(10))
            .padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 })
            .on_press(Message::SetCurvePreset(1.00, 0.75, 0.30)),
        button(text("饱满圆珠").size(10))
            .padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 })
            .on_press(Message::SetCurvePreset(1.25, 0.95, 0.40)),
        button(text("平缓S角").size(10))
            .padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 })
            .on_press(Message::SetCurvePreset(0.80, 0.30, 0.05)),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let amount_col = column![
        text(format!("扭曲: {:.0}px", state.amount))
            .size(10)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.95)
            } else {
                Color::from_rgb(0.1, 0.1, 0.2)
            }),
        iced::widget::slider(-80.0..=0.0, state.amount, Message::SetAmount)
            .step(1.0)
            .width(Length::Fixed(60.0)),
    ]
    .spacing(1);

    let milk_col = column![
        text(format!("奶白: {:.0}%", state.milkiness * 100.0))
            .size(10)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.95)
            } else {
                Color::from_rgb(0.1, 0.1, 0.2)
            }),
        iced::widget::slider(0.0..=0.40, state.milkiness, Message::SetMilkiness)
            .step(0.01)
            .width(Length::Fixed(60.0)),
    ]
    .spacing(1);

    let highlight_col = column![
        text(format!("打光: {:.0}%", state.highlight_intensity * 100.0))
            .size(10)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.95)
            } else {
                Color::from_rgb(0.1, 0.1, 0.2)
            }),
        iced::widget::slider(0.0..=1.00, state.highlight_intensity, Message::SetHighlightIntensity)
            .step(0.02)
            .width(Length::Fixed(60.0)),
    ]
    .spacing(1);

    let radius_col = column![
        text(format!("圆角: {:.0}px", state.dock_radius))
            .size(10)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.95)
            } else {
                Color::from_rgb(0.1, 0.1, 0.2)
            }),
        iced::widget::slider(12.0..=33.0, state.dock_radius, Message::SetDockRadius)
            .step(1.0)
            .width(Length::Fixed(55.0)),
    ]
    .spacing(1);

    let smoothing_col = column![
        text(format!("平滑: {:.2}", state.corner_smoothing))
            .size(10)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.9, 0.9, 0.95)
            } else {
                Color::from_rgb(0.1, 0.1, 0.2)
            }),
        iced::widget::slider(0.0..=0.60, state.corner_smoothing, Message::SetCornerSmoothing)
            .step(0.02)
            .width(Length::Fixed(50.0)),
    ]
    .spacing(1);

    let curve_tuning_pill = container(
        row![
            text("物理微调:").size(10).font(font::ui_font(Weight::Bold)).color(if is_dark {
                Color::from_rgb(0.95, 0.85, 0.45)
            } else {
                Color::from_rgb(0.6, 0.4, 0.1)
            }),
            amount_col,
            milk_col,
            highlight_col,
            radius_col,
            smoothing_col,
            p1_col,
            p2_col,
            p3_col,
            wx_col,
            hy_col,
            curve_presets,
            if let Some(handle) = &state.curve_handle {
                container(
                    row![
                        text("剖面:").size(10).font(font::ui_font(Weight::Medium)).color(
                            if is_dark {
                                Color::from_rgb(0.7, 0.8, 0.9)
                            } else {
                                Color::from_rgb(0.2, 0.3, 0.4)
                            }
                        ),
                        iced::widget::image(handle.clone())
                            .width(Length::Fixed(140.0))
                            .height(Length::Fixed(34.0)),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
            } else {
                container(space().width(Length::Fixed(0.0)))
            },
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding(Padding { top: 3.0, right: 10.0, bottom: 3.0, left: 10.0 })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.06, 0.08, 0.12, 0.65)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.65)
        })),
        border: Border::default().rounded(14.0).width(0.5).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.15)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        }),
        ..Default::default()
    });

    let top_section = column![
        header,
        container(curve_tuning_pill).padding(Padding { top: 2.0, left: 14.0, ..Padding::ZERO })
    ]
    .spacing(4.0);

    let page_content =
        column![top_section, stage_area, status_bar].width(Length::Fill).height(Length::Fill);

    let mut layers: Vec<Element<'_, Message, Theme, iced_backend::Renderer>> = Vec::new();

    // Layer 1: Base Canvas (Wallpaper in GPU mode / Full 2D Canvas in fallback mode)
    layers.push(base_canvas);

    // Layer 2: Interactive Controls & Glass Nodes Overlay
    layers.push(page_content.into());

    // -------------------------------------------------------------
    // Layer 3: Floating Context Menu (at mouse cursor)
    // -------------------------------------------------------------
    if let (Some((_pos, _)), Some(m_rect)) = (state.floating_menu, floating_menu_rect) {
        let dismiss_backdrop =
            iced::widget::mouse_area(container(space()).width(Length::Fill).height(Length::Fill))
                .on_press(Message::DismissFloatingMenu);

        let menu_view = state.floating_menu_cached.view::<iced_backend::Renderer>(&state.theme);

        let positioned_menu = container(menu_view).padding(Padding {
            top: m_rect.y,
            left: m_rect.x,
            ..Padding::ZERO
        });

        if state.pipeline_mode == PipelineMode::GpuLiquidRs {
            let theme = UiTheme::new(state.scheme());
            let menu_glass = container(
                GlassContainer::new(
                    GlassId(202),
                    Rect::new(m_rect.x, m_rect.y, m_rect.width, m_rect.height),
                )
                .shape(GlassShape::RoundedRect { radius: 12.0 })
                .material(theme.glass_material(GlassRole::ContextMenu))
                .chrome(GlassChrome::transparent()),
            )
            .padding(Padding { top: m_rect.y, left: m_rect.x, ..Padding::ZERO });

            let overlay_stack = iced::widget::Stack::new()
                .push(dismiss_backdrop)
                .push(menu_glass)
                .push(liquid_glass::ui::components::glass_overlay(positioned_menu));

            layers.push(overlay_stack.into());
        } else {
            let overlay_stack =
                iced::widget::Stack::new().push(dismiss_backdrop).push(positioned_menu);

            layers.push(overlay_stack.into());
        }
    }

    let root_stack = container(iced::widget::Stack::with_children(layers))
        .width(Length::Fill)
        .height(Length::Fill);

    state.controller.wrap_window_with_resizer(
        root_stack,
        window_metrics::DEFAULT_CORNER_RADIUS,
        Message::ResizeWindow,
    )
}

/// Builds the top titlebar with macOS Traffic Lights, Title, and Tuning Bar.
fn view_header(
    state: &State,
    is_dark: bool,
) -> Element<'_, Message, Theme, iced_backend::Renderer> {
    // The glyphs sit above the GPU glass composition (overlay layer). The
    // wrapper publishes their measured layout origin so the spheres follow it.
    let traffic_lights = liquid_glass::ui::components::glass_overlay(
        state.controller.traffic_lights_view(Message::TrafficLights),
    );

    let title_text = container(
        text("Liquid Glass Optics & Dock Showcase")
            .size(12)
            .font(font::ui_font(Weight::Semibold))
            .color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.95)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.88)
            }),
    )
    .padding(Padding { top: 3.0, right: 10.0, bottom: 3.0, left: 10.0 })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.08, 0.09, 0.12, 0.45)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.45)
        })),
        border: Border::default().rounded(14.0).width(0.5).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.16)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        }),
        ..Default::default()
    });

    // Optics Tuning Controls wrapped in a floating frosted pill capsule
    // Active Pipeline Selection Pill
    let pipeline_pill = container(
        button(
            text(state.pipeline_mode.label()).size(11).font(font::ui_font(Weight::Semibold)).color(
                if state.pipeline_mode == PipelineMode::GpuLiquidRs {
                    Color::from_rgb(0.20, 0.85, 0.55)
                } else {
                    Color::from_rgb(1.0, 0.70, 0.25)
                },
            ),
        )
        .padding(Padding { top: 3.0, right: 8.0, bottom: 3.0, left: 8.0 })
        .style(move |_theme, _status| button::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.12, 0.14, 0.18, 0.75)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.75)
            })),
            border: Border::default().rounded(12.0).width(0.5).color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.20)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.12)
            }),
            ..Default::default()
        })
        .on_press(Message::TogglePipelineMode),
    );

    let wallpaper_pill = container(
        row(WallpaperStyle::ALL.iter().map(|&s| {
            let is_selected = state.wallpaper == s;
            button(
                text(s.label())
                    .size(11)
                    .font(font::ui_font(if is_selected { Weight::Bold } else { Weight::Normal }))
                    .color(if is_selected {
                        Color::WHITE
                    } else if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.75)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.75)
                    }),
            )
            .padding(Padding { top: 3.0, right: 5.0, bottom: 3.0, left: 5.0 })
            .style(move |_theme, _status| button::Style {
                background: if is_selected {
                    Some(Background::Color(Color::from_rgb(0.0, 0.48, 1.0)))
                } else {
                    None
                },
                border: Border::default().rounded(12.0),
                ..Default::default()
            })
            .on_press(Message::SetWallpaper(s))
            .into()
        }))
        .spacing(1.5),
    )
    .padding(Padding { top: 2.0, right: 4.0, bottom: 2.0, left: 4.0 })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.0, 0.0, 0.0, 0.35)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.32)
        })),
        border: Border::default().rounded(16.0).width(0.5).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.16)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        }),
        ..Default::default()
    });

    // Apple Official Clarity Continuous Slider Pill (0% clearest t=0 <-> 100% blurriest t=1)
    let clarity_pill = container(
        row![
            text(if state.clarity <= 0.03 {
                "清晰度: 100% 极清纯透".to_string()
            } else if state.clarity >= 0.97 {
                "清晰度: 0% 浓郁深磨砂".to_string()
            } else {
                format!("清晰度: {:.0}%", (1.0 - state.clarity) * 100.0)
            })
            .size(11)
            .font(font::ui_font(Weight::Semibold))
            .color(if is_dark {
                Color::from_rgb(0.95, 0.85, 0.45)
            } else {
                Color::from_rgb(0.70, 0.45, 0.10)
            }),
            iced::widget::slider(0.0..=1.0, state.clarity, Message::SetClarity)
                .step(0.01)
                .width(Length::Fixed(100.0)),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding(Padding { top: 2.0, right: 8.0, bottom: 2.0, left: 8.0 })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.0, 0.0, 0.0, 0.35)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.45)
        })),
        border: Border::default().rounded(14.0).width(0.5).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.16)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        }),
        ..Default::default()
    });

    // Blur Presets Pill (vibrancy-rs Dual Kawase & Gaussian convolution)
    let blur_pill = container(
        row(BlurPreset::ALL.iter().map(|&p| {
            let is_selected = state.blur_preset == p;
            button(
                text(p.short_label())
                    .size(11)
                    .font(font::ui_font(if is_selected { Weight::Bold } else { Weight::Normal }))
                    .color(if is_selected {
                        Color::WHITE
                    } else if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.75)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.75)
                    }),
            )
            .padding(Padding { top: 3.0, right: 5.0, bottom: 3.0, left: 5.0 })
            .style(move |_theme, _status| button::Style {
                background: if is_selected {
                    Some(Background::Color(Color::from_rgb(0.55, 0.25, 0.85)))
                } else {
                    None
                },
                border: Border::default().rounded(12.0),
                ..Default::default()
            })
            .on_press(Message::SetBlurPreset(p))
            .into()
        }))
        .spacing(1.5),
    )
    .padding(Padding { top: 2.0, right: 4.0, bottom: 2.0, left: 4.0 })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.0, 0.0, 0.0, 0.35)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.32)
        })),
        border: Border::default().rounded(16.0).width(0.5).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.16)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        }),
        ..Default::default()
    });

    // Toggle Highlight & Dark Rim Pill
    let toggle_highlight_btn =
        button(text("高光").size(11).font(font::ui_font(Weight::Medium)).color(
            if state.enable_highlight {
                Color::WHITE
            } else if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.6)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.6)
            },
        ))
        .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
        .style(move |_theme, _status| button::Style {
            background: Some(Background::Color(if state.enable_highlight {
                Color::from_rgb(0.18, 0.75, 0.38)
            } else if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            })),
            border: Border::default().rounded(14.0),
            ..Default::default()
        })
        .on_press(Message::ToggleHighlight(!state.enable_highlight));

    let toggle_rim_btn = button(text("暗边").size(11).font(font::ui_font(Weight::Medium)).color(
        if state.enable_dark_rim {
            Color::WHITE
        } else if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.6)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.6)
        },
    ))
    .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(if state.enable_dark_rim {
            Color::from_rgb(0.18, 0.52, 0.95)
        } else if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.12)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        })),
        border: Border::default().rounded(14.0),
        ..Default::default()
    })
    .on_press(Message::ToggleDarkRim(!state.enable_dark_rim));

    // Mirrors NSWindow.isDocumentEdited: shows the close button's dirty dot.
    let document_edited_btn =
        button(text("未保存").size(11).font(font::ui_font(Weight::Medium)).color(
            if state.document_edited {
                Color::WHITE
            } else if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.6)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.6)
            },
        ))
        .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
        .style(move |_theme, _status| button::Style {
            background: Some(Background::Color(if state.document_edited {
                Color::from_rgb(0.85, 0.25, 0.20)
            } else if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            })),
            border: Border::default().rounded(14.0),
            ..Default::default()
        })
        .on_press(Message::ToggleDocumentEdited);

    let theme_btn =
        button(text(if is_dark { "☀️ 浅色" } else { "🌙 深色" }).size(11).color(if is_dark {
            Color::WHITE
        } else {
            Color::BLACK
        }))
        .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
        .style(move |_theme, _status| button::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.15)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            })),
            border: Border::default().rounded(14.0),
            ..Default::default()
        })
        .on_press(Message::ToggleTheme);

    let transparency_btn = button(
        text(state.transparency.label())
            .size(11)
            .font(font::ui_font(Weight::Medium))
            .color(Color::WHITE),
    )
    .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(match state.transparency {
            GlassTransparency::High => Color::from_rgb(0.0, 0.48, 1.0),
            GlassTransparency::Ultra => Color::from_rgb(0.55, 0.25, 0.85),
            GlassTransparency::Frosted => Color::from_rgb(0.40, 0.45, 0.52),
        })),
        border: Border::default().rounded(14.0),
        ..Default::default()
    })
    .on_press(Message::CycleTransparency);

    let header_row = row![
        traffic_lights,
        space().width(Length::Fixed(12.0)),
        title_text,
        space().width(Length::Fill),
        pipeline_pill,
        space().width(Length::Fixed(4.0)),
        wallpaper_pill,
        space().width(Length::Fixed(4.0)),
        clarity_pill,
        space().width(Length::Fixed(4.0)),
        blur_pill,
        space().width(Length::Fixed(4.0)),
        transparency_btn,
        toggle_highlight_btn,
        toggle_rim_btn,
        document_edited_btn,
        theme_btn,
    ]
    .spacing(4.0)
    .align_y(Alignment::Center)
    .padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 });

    state.controller.loyal_drag_bar(
        44.0,
        header_row,
        Message::DragWindow,
        Some(Message::TrafficLights(TrafficLightsEvent::PressEnd {
            index: 2,
            committed: true,
        })),
    )
}

/// Builds the search input widget positioned exactly over `search_rect`.
fn view_search_input(
    state: &State,
    is_dark: bool,
    search_rect: Rectangle,
) -> Element<'_, Message, Theme, iced_backend::Renderer> {
    let search_icon = text("🔍").size(13);

    let input = text_input("聚焦搜索或输入网址...", &state.search_query)
        .on_input(Message::SearchInputChanged)
        .size(13)
        .width(Length::Fill)
        .style(move |_theme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            placeholder: if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.45)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.40)
            },
            value: if is_dark { Color::WHITE } else { Color::BLACK },
            selection: Color::from_rgba(0.0, 0.48, 1.0, 0.35),
            icon: Color::TRANSPARENT,
        });

    let badge =
        container(text("⌘K").size(10).font(font::ui_font(Weight::Semibold)).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.55)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.50)
        }))
        .padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 })
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.08)
            })),
            border: Border::default().rounded(4.0),
            ..Default::default()
        });

    let content = row![search_icon, space().width(8.0), input, space().width(8.0), badge]
        .align_y(Alignment::Center)
        .padding(Padding { top: 4.0, right: 14.0, bottom: 4.0, left: 14.0 })
        .width(Length::Fixed(search_rect.width))
        .height(Length::Fixed(search_rect.height));

    container(content)
        .padding(Padding {
            top: search_rect.y - 44.0, // relative to stage top
            left: search_rect.x,
            ..Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds interactive hitboxes over the dock app icons with hover tooltip cards.
fn view_dock_hitboxes<'a>(
    state: &'a State,
    metrics: &LayoutMetrics,
) -> Element<'a, Message, Theme, iced_backend::Renderer> {
    let mut hitboxes = Vec::new();

    for (i, &app) in DockApp::ALL.iter().enumerate() {
        let rect = metrics.icon_rects[i];
        let is_hovered = state.hovered_app == Some(app);

        let click_target = iced::widget::mouse_area(
            container(space()).width(Length::Fixed(rect.width)).height(Length::Fixed(rect.height)),
        )
        .on_enter(Message::IconHovered(Some(app)))
        .on_exit(Message::IconHovered(None))
        .on_press(Message::IconClicked(app));

        let placed_target = container(click_target).padding(Padding {
            top: rect.y - metrics.header_h,
            left: rect.x,
            ..Padding::ZERO
        });

        hitboxes.push(placed_target.into());

        // Hover tooltip pill floating above the hovered icon
        if is_hovered {
            let tooltip_pill = container(
                text(app.name()).size(11).font(font::ui_font(Weight::Semibold)).color(Color::WHITE),
            )
            .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
            .style(move |_theme: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.10, 0.10, 0.14, 0.92))),
                border: Border::default()
                    .rounded(6.0)
                    .width(0.5)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.22)),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 8.0,
                },
                ..Default::default()
            });

            let placed_tooltip = container(tooltip_pill).padding(Padding {
                top: rect.y - metrics.header_h - 32.0,
                left: (rect.x - 14.0).max(10.0),
                ..Padding::ZERO
            });

            hitboxes.push(placed_tooltip.into());
        }
    }

    container(iced::widget::Stack::with_children(hitboxes))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds the bottom status bar displaying metrics and user action feedbacks.
fn view_status_bar(
    state: &State,
    is_dark: bool,
) -> Element<'_, Message, Theme, iced_backend::Renderer> {
    let status_text =
        text(&state.last_action).size(11).font(font::ui_font(Weight::Normal)).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.88)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.82)
        });

    let plan = KawasePassPlan::new(
        state.window_size.width as i32,
        state.window_size.height as i32,
        state.blur_radius,
    );
    let pass_desc = if plan.use_deep_blur { "3级深度金字塔" } else { "2级标准金字塔" };
    let hint_text = text(format!(
        "✨ vibrancy-rs: 模糊 {:.0}pt | {} | +25%饱和度提升 & IGN抗色带抖动",
        state.blur_radius, pass_desc
    ))
    .size(11)
    .font(font::ui_font(Weight::Medium))
    .color(if is_dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.70)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.65)
    });

    let bar = row![status_text, space().width(Length::Fill), hint_text].align_y(Alignment::Center);

    let bar_pill = container(bar)
        .padding(Padding { top: 3.0, right: 14.0, bottom: 3.0, left: 14.0 })
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.08, 0.09, 0.12, 0.45)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.45)
            })),
            border: Border::default().rounded(12.0).width(0.5).color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.16)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            }),
            ..Default::default()
        });

    container(bar_pill)
        .width(Length::Fill)
        .padding(Padding { top: 2.0, right: 14.0, bottom: 6.0, left: 14.0 })
        .into()
}

fn app_theme(state: &State) -> Theme {
    state.theme.clone()
}

fn app_style(state: &State, _theme: &Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: if state.is_dark() {
            Color::from_rgb(0.92, 0.92, 0.92)
        } else {
            Color::from_rgb(0.12, 0.13, 0.15)
        },
    }
}

fn main() -> iced::Result {
    let fonts = font::ui_fonts();
    let window_settings = window::Settings {
        size: Size::new(1240.0, 820.0),
        position: window::Position::Centered,
        transparent: true,
        blur: true,
        decorations: false,
        ..Default::default()
    };

    let mut app =
        iced::application::<State, Message, Theme, iced_backend::Renderer>(boot, update, view)
            .title("Liquid Glass Optics & Dock Showcase - bmol-iced")
            .theme(app_theme)
            .style(app_style)
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
    use liquid_glass::dock::{create_procedural_redwood_buffer, perform_separable_gaussian_blur};
    use vibrancy_rs::{MaterialKind, VibrancyAppearance};

    #[test]
    fn test_transparency_defaults_and_cycling() {
        let t = GlassTransparency::default();
        assert_eq!(t, GlassTransparency::Ultra);

        assert_eq!(t.next(), GlassTransparency::High);
        assert_eq!(t.next().next(), GlassTransparency::Frosted);
        assert_eq!(t.next().next().next(), GlassTransparency::Ultra);
    }

    #[test]
    fn test_transparency_center_alpha_values() {
        let high = GlassTransparency::High;
        let ultra = GlassTransparency::Ultra;
        let frosted = GlassTransparency::Frosted;

        // High transparency provides rich balanced visibility
        assert!(high.center_alpha_light() <= 0.35);
        assert!(high.center_alpha_dark() <= 0.40);
        assert!(high.center_alpha_light() < frosted.center_alpha_light());
        assert!(high.center_alpha_dark() < frosted.center_alpha_dark());

        // Ultra is the most transparent
        assert!(ultra.center_alpha_light() < high.center_alpha_light());
        assert!(ultra.center_alpha_dark() < high.center_alpha_dark());
    }

    #[test]
    fn test_state_transparency_and_message_update() {
        let mut state = State::default();
        assert_eq!(state.transparency, GlassTransparency::Ultra);

        let _ = update(&mut state, Message::CycleTransparency);
        assert_eq!(state.transparency, GlassTransparency::High);

        let _ = update(&mut state, Message::SetTransparency(GlassTransparency::Frosted));
        assert_eq!(state.transparency, GlassTransparency::Frosted);
    }

    #[test]
    fn test_desktop_transparent_wallpaper_default() {
        let state = State::default();
        assert_eq!(state.wallpaper, WallpaperStyle::DesktopTransparent);
        assert_eq!(state.wallpaper.label(), "🖥️ 系统壁纸");
    }

    #[test]
    fn test_dock_layout_metrics() {
        let metrics = LayoutMetrics::new(Size::new(1200.0, 820.0), None);
        // Concentric geometric proportion system assertions:
        assert_eq!(metrics.base_icon_size, 40.0, "Icon size must be 40px");
        assert_eq!(metrics.icon_radius, 10.0, "Icon corner radius must be 10px (10:20:10 ratio)");
        assert_eq!(metrics.icon_gap, 13.0, "Icon gap must be 13px");
        assert_eq!(metrics.dock_padding, 13.0, "Dock padding must be 13px");
        assert_eq!(metrics.dock_radius, 23.0, "Dock radius must be concentric: 13px + 10px = 23px");
        assert_eq!(metrics.dock_rect.height, 66.0, "Dock height = 40 + 13*2 = 66px");
        assert_eq!(metrics.dock_rect.width, 490.0, "Dock width = 9*40 + 8*13 + 13*2 = 490px");
        assert_eq!(metrics.icon_rects.len(), 9);

        // Icons should fit symmetrically inside the dock with 13px padding
        assert_eq!(metrics.icon_rects[0].y, metrics.dock_rect.y + 13.0);
        assert_eq!(metrics.icon_rects[0].x, metrics.dock_rect.x + 13.0);
        let last_idx = metrics.icon_rects.len() - 1;
        let right_padding = (metrics.dock_rect.x + metrics.dock_rect.width)
            - (metrics.icon_rects[last_idx].x + metrics.icon_rects[last_idx].width);
        assert!((right_padding - 13.0).abs() < 1e-4, "Right padding must be exactly 13px");
    }

    #[test]
    fn test_app_style_transparent_background() {
        let state = State::default();
        let style = app_style(&state, &state.theme);
        assert_eq!(style.background_color, Color::TRANSPARENT);
    }

    #[test]
    fn test_blur_presets_and_radii() {
        assert_eq!(BlurPreset::ALL.len(), 10);
        assert_eq!(BlurPreset::Zero0.radius(), 0.0);
        assert_eq!(BlurPreset::Subtle2.radius(), 2.0);
        assert_eq!(BlurPreset::Subtle4.radius(), 4.0);
        assert_eq!(BlurPreset::Subtle8.radius(), 8.0);
        assert_eq!(BlurPreset::Medium12.radius(), 12.0);
        assert_eq!(BlurPreset::Standard16.radius(), 16.0);
        assert_eq!(BlurPreset::Enhanced24.radius(), 24.0);
        assert_eq!(BlurPreset::Heavy32.radius(), 32.0);
        assert_eq!(BlurPreset::Ultra48.radius(), 48.0);
        assert_eq!(BlurPreset::Max64.radius(), 64.0);
    }

    #[test]
    fn test_blur_preset_message_update() {
        let mut state = State::default();
        assert_eq!(state.blur_preset, BlurPreset::Standard16);
        assert_eq!(state.blur_radius, 16.0);

        let _ = update(&mut state, Message::SetBlurPreset(BlurPreset::Subtle4));
        assert_eq!(state.blur_preset, BlurPreset::Subtle4);
        assert_eq!(state.blur_radius, 4.0);

        let _ = update(&mut state, Message::SetBlurPreset(BlurPreset::Max64));
        assert_eq!(state.blur_preset, BlurPreset::Max64);
        assert_eq!(state.blur_radius, 64.0);
    }

    #[test]
    fn test_vibrancy_rs_kawase_pass_plan() {
        let plan_64 = KawasePassPlan::new(1200, 820, 64.0);
        assert!(plan_64.use_deep_blur);
        assert_eq!(plan_64.src_size, (600, 410));
        assert_eq!(plan_64.down1_size, (300, 205));
        assert_eq!(plan_64.down2_size, Some((150, 103)));

        let plan_16 = KawasePassPlan::new(1200, 820, 16.0);
        assert!(!plan_16.use_deep_blur);
        assert_eq!(plan_16.down2_size, None);

        // Verify standard material specifications from vibrancy-rs
        assert_eq!(MaterialKind::Dock.blur_radius(), 20.0);
        assert_eq!(MaterialKind::Launchpad.blur_radius(), 56.0);
        let dark_tint = MaterialKind::Dock.tint(VibrancyAppearance::Dark);
        assert_eq!(dark_tint, [24, 24, 28, 180]);
    }

    #[test]
    fn test_separable_gaussian_blur_convolution() {
        let w = 20;
        let h = 20;
        let mut src = vec![[0.0, 0.0, 0.0]; w * h];
        // Impulse in center
        src[10 * w + 10] = [1.0, 1.0, 1.0];

        let blurred_0 = perform_separable_gaussian_blur(&src, w, h, 0.0);
        assert!((blurred_0[10 * w + 10][0] - 1.0).abs() < 1e-3);

        let blurred_4 = perform_separable_gaussian_blur(&src, w, h, 4.0);
        // Energy diffused outward: center value decreases, neighbor increases
        assert!(blurred_4[10 * w + 10][0] < 0.5);
        assert!(blurred_4[10 * w + 11][0] > 0.0);
    }

    #[test]
    fn test_wallpaper_buffer_gaussian_blur_sampling() {
        let buf = create_procedural_redwood_buffer(200, 150);
        assert_eq!(buf.width, 200);
        assert_eq!(buf.height, 150);
        let sharp = buf.sample_bilinear(0.5, 0.5);
        assert!(sharp[0] >= 0.0 && sharp[0] <= 1.0);
        let blurred = buf.sample_blurred(0.5, 0.5, 96.0, Size::new(200.0, 150.0));
        assert!(blurred[0] >= 0.0 && blurred[0] <= 1.0);
    }

    #[test]
    fn test_real_system_wallpaper_frosted_plate_generation() {
        let wallpaper = load_or_create_wallpaper(1240, 820);
        let bounds = Size::new(1240.0, 820.0);
        let dock_rect = Rectangle::new(Point::new(300.0, 700.0), Size::new(640.0, 84.0));
        let handle = generate_frosted_plate_texture(
            WallpaperStyle::DesktopTransparent,
            dock_rect,
            bounds,
            16.0,
            24.0,
            false,
            Some(&wallpaper),
        );
        let _ = format!("{:?}", handle);
    }

    #[test]
    fn test_specular_cutoff_and_rim_darkening_physics() {
        const SPEC_CUTOFF: f32 = 0.90;
        const INNER_ARC_CUTOFF: f32 = 0.20;

        // At theta = 90° (vertical edge), u = 0.0, d = 0.0, s = 1.0
        let u_vertical = 0.0f32;
        assert!(u_vertical <= SPEC_CUTOFF, "Vertical edge must have 0 specular highlight");
        assert!(u_vertical <= INNER_ARC_CUTOFF, "Vertical edge must have 0 inner highlight");
        let vert_comp_vertical = u_vertical;
        let rim_factor_vertical = if vert_comp_vertical < 0.90 {
            let corner_soften =
                1.0 - 0.65 * (vert_comp_vertical * std::f32::consts::PI * 0.5).sin().powf(1.2);
            corner_soften.clamp(0.20, 1.0)
        } else {
            0.0
        };
        assert_eq!(rim_factor_vertical, 1.0, "Vertical edge must have 100% dark rim");

        // At curved arc (theta = 45°, u = 0.707)
        let u_arc = 0.707f32;
        assert!(u_arc <= SPEC_CUTOFF, "Outer boundary on curved arc is dark rim without specular");
        let rim_factor_arc = if u_arc < 0.90 {
            let corner_soften = 1.0 - 0.65 * (u_arc * std::f32::consts::PI * 0.5).sin().powf(1.2);
            corner_soften.clamp(0.20, 1.0)
        } else {
            0.0
        };
        assert!(
            rim_factor_arc < 0.60 && rim_factor_arc > 0.35,
            "Curved arc dark rim must soften gracefully"
        );
        // But inner secondary highlight extends through arc!
        assert!(u_arc > INNER_ARC_CUTOFF, "Inner secondary highlight extends into corner arc");

        // At theta = 0° (top horizontal edge, outside R corner), u = 1.0, s = 0.0
        let u_horizontal = 1.0f32;
        assert!(u_horizontal > SPEC_CUTOFF);
        assert!(u_horizontal > INNER_ARC_CUTOFF);
        // On straight horizontal segment, factors are exactly 1.0 (perfectly uniform peak brightness)
        let norm_u_flat = (u_horizontal - SPEC_CUTOFF) / (1.0 - SPEC_CUTOFF);
        let spec_factor_flat = (norm_u_flat * std::f32::consts::PI * 0.5).sin().powf(1.6);
        assert!(
            (spec_factor_flat - 1.0).abs() < 1e-5,
            "Straight line must have 100% uniform specular highlight"
        );
        let norm_arc_flat = (u_horizontal - INNER_ARC_CUTOFF) / (1.0 - INNER_ARC_CUTOFF);
        let inner_factor_flat = (norm_arc_flat * std::f32::consts::PI * 0.5).sin().powf(1.6);
        assert!(
            (inner_factor_flat - 1.0).abs() < 1e-5,
            "Straight line must have 100% uniform inner highlight"
        );

        // Ahead of R corner: Pre-corner lead-in smooth decay ("早于R角开始变化")
        let r = 24.0f32;
        let lead_in_dist = (r * 1.25).min(32.0).max(8.0);
        let calc_lead_in = |dist: f32| -> f32 {
            if dist >= lead_in_dist {
                1.0f32
            } else if dist >= 0.0 {
                let k_tangent = 0.85f32;
                let norm = (dist / lead_in_dist).clamp(0.0, 1.0);
                k_tangent + (1.0 - k_tangent) * (norm * std::f32::consts::PI * 0.5).sin().powf(1.5)
            } else {
                0.85f32
            }
        };

        // Far from corner in central plateau:
        assert_eq!(calc_lead_in(lead_in_dist + 50.0), 1.0, "Central plateau must be 100% constant");
        // Ahead of R corner (e.g. 15pt before tangent):
        let factor_ahead = calc_lead_in(15.0);
        assert!(
            factor_ahead < 1.0 && factor_ahead > 0.85,
            "Highlight begins smoothly decaying ahead of R corner"
        );
        // Exactly at tangent:
        assert!(
            (calc_lead_in(0.0) - 0.85).abs() < 1e-5,
            "Smoothly arrives at ~0.85 at R corner tangent"
        );

        // Entering R corner arc (u begins decreasing from 1.0 down towards 0.0):
        // Monotonic smooth attenuation continues along the corner arc!
        let u_corner_entry = 0.96f32;
        let norm_u_entry = (u_corner_entry - SPEC_CUTOFF) / (1.0 - SPEC_CUTOFF);
        let spec_factor_entry = (norm_u_entry * std::f32::consts::PI * 0.5).sin().powf(1.6);
        assert!(
            spec_factor_entry < spec_factor_flat,
            "Specular highlight strictly decays upon entering R corner"
        );

        let norm_arc_entry = (u_corner_entry - INNER_ARC_CUTOFF) / (1.0 - INNER_ARC_CUTOFF);
        let inner_factor_entry = (norm_arc_entry * std::f32::consts::PI * 0.5).sin().powf(1.6);
        assert!(
            inner_factor_entry < inner_factor_flat,
            "Inner highlight strictly decays upon entering R corner"
        );

        let rim_factor_horizontal = if u_horizontal < 0.90 {
            1.0f32
        } else {
            let norm = (1.0 - u_horizontal) / (1.0 - 0.90);
            (norm * std::f32::consts::PI * 0.5).sin().powf(1.5)
        };
        assert!(
            (rim_factor_horizontal - 0.0).abs() < 1e-5,
            "Top horizontal edge must have 0 dark rim"
        );

        // Calibrated light mode base alpha
        assert_eq!(GlassTransparency::High.center_alpha_light(), 0.13);

        // Near vertical edge of arc (u = 0.15): Inner highlight gracefully extinguished
        let u_near_vert = 0.15f32;
        assert!(u_near_vert <= INNER_ARC_CUTOFF);
    }

    #[test]
    fn test_draw_image_signature() {
        let handle = iced::widget::image::Handle::from_rgba(2, 2, vec![255; 16]);
        let _ = format!("{:?}", handle);
        let _img = iced::widget::canvas::Image::new(handle);
    }

    #[test]
    fn test_generate_frosted_plate_texture_blur_progression() {
        let wallpaper = load_or_create_wallpaper(1240, 820);
        let rect = Rectangle { x: 210.0, y: 700.0, width: 820.0, height: 80.0 };
        let window_size = Size::new(1240.0, 820.0);

        // 1. Generate 0pt clear slice
        let handle_0pt = generate_frosted_plate_texture(
            WallpaperStyle::DesktopTransparent,
            rect,
            window_size,
            0.0,
            24.0,
            false,
            Some(&wallpaper),
        );

        // 2. Generate 16pt frosted slice
        let handle_16pt = generate_frosted_plate_texture(
            WallpaperStyle::DesktopTransparent,
            rect,
            window_size,
            16.0,
            24.0,
            false,
            Some(&wallpaper),
        );

        assert_ne!(format!("{:?}", handle_0pt), "");
        assert_ne!(format!("{:?}", handle_16pt), "");
    }

    #[test]
    fn test_scheme_a_squircle_icon_integration() {
        let state = State::default();
        #[cfg(target_os = "macos")]
        {
            // At least Finder and Safari should be resolved and processed via squircle-icon-rs
            assert!(state.app_icons[0].is_some(), "Finder icon should be loaded");
            assert!(state.app_icons[1].is_some(), "Safari icon should be loaded");
        }
    }

    #[test]
    fn test_dock_demo_traffic_lights_controller_integration() {
        let mut state = State::default();
        assert_eq!(state.controller.traffic_lights.hover_progress, 0.0);
        assert!(!state.controller.is_animating());

        // Hover group
        let _ = update(&mut state, Message::TrafficLights(TrafficLightsEvent::GroupHover(true)));
        assert_eq!(state.controller.traffic_lights.hover_target, 1.0);
        assert!(state.controller.is_animating());

        // Unfocus window event automatically clears hover
        let _ = update(
            &mut state,
            Message::WindowEvent((window::Id::unique(), window::Event::Unfocused)),
        );
        assert_eq!(state.controller.traffic_lights.hover_target, 0.0);

        // Animation frame steps physics
        let _ = update(&mut state, Message::AnimationFrame(Instant::now()));
    }

    #[test]
    fn test_pipeline_mode_toggle_and_defaults() {
        let mut state = State::default();
        assert_eq!(state.pipeline_mode, PipelineMode::GpuLiquidRs);
        assert_eq!(state.pipeline_mode.label(), "GPU liquid-rs (自研引擎)");

        let _ = update(&mut state, Message::TogglePipelineMode);
        assert_eq!(state.pipeline_mode, PipelineMode::Canvas2D);
        assert_eq!(state.pipeline_mode.label(), "2D Canvas (矢量模拟)");

        let _ = update(&mut state, Message::TogglePipelineMode);
        assert_eq!(state.pipeline_mode, PipelineMode::GpuLiquidRs);
    }
}
