//! Standalone showcase for authentic Apple-style Liquid Glass Context Menus.
//!
//! Fully powered by `bmol-window-shell`:
//! - Complete frameless window shell integration with physical non-client rim
//! - Authentic macOS traffic lights with dynamic symmetric margins (`margin_left == margin_top`)
//! - Interactive 8-direction border resize handles and loyal titlebar drag bar
//! - Native macOS squircle corner clipping and Stage Manager guard
//! - Dual Appearance: Side-by-side demonstration of both Light and Dark physical glass menus
//! - Ultra-Heavy Backdrop Blur (64pt Dual-Kawase equivalent) completely dissolving high-frequency details
//! - Continuous 8.0 pt squircle container curvature matching macOS HIG
//! - 1px fine edge highlight rim with deep elevation drop shadows (36pt blur)
//! - Interactive right-click popup at cursor location with outside-click dismissal

#![allow(clippy::too_many_lines, clippy::cast_precision_loss)]

use std::time::Instant;

use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Rectangle,
    Shadow, Size, Subscription, Task, Theme, Vector,
    font::Weight,
    mouse,
    widget::{
        button,
        canvas::{self, Canvas, Frame, Geometry, Path, Stroke},
        column, container, row, space, stack, text,
    },
    window,
};
use bmol_designs::menu_metrics;
use bmol_window_shell::{
    WindowChromeConfig, WindowShellController, is_system_dark_mode, traffic_lights, window_metrics,
};
use liquid_glass::{
    ContextMenu, ControlAction, MenuItem, TrafficLightsState, UiColorScheme, UiIcon, UiTheme,
    ui::font,
};

/// Colorful gradient wallpaper styles to test glass translucency and edge glare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallpaperStyle {
    #[default]
    Aurora,
    Sunset,
    Oceanic,
    Tahoe,
    PureWhite,
    PureBlack,
}

impl WallpaperStyle {
    pub const ALL: [Self; 6] = [
        Self::Aurora,
        Self::Sunset,
        Self::Oceanic,
        Self::Tahoe,
        Self::PureWhite,
        Self::PureBlack,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Aurora => "Aurora Glow",
            Self::Sunset => "Sunset Flare",
            Self::Oceanic => "Deep Oceanic",
            Self::Tahoe => "Tahoe Sky",
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
}

/// Canvas program rendering vibrant wallpapers and physical heavy blur diffusion cores.
struct WallpaperCanvas {
    style: WallpaperStyle,
    is_dark: bool,
    blur_preset: BlurPreset,
    menu_occlusions: Vec<Rectangle>,
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

