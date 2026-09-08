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
        canvas::{self, Canvas, Frame, Geometry},
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

    // Draggable menu cards
    StartDragCard(DragTarget),
    EndDragCard,
    ResetCardPositions,
}

/// Canvas program rendering television color test blocks (SMPTE bars, grid, pure calibration).
struct WallpaperCanvas {
    style: WallpaperStyle,
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

    // Draggable card positions
    pub light_pos: Point,
    pub dark_pos: Point,
    pub active_drag: Option<(DragTarget, Point)>,
    pub top_card: DragTarget,
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

/// Visual and metadata configuration for draggable showcase menu cards.
#[derive(Debug, Clone, Copy)]
struct MenuCardConfig {
    target: DragTarget,
    badge_title: &'static str,
    badge_sub: &'static str,
    is_dark_card: bool,
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
            .size(15.0)
            .font(font::ui_font(Weight::Bold))
            .color(palette.accent),
        column![
            text(config.badge_title)
                .size(12.0)
                .font(font::ui_font(Weight::Bold))
                .color(palette.text_primary),
            text(config.badge_sub)
                .size(10.0)
                .font(font::ui_font(Weight::Normal))
                .color(palette.text_secondary),
        ]
        .spacing(1.0),
        space().width(Length::Fill),
        text(if is_dragging { "松开固定" } else { "按住拖拽" })
            .size(10.0)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dragging { palette.accent } else { palette.text_tertiary }),
    ]
    .spacing(8.0)
    .align_y(Alignment::Center);

    let drag_header = iced::widget::mouse_area(
        container(grip_pill)
            .width(Length::Fill)
            .padding(Padding {
                top: 6.0,
                right: 10.0,
                bottom: 6.0,
                left: 10.0,
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

    let menu_element = menu.view::<iced::Renderer>(theme);

    let card_box = column![drag_header, menu_element]
        .spacing(6.0)
        .width(Length::Fixed(menu_metrics::DEFAULT_WIDTH));

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
        .unwrap_or("就绪。可按住顶部手柄拖拽浅色/深色菜单至各色彩条上方，通过数码测色计校准");

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
        text("CALIBRATED: 白底(深86/浅255) | 黑底(深33/浅185) | 黄底(深[86,86,33]/浅[255,255,185]) | 蓝底(深[33,33,86]/浅[185,185,255])")
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

    let light_card = view_menu_card(
        MenuCardConfig {
            target: DragTarget::LightCard,
            badge_title: "☀️ Light Mode Menu",
            badge_sub: "校准值: 白255 | 黑185 (α=72.5%)",
            is_dark_card: false,
        },
        &state.light_menu,
        is_light_dragging,
        theme,
        palette,
    );

    let dark_card = view_menu_card(
        MenuCardConfig {
            target: DragTarget::DarkCard,
            badge_title: "🌙 Dark Mode Menu",
            badge_sub: "校准值: 白86 | 黑33 (α=79.2%)",
            is_dark_card: true,
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

    let wallpaper_canvas = Canvas::new(WallpaperCanvas {
        style: state.wallpaper,
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
}