        let base_bg = match self.style {
            WallpaperStyle::PureWhite => Color::WHITE,
            WallpaperStyle::PureBlack => Color::BLACK,
            WallpaperStyle::Aurora => {
                if self.is_dark {
                    Color::from_rgb(0.04, 0.03, 0.10)
                } else {
                    Color::from_rgb(0.92, 0.90, 0.98)
                }
            }
            WallpaperStyle::Sunset => {
                if self.is_dark {
                    Color::from_rgb(0.10, 0.03, 0.06)
                } else {
                    Color::from_rgb(0.98, 0.91, 0.88)
                }
            }
            WallpaperStyle::Oceanic => {
                if self.is_dark {
                    Color::from_rgb(0.01, 0.05, 0.12)
                } else {
                    Color::from_rgb(0.88, 0.95, 0.98)
                }
            }
            WallpaperStyle::Tahoe => {
                if self.is_dark {
                    Color::from_rgb(0.03, 0.07, 0.15)
                } else {
                    Color::from_rgb(0.85, 0.92, 0.99)
                }
            }
        };
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), base_bg);

        // For calibration backgrounds, provide an immaculate flat canvas for exact digital colorimeter readings
        if self.style == WallpaperStyle::PureWhite || self.style == WallpaperStyle::PureBlack {
            return vec![frame.into_geometry()];
        }

        // 1. Draw large saturated glowing color orbs across the backdrop
        let (w, h) = (bounds.width, bounds.height);
        let orbs: &[(Point, f32, Color)] = match self.style {
            WallpaperStyle::Aurora => &[
                (Point::new(w * 0.22, h * 0.35), 360.0, Color::from_rgba(0.60, 0.12, 0.98, 0.65)),
                (Point::new(w * 0.68, h * 0.40), 400.0, Color::from_rgba(0.00, 0.88, 0.96, 0.58)),
                (Point::new(w * 0.46, h * 0.65), 380.0, Color::from_rgba(0.98, 0.18, 0.65, 0.60)),
                (Point::new(w * 0.85, h * 0.80), 320.0, Color::from_rgba(0.18, 0.45, 1.00, 0.55)),
            ],
            WallpaperStyle::Sunset => &[
                (Point::new(w * 0.20, h * 0.35), 380.0, Color::from_rgba(0.98, 0.42, 0.05, 0.68)),
                (Point::new(w * 0.65, h * 0.32), 360.0, Color::from_rgba(0.95, 0.10, 0.38, 0.62)),
                (Point::new(w * 0.48, h * 0.72), 400.0, Color::from_rgba(1.00, 0.76, 0.10, 0.58)),
                (Point::new(w * 0.82, h * 0.68), 320.0, Color::from_rgba(0.70, 0.10, 0.65, 0.50)),
            ],
            WallpaperStyle::Oceanic => &[
                (Point::new(w * 0.28, h * 0.35), 400.0, Color::from_rgba(0.05, 0.82, 0.85, 0.65)),
                (Point::new(w * 0.72, h * 0.45), 370.0, Color::from_rgba(0.10, 0.48, 0.98, 0.60)),
                (Point::new(w * 0.42, h * 0.80), 350.0, Color::from_rgba(0.04, 0.90, 0.55, 0.55)),
                (Point::new(w * 0.85, h * 0.25), 290.0, Color::from_rgba(0.38, 0.20, 0.92, 0.48)),
            ],
            WallpaperStyle::Tahoe => &[
                (Point::new(w * 0.25, h * 0.38), 390.0, Color::from_rgba(0.15, 0.58, 0.98, 0.62)),
                (Point::new(w * 0.70, h * 0.35), 350.0, Color::from_rgba(0.42, 0.82, 1.00, 0.58)),
                (Point::new(w * 0.52, h * 0.78), 380.0, Color::from_rgba(0.08, 0.78, 0.72, 0.54)),
                (Point::new(w * 0.15, h * 0.82), 300.0, Color::from_rgba(0.32, 0.42, 0.90, 0.50)),
            ],
            WallpaperStyle::PureWhite | WallpaperStyle::PureBlack => &[],
        };

        for &(center, radius, color) in orbs {
            let num_rings = 8;
            for i in (1..=num_rings).rev() {
                let r = radius * (i as f32 / num_rings as f32);
                let alpha_scale = (1.0 - (i as f32 / (num_rings as f32 + 1.0))).powf(1.5);
                let ring_color = Color {
                    a: color.a * alpha_scale * if self.is_dark { 0.85 } else { 0.65 },
                    ..color
                };
                let circle = Path::circle(center, r);
                frame.fill(&circle, ring_color);
            }
        }

        // 2. High-frequency backdrop grid (demonstrates how heavy blur eliminates sharp detail)
        let grid_spacing = 32.0;
        let grid_stroke = Stroke::default().with_color(if self.is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.04)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.04)
        }).with_width(1.0);

        let mut x = grid_spacing;
        while x < bounds.width {
            let mut skip_line = false;
            if self.blur_preset.radius() >= 48.0 {
                for rect in &self.menu_occlusions {
                    if x >= rect.x - 8.0 && x <= rect.x + rect.width + 8.0 {
                        skip_line = true;
                        break;
                    }
                }
            }
            if !skip_line {
                let path = Path::line(Point::new(x, 0.0), Point::new(x, bounds.height));
                frame.stroke(&path, grid_stroke);
            }
            x += grid_spacing;
        }

        let mut y = grid_spacing;
        while y < bounds.height {
            let mut skip_line = false;
            if self.blur_preset.radius() >= 48.0 {
                for rect in &self.menu_occlusions {
                    if y >= rect.y - 8.0 && y <= rect.y + rect.height + 8.0 {
                        skip_line = true;
                        break;
                    }
                }
            }
            if !skip_line {
                let path = Path::line(Point::new(0.0, y), Point::new(bounds.width, y));
                frame.stroke(&path, grid_stroke);
            }
            y += grid_spacing;
        }

        // 3. Physical heavy blur diffusion cores underneath context menus
        let blur_r = self.blur_preset.radius();
        for rect in &self.menu_occlusions {
            let center = Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            let passes = if blur_r >= 48.0 { 6 } else { 3 };
            for p in 0..passes {
                let expand = blur_r * (p as f32 / passes as f32);
                let alpha = (0.28 / (passes as f32)) * (1.0 - (p as f32 / passes as f32) * 0.4);
                let halo_rect = Rectangle {
                    x: (rect.x - expand).max(0.0),
                    y: (rect.y - expand).max(0.0),
                    width: rect.width + expand * 2.0,
                    height: rect.height + expand * 2.0,
                };
                let halo_path = Path::rounded_rectangle(
                    Point::new(halo_rect.x, halo_rect.y),
                    halo_rect.size(),
                    (menu_metrics::CONTAINER_CORNER_RADIUS + expand * 0.5).into(),
                );

                let halo_tint = if self.is_dark {
                    Color::from_rgba(0.08, 0.10, 0.16, alpha)
                } else {
                    Color::from_rgba(1.0, 1.0, 1.0, alpha * 1.5)
                };
                frame.fill(&halo_path, halo_tint);
            }

            // Radial environmental diffuse glow
            let env_radius = (rect.width * 0.5 + blur_r * 1.2).max(120.0);
            let glow_color = match self.style {
                WallpaperStyle::Aurora => Color::from_rgba(0.55, 0.20, 0.90, 0.12),
                WallpaperStyle::Sunset => Color::from_rgba(0.95, 0.35, 0.10, 0.12),
                WallpaperStyle::Oceanic => Color::from_rgba(0.10, 0.60, 0.90, 0.12),
                WallpaperStyle::Tahoe => Color::from_rgba(0.20, 0.70, 0.85, 0.12),
                WallpaperStyle::PureWhite | WallpaperStyle::PureBlack => Color::TRANSPARENT,
            };
            let circle = Path::circle(center, env_radius);
            frame.fill(&circle, glow_color);
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
            wallpaper: WallpaperStyle::Aurora,
            blur_preset: BlurPreset::UltraHeavy64,
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
        }
    }
}

impl State {
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
    ContextMenu::new()
        .width(menu_metrics::DEFAULT_WIDTH)
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
            .size(14.0)
            .font(font::ui_font(Weight::Semibold))
            .color(palette.text_primary),
        space().width(8.0),
        text("Heavy 64pt Blur · Dual Light & Dark")
            .size(11.5)
            .font(font::ui_font(Weight::Normal))
            .color(palette.text_secondary),
    ]
    .align_y(Alignment::Center);

    let wallpaper_picker = view_wallpaper_picker(state, palette, is_dark);
    let blur_picker = view_blur_preset_picker(state, palette, is_dark);

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

/// Visual styling configuration for side-by-side showcase cards.
#[derive(Debug, Clone, Copy)]
struct CardStyle {
    badge_title: &'static str,
    badge_sub: &'static str,
    bg: Color,
    border: Color,
    shadow: Color,
}

/// Builds a side-by-side menu comparison card.
fn view_menu_card<'a>(
    menu: &'a ContextMenu<Message>,
    style: CardStyle,
    theme: &'a Theme,
    palette: &'a liquid_glass::UiPalette,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let header_badge = container(
        column![
            text(style.badge_title)
                .size(13.0)
                .font(font::ui_font(Weight::Bold))
                .color(palette.text_primary),
            text(style.badge_sub)
                .size(10.5)
                .font(font::ui_font(Weight::Normal))
                .color(palette.text_secondary),
        ]
        .spacing(2.0)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .center_x(Length::Fill)
    .padding(Padding {
        top: 6.0,
        right: 12.0,
        bottom: 8.0,
        left: 12.0,
    });

    let menu_element = menu.view::<iced::Renderer>(theme);

    let card_box = container(column![header_badge, menu_element].spacing(8.0).align_x(Alignment::Center))
        .padding(Padding {
            top: 8.0,
            right: 8.0,
            bottom: 12.0,
            left: 8.0,
        })
        .style(move |_theme| container::Style {
            background: Some(Background::Color(style.bg)),
            border: Border::default()
                .rounded(14.0)
                .width(1.0)
                .color(style.border),
            shadow: Shadow {
                color: style.shadow,
                offset: Vector::new(0.0, 16.0),
                blur_radius: 36.0,
            },
            ..container::Style::default()
        });

    card_box.into()
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
        .unwrap_or("Ready. Right-click anywhere to summon Liquid Glass Context Menu.");

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
        text("CALIBRATED: Dark(White 86 / Black 33) | Light(White 255 / Black 185)")
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

    // 2. Stage with side-by-side Light & Dark menu cards
    let light_card = view_menu_card(
        &state.light_menu,
        CardStyle {
            badge_title: "☀️ Light Mode Menu",
            badge_sub: "Calibrated: White 255 | Black 185 (α=72.5%)",
            bg: Color::from_rgba(1.0, 1.0, 1.0, 0.28),
            border: Color::from_rgba(1.0, 1.0, 1.0, 0.50),
            shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.24),
        },
        theme,
        palette,
    );

    let dark_card = view_menu_card(
        &state.dark_menu,
        CardStyle {
            badge_title: "🌙 Dark Mode Menu",
            badge_sub: "Calibrated: White 86 | Black 33 (α=79.2%)",
            bg: Color::from_rgba(0.0, 0.0, 0.0, 0.40),
            border: Color::from_rgba(1.0, 1.0, 1.0, 0.18),
            shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
        },
        theme,
        palette,
    );

    let stage_menus_row = row![light_card, dark_card]
        .spacing(48.0)
        .align_y(Alignment::Center);

    let floating_opts_bar = row![
        text("Floating Right-Click Menu Appearance: ")
            .size(11.5)
            .font(font::ui_font(Weight::Medium))
            .color(palette.text_primary),
        row(FloatingAppearance::ALL.iter().map(|&app| {
            let selected = state.floating_appearance == app;
            button(
                text(app.label())
                    .size(11.0)
                    .font(font::ui_font(if selected {
                        Weight::Semibold
                    } else {
                        Weight::Normal
                    }))
                    .color(if selected {
                        Color::WHITE
                    } else {
                        palette.text_secondary
                    }),
            )
            .padding(Padding {
                top: 3.0,
                right: 8.0,
                bottom: 3.0,
                left: 8.0,
            })
            .style(move |_theme, _status| {
                if selected {
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
            .on_press(Message::SetFloatingAppearance(app))
            .into()
        }))
        .spacing(4.0),
        space().width(Length::Fill),
        text("💡 Right-click anywhere to pop up dynamic menu at cursor")
            .size(11.0)
            .font(font::ui_font(Weight::Normal))
            .color(palette.text_secondary),
    ]
    .align_y(Alignment::Center)
    .padding(Padding {
        top: 6.0,
        right: 14.0,
        bottom: 6.0,
        left: 14.0,
    });

    let floating_opts_container = container(floating_opts_bar)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.12, 0.12, 0.15, 0.70)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.65)
            })),
            border: Border::default().rounded(8.0).width(0.5).color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            }),
            ..container::Style::default()
        });

    let stage_content = column![
        floating_opts_container,
        stage_menus_row,
    ]
    .spacing(24.0)
    .align_x(Alignment::Center);

    let stage_centered = container(stage_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    // Approximate occlusion rectangles for heavy blur diffusion core
    let menu_occlusions = vec![
        Rectangle {
            x: 200.0,
            y: 140.0,
            width: 240.0,
            height: 480.0,
        },
        Rectangle {
            x: 520.0,
            y: 140.0,
            width: 240.0,
            height: 480.0,
        },
    ];

    let wallpaper_canvas = Canvas::new(WallpaperCanvas {
        style: state.wallpaper,
        is_dark,
        blur_preset: state.blur_preset,
        menu_occlusions,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    let wallpaper_layer = container(wallpaper_canvas)
        .width(Length::Fill)
        .height(Length::Fill);

    let stage_stack = stack![wallpaper_layer, stage_centered]
        .width(Length::Fill)
        .height(Length::Fill);

    let page_content = if state.show_status_bar {
        column![
            draggable_header,
            stage_stack,
            view_status_bar(state, palette, is_dark)
        ]
    } else {
        column![draggable_header, stage_stack]
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

        let positioned_menu = container(floating_menu_widget)
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
        assert_eq!(state.wallpaper, WallpaperStyle::Aurora);
        assert_eq!(state.blur_preset, BlurPreset::UltraHeavy64);
        assert_eq!(state.floating_appearance, FloatingAppearance::FollowTheme);
        assert!(state.show_status_bar);
        assert!(state.show_line_numbers);
        assert!(!state.word_wrap);
        assert_eq!(state.floating_menu, None);
        assert_eq!(state.controller.window_id, None);
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

        let _ = update(&mut state, Message::SetWallpaper(WallpaperStyle::Sunset));
        assert_eq!(state.wallpaper, WallpaperStyle::Sunset);

        let _ = update(&mut state, Message::SetBlurPreset(BlurPreset::Heavy48));
        assert_eq!(state.blur_preset, BlurPreset::Heavy48);
        assert_eq!(state.blur_preset.radius(), 48.0);
    }

    #[test]
    fn test_demo_menu_building_and_rendering() {
        let mut state = State::default();
        state.rebuild_menus();
        let menu = build_demo_menu(&state);
        assert!(!menu.is_empty());
        assert!(menu.len() >= 10);

        let theme_dark = app_theme(&state);
        let elem_dark = menu.view::<iced::Renderer>(&theme_dark);
        drop(elem_dark);

        let mut light_state = State::default();
        light_state.controller.set_dark_mode(false);
        light_state.rebuild_menus();
        let theme_light = app_theme(&light_state);
        let elem_light = menu.view::<iced::Renderer>(&theme_light);
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
}
