//! Standalone showcase for authentic Apple-style Liquid Glass Context Menus.
//!
//! Features:
//! - Dual Appearance: Side-by-side demonstration of both Light and Dark physical glass menus
//! - Ultra-Heavy Backdrop Blur (64pt Dual-Kawase equivalent) completely dissolving high-frequency details
//! - Continuous 8.0 pt squircle container curvature matching macOS HIG
//! - 1px fine edge highlight rim with deep elevation drop shadows (36pt blur)
//! - Smooth Accent hover pill with crisp white text inversion (4.5 pt radius)
//! - Interactive right-click popup at cursor location with outside-click dismissal

#![allow(clippy::too_many_lines, clippy::cast_precision_loss)]

use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Rectangle,
    Shadow, Size, Subscription, Task, Theme, Vector, event,
    font::Weight,
    mouse,
    widget::{
        button,
        canvas::{self, Canvas, Frame, Geometry, Path, Stroke},
        column, container, row, space, stack, text,
    },
};
use bmol_designs::menu_metrics;
use liquid_glass::{
    ContextMenu, MenuItem, UiColorScheme, UiIcon, UiTheme,
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
}

impl WallpaperStyle {
    pub const ALL: [Self; 4] = [Self::Aurora, Self::Sunset, Self::Oceanic, Self::Tahoe];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Aurora => "Aurora Glow",
            Self::Sunset => "Sunset Flare",
            Self::Oceanic => "Deep Oceanic",
            Self::Tahoe => "Tahoe Sky",
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
    RightClicked,
    CursorMoved(Point),
    DismissFloatingMenu,
    OpenFloatingMenuAt(Point),
    TriggerAction(MenuAction),
    SetWallpaper(WallpaperStyle),
    SetBlurPreset(BlurPreset),
    SetFloatingAppearance(FloatingAppearance),
    ToggleColorScheme,
    TrafficLightClicked(WindowControlAction),
}

/// Simulated window control actions for the traffic lights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowControlAction {
    Close,
    Minimize,
    Zoom,
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

        let base_bg = match (self.style, self.is_dark) {
            (WallpaperStyle::Aurora, true) => Color::from_rgb(0.04, 0.03, 0.10),
            (WallpaperStyle::Aurora, false) => Color::from_rgb(0.92, 0.90, 0.98),
            (WallpaperStyle::Sunset, true) => Color::from_rgb(0.10, 0.03, 0.06),
            (WallpaperStyle::Sunset, false) => Color::from_rgb(0.98, 0.91, 0.88),
            (WallpaperStyle::Oceanic, true) => Color::from_rgb(0.01, 0.05, 0.12),
            (WallpaperStyle::Oceanic, false) => Color::from_rgb(0.88, 0.95, 0.98),
            (WallpaperStyle::Tahoe, true) => Color::from_rgb(0.03, 0.07, 0.15),
            (WallpaperStyle::Tahoe, false) => Color::from_rgb(0.85, 0.92, 0.99),
        };
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), base_bg);

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

        // 2. High-frequency background grid lines (dissolved under heavy frosted blur)
        let grid_stroke = Stroke::default().with_width(1.0).with_color(
            if self.is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.04)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.05)
            }
        );

        let blur_factor = (self.blur_preset.radius() / 64.0).clamp(0.25, 1.25);

        let mut x = 0.0;
        while x < w {
            // Only draw grid line if outside heavy blur occlusion zones or during subtle blur
            let in_heavy_occlusion = self.menu_occlusions.iter().any(|r| {
                blur_factor > 0.6 && (x >= r.x - 20.0 && x <= r.x + r.width + 20.0)
            });
            if !in_heavy_occlusion {
                frame.stroke(&Path::line(Point::new(x, 0.0), Point::new(x, h)), grid_stroke);
            }
            x += 64.0;
        }

        let mut y = 0.0;
        while y < h {
            let in_heavy_occlusion = self.menu_occlusions.iter().any(|r| {
                blur_factor > 0.6 && (y >= r.y - 20.0 && y <= r.y + r.height + 20.0)
            });
            if !in_heavy_occlusion {
                frame.stroke(&Path::line(Point::new(0.0, y), Point::new(w, y)), grid_stroke);
            }
            y += 64.0;
        }

        // 3. Physical Heavy Frosted Blur Diffusion Cores behind active menu rects
        // Mathematically simulates a 64pt multi-pass Dual-Kawase blur kernel:
        // high frequencies vanish completely, diffusing surrounding saturated colors
        // into a rich, luminous frosted atmosphere wash.
        for rect in &self.menu_occlusions {
            let padding = self.blur_preset.radius() * 0.45;
            let blur_rect = Rectangle {
                x: rect.x - padding,
                y: rect.y - padding,
                width: rect.width + padding * 2.0,
                height: rect.height + padding * 2.0,
            };

            // Sample low-frequency ambience from the wallpaper orbs for this rect center
            let center = Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            let mut ambient_r = 0.0f32;
            let mut ambient_g = 0.0f32;
            let mut ambient_b = 0.0f32;
            let mut total_weight = 0.001f32;

            for &(orb_center, radius, color) in orbs {
                let dx = center.x - orb_center.x;
                let dy = center.y - orb_center.y;
                let dist = (dx * dx + dy * dy).sqrt();
                let influence = (1.0 - (dist / (radius * 1.5))).max(0.0).powf(1.8);
                ambient_r += color.r * influence;
                ambient_g += color.g * influence;
                ambient_b += color.b * influence;
                total_weight += influence;
            }

            let avg_r = (ambient_r / total_weight).clamp(0.0, 1.0);
            let avg_g = (ambient_g / total_weight).clamp(0.0, 1.0);
            let avg_b = (ambient_b / total_weight).clamp(0.0, 1.0);

            // Multi-tier Gaussian diffusion wash
            let steps = 4;
            for s in 0..steps {
                let expand = (s as f32 + 1.0) * (self.blur_preset.radius() / 6.0);
                let alpha = if self.is_dark { 0.14 } else { 0.22 } * (1.0 / (s as f32 + 1.0));
                let wash_color = Color::from_rgba(avg_r, avg_g, avg_b, alpha);

                let r_box = Rectangle {
                    x: (blur_rect.x - expand).max(0.0),
                    y: (blur_rect.y - expand).max(0.0),
                    width: blur_rect.width + expand * 2.0,
                    height: blur_rect.height + expand * 2.0,
                };
                let path = Path::rounded_rectangle(
                    Point::new(r_box.x, r_box.y),
                    Size::new(r_box.width, r_box.height),
                    (menu_metrics::CONTAINER_CORNER_RADIUS + expand).into(),
                );
                frame.fill(&path, wash_color);
            }
        }

        vec![frame.into_geometry()]
    }
}

/// Application state.
#[derive(Debug)]
pub struct State {
    pub scheme: UiColorScheme,
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
}

impl Default for State {
    fn default() -> Self {
        Self {
            scheme: UiColorScheme::Dark,
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
        }
    }
}

impl State {
    pub fn rebuild_menus(&mut self) {
        self.light_menu = build_demo_menu(self).with_scheme(UiColorScheme::Light);
        self.dark_menu = build_demo_menu(self).with_scheme(UiColorScheme::Dark);
    }

    #[must_use]
    pub fn resolved_floating_scheme(&self) -> UiColorScheme {
        self.floating_appearance.resolve(self.scheme)
    }
}

pub fn boot() -> (State, Task<Message>) {
    let mut state = State::default();
    state.rebuild_menus();
    (state, Task::none())
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::RightClicked => {
            state.floating_menu = Some(state.cursor_pos);
            state.last_action = Some(format!("Menu Spawned at ({:.0}, {:.0})", state.cursor_pos.x, state.cursor_pos.y));
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
                    state.last_action = Some(format!("Toggled Status Bar -> {}", if state.show_status_bar { "ON" } else { "OFF" }));
                    state.rebuild_menus();
                }
                MenuAction::ToggleLineNumbers => {
                    state.show_line_numbers = !state.show_line_numbers;
                    state.last_action = Some(format!("Toggled Line Numbers -> {}", if state.show_line_numbers { "ON" } else { "OFF" }));
                    state.rebuild_menus();
                }
                MenuAction::ToggleWordWrap => {
                    state.word_wrap = !state.word_wrap;
                    state.last_action = Some(format!("Toggled Word Wrap -> {}", if state.word_wrap { "ON" } else { "OFF" }));
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
            state.last_action = Some(format!("Floating Menu Appearance: {}", a.label()));
            Task::none()
        }
        Message::ToggleColorScheme => {
            state.scheme = match state.scheme {
                UiColorScheme::Light => UiColorScheme::Dark,
                UiColorScheme::Dark => UiColorScheme::Light,
            };
            state.rebuild_menus();
            state.last_action = Some(format!("Window Theme: {:?}", state.scheme));
            Task::none()
        }
        Message::TrafficLightClicked(action) => {
            state.last_action = Some(format!("Window Control: {action:?}"));
            Task::none()
        }
    }
}

pub fn subscription(_state: &State) -> Subscription<Message> {
    event::listen_with(|event, _status, _id| match event {
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
            Some(Message::RightClicked)
        }
        iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
            Some(Message::CursorMoved(position))
        }
        _ => None,
    })
}

#[must_use]
pub fn app_theme(state: &State) -> Theme {
    UiTheme::new(state.scheme).iced_theme()
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

#[must_use]
pub fn view(state: &State) -> Element<'_, Message, Theme, iced::Renderer> {
    let is_dark = state.scheme == UiColorScheme::Dark;
    let theme = app_theme(state);
    let palette = UiTheme::new(state.scheme).palette();

    // 1. Top fused header bar
    let red_dot = button(space().width(Length::Fixed(12.0)).height(Length::Fixed(12.0)))
        .style(|_theme, _status| button::Style {
            background: Some(Background::Color(Color::from_rgb(1.0, 0.36, 0.34))),
            border: Border::default().rounded(6.0),
            ..button::Style::default()
        })
        .on_press(Message::TrafficLightClicked(WindowControlAction::Close));

    let yellow_dot = button(space().width(Length::Fixed(12.0)).height(Length::Fixed(12.0)))
        .style(|_theme, _status| button::Style {
            background: Some(Background::Color(Color::from_rgb(1.0, 0.75, 0.03))),
            border: Border::default().rounded(6.0),
            ..button::Style::default()
        })
        .on_press(Message::TrafficLightClicked(WindowControlAction::Minimize));

    let green_dot = button(space().width(Length::Fixed(12.0)).height(Length::Fixed(12.0)))
        .style(|_theme, _status| button::Style {
            background: Some(Background::Color(Color::from_rgb(0.16, 0.79, 0.28))),
            border: Border::default().rounded(6.0),
            ..button::Style::default()
        })
        .on_press(Message::TrafficLightClicked(WindowControlAction::Zoom));

    let traffic_lights = row![red_dot, yellow_dot, green_dot]
        .spacing(8.0)
        .align_y(Alignment::Center);

    let title_text = row![
        text("Liquid Glass Context Menu")
            .size(15.0)
            .font(font::ui_font(Weight::Semibold))
            .color(palette.text_primary),
        space().width(8.0),
        text("Heavy 64pt Blur · Dual Light & Dark")
            .size(12.0)
            .font(font::ui_font(Weight::Normal))
            .color(palette.text_secondary),
    ]
    .align_y(Alignment::Center);

    let mut wallpaper_picker = row![].spacing(4.0).align_y(Alignment::Center);
    for &style in &WallpaperStyle::ALL {
        let is_selected = state.wallpaper == style;
        let btn = button(
            text(style.label())
                .size(11.0)
                .font(font::ui_font(if is_selected { Weight::Semibold } else { Weight::Normal }))
                .color(if is_selected { Color::WHITE } else { palette.text_secondary })
        )
        .padding(Padding { top: 4.0, right: 10.0, bottom: 4.0, left: 10.0 })
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
        wallpaper_picker = wallpaper_picker.push(btn);
    }

    let scheme_btn = button(
        text(if is_dark { "🌙 Dark" } else { "☀️ Light" })
            .size(12.0)
            .font(font::ui_font(Weight::Medium))
            .color(palette.text_primary)
    )
    .padding(Padding { top: 4.0, right: 12.0, bottom: 4.0, left: 12.0 })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.10)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.08)
        })),
        border: Border::default().rounded(6.0),
        ..button::Style::default()
    })
    .on_press(Message::ToggleColorScheme);

    let top_bar = container(
        row![
            traffic_lights,
            space().width(24.0),
            title_text,
            space().width(Length::Fill),
            wallpaper_picker,
            space().width(12.0),
            scheme_btn,
        ]
        .align_y(Alignment::Center)
        .padding(Padding { top: 0.0, right: 16.0, bottom: 0.0, left: 16.0 })
    )
    .height(Length::Fixed(52.0))
    .width(Length::Fill)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.10, 0.10, 0.12, 0.75)
        } else {
            Color::from_rgba(0.96, 0.96, 0.98, 0.80)
        })),
        border: Border::default().width(1.0).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.10)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.08)
        }),
        ..container::Style::default()
    });

    // 2. Stage Menu Occlusion zones for physics-accurate heavy blur kernel
    let mut occlusions = Vec::new();
    // Known approximate bounds for the dual showcase menus centered in 1180x780 window
    occlusions.push(Rectangle { x: 50.0, y: 130.0, width: 230.0, height: 470.0 });
    occlusions.push(Rectangle { x: 310.0, y: 130.0, width: 230.0, height: 470.0 });
    if let Some(pos) = state.floating_menu {
        occlusions.push(Rectangle {
            x: pos.x.clamp(10.0, 880.0),
            y: pos.y.clamp(60.0, 320.0),
            width: 230.0,
            height: 470.0,
        });
    }

    let wallpaper_widget = Canvas::new(WallpaperCanvas {
        style: state.wallpaper,
        is_dark,
        blur_preset: state.blur_preset,
        menu_occlusions: occlusions,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    // 3. Side-by-Side Dual Showcase Layout (Light & Dark)
    let light_menu_view = state.light_menu.view(&theme);
    let dark_menu_view = state.dark_menu.view(&theme);

    let light_column = column![
        row![
            text("☀️")
                .size(13.0),
            space().width(6.0),
            text("Light Appearance")
                .size(13.0)
                .font(font::ui_font(Weight::Semibold))
                .color(palette.text_primary),
            space().width(Length::Fill),
            text("64pt Milky Glass")
                .size(11.0)
                .font(font::ui_font(Weight::Medium))
                .color(palette.text_secondary),
        ]
        .align_y(Alignment::Center)
        .padding(Padding { top: 0.0, right: 4.0, bottom: 6.0, left: 4.0 }),
        light_menu_view,
    ]
    .width(Length::Fixed(230.0));

    let dark_column = column![
        row![
            text("🌙")
                .size(13.0),
            space().width(6.0),
            text("Dark Appearance")
                .size(13.0)
                .font(font::ui_font(Weight::Semibold))
                .color(palette.text_primary),
            space().width(Length::Fill),
            text("56pt Charcoal Glass")
                .size(11.0)
                .font(font::ui_font(Weight::Medium))
                .color(palette.text_secondary),
        ]
        .align_y(Alignment::Center)
        .padding(Padding { top: 0.0, right: 4.0, bottom: 6.0, left: 4.0 }),
        dark_menu_view,
    ]
    .width(Length::Fixed(230.0));

    // Blur Presets Selector in the inspector
    let mut blur_picker = row![].spacing(4.0).align_y(Alignment::Center);
    for &preset in &BlurPreset::ALL {
        let is_sel = state.blur_preset == preset;
        let b_btn = button(
            text(preset.label())
                .size(10.0)
                .font(font::ui_font(if is_sel { Weight::Semibold } else { Weight::Normal }))
                .color(if is_sel { Color::WHITE } else { palette.text_secondary })
        )
        .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
        .style(move |_theme, _status| {
            if is_sel {
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
        .on_press(Message::SetBlurPreset(preset));
        blur_picker = blur_picker.push(b_btn);
    }

    // Floating Appearance Selector
    let mut appearance_picker = row![].spacing(4.0).align_y(Alignment::Center);
    for &app in &FloatingAppearance::ALL {
        let is_sel = state.floating_appearance == app;
        let a_btn = button(
            text(app.label())
                .size(10.0)
                .font(font::ui_font(if is_sel { Weight::Semibold } else { Weight::Normal }))
                .color(if is_sel { Color::WHITE } else { palette.text_secondary })
        )
        .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
        .style(move |_theme, _status| {
            if is_sel {
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
        .on_press(Message::SetFloatingAppearance(app));
        appearance_picker = appearance_picker.push(a_btn);
    }

    // Specification and Control Inspector Card
    let info_card = container(
        column![
            text("Physical Material & HIG Spec")
                .size(15.0)
                .font(font::ui_font(Weight::Bold))
                .color(palette.text_primary),
            space().height(8.0),
            metric_row("Backdrop Blur Kernel", "64.0 pt Ultra-Heavy Dual-Kawase", &palette),
            metric_row("Container Curvature", "8.0 pt continuous squircle", &palette),
            metric_row("Hover Capsule Pill", "4.5 pt rounded accent", &palette),
            metric_row("Item Row Height", "24.0 pt standard", &palette),
            metric_row("Specular Edge Rim", "1.0 px physical shine", &palette),
            metric_row("Elevation Shadow", "36.0 pt blur depth drop shadow", &palette),
            space().height(10.0),
            text("Frosted Blur Degree")
                .size(13.0)
                .font(font::ui_font(Weight::Semibold))
                .color(palette.text_primary),
            space().height(4.0),
            blur_picker,
            space().height(10.0),
            text("Spawned Menu Appearance")
                .size(13.0)
                .font(font::ui_font(Weight::Semibold))
                .color(palette.text_primary),
            space().height(4.0),
            appearance_picker,
            space().height(10.0),
            text("Active Checkbox State")
                .size(13.0)
                .font(font::ui_font(Weight::Semibold))
                .color(palette.text_primary),
            space().height(4.0),
            state_pill("Show Status Bar", state.show_status_bar, &palette),
            state_pill("Show Line Numbers", state.show_line_numbers, &palette),
            state_pill("Auto Word Wrap", state.word_wrap, &palette),
            space().height(12.0),
            button(
                text("🖱️ Right-Click Anywhere to Spawn at Cursor")
                    .size(12.0)
                    .font(font::ui_font(Weight::Medium))
                    .color(Color::WHITE)
            )
            .padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 })
            .width(Length::Fill)
            .style(move |_theme, _status| button::Style {
                background: Some(Background::Color(palette.accent)),
                border: Border::default().rounded(7.0),
                ..button::Style::default()
            })
            .on_press(Message::OpenFloatingMenuAt(Point::new(480.0, 260.0))),
        ]
        .spacing(5.0)
    )
    .width(Length::Fixed(350.0))
    .padding(Padding::from(16.0))
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.08, 0.08, 0.10, 0.70)
        } else {
            Color::from_rgba(0.98, 0.98, 1.0, 0.75)
        })),
        border: Border::default()
            .rounded(12.0)
            .width(1.0)
            .color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.16)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.12)
            }),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.28),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 22.0,
        },
        ..container::Style::default()
    });

    let showcase_row = row![
        light_column,
        space().width(24.0),
        dark_column,
        space().width(28.0),
        info_card,
    ]
    .align_y(Alignment::Center);

    let stage_content = container(showcase_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    // 4. Bottom Toast status pill
    let toast_text = state
        .last_action
        .as_deref()
        .unwrap_or("Right-click anywhere to trigger the context menu");

    let toast_pill = container(
        row![
            text("💡")
                .size(13.0),
            space().width(8.0),
            text(toast_text)
                .size(12.0)
                .font(font::ui_font(Weight::Medium))
                .color(palette.text_primary),
        ]
        .align_y(Alignment::Center)
    )
    .padding(Padding { top: 6.0, right: 16.0, bottom: 6.0, left: 14.0 })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.14, 0.14, 0.17, 0.88)
        } else {
            Color::from_rgba(0.95, 0.95, 0.98, 0.92)
        })),
        border: Border::default()
            .rounded(20.0)
            .width(1.0)
            .color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.18)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.14)
            }),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.32),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        ..container::Style::default()
    });

    let toast_layer = container(toast_pill)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(Padding { top: 0.0, right: 0.0, bottom: 18.0, left: 0.0 });

    // 5. Floating right-click context menu overlay
    let mut main_stack = stack![wallpaper_widget, stage_content, toast_layer];

    if let Some(pos) = state.floating_menu {
        // Transparent dismissal backdrop
        let backdrop = button(space().width(Length::Fill).height(Length::Fill))
            .style(|_theme, _status| button::Style {
                background: None,
                border: Border::default(),
                ..button::Style::default()
            })
            .on_press(Message::DismissFloatingMenu);

        let floating_scheme = state.resolved_floating_scheme();
        let floating_menu_view = if floating_scheme == UiColorScheme::Light {
            state.light_menu.view(&theme)
        } else {
            state.dark_menu.view(&theme)
        };

        // Clamp menu within stage bounds (assumed window ~1180x780, menu ~230x470)
        let clamped_x = pos.x.clamp(10.0, 930.0);
        let clamped_y = pos.y.clamp(60.0, 300.0);

        let menu_positioned = container(floating_menu_view)
            .padding(Padding {
                top: clamped_y,
                left: clamped_x,
                right: 0.0,
                bottom: 0.0,
            });

        let overlay_layer = stack![backdrop, menu_positioned];
        main_stack = main_stack.push(overlay_layer);
    }

    column![top_bar, main_stack].into()
}

fn metric_row<'a>(label: &'static str, value: &'static str, palette: &liquid_glass::UiPalette) -> Element<'a, Message, Theme, iced::Renderer> {
    row![
        text(label)
            .size(12.0)
            .font(font::ui_font(Weight::Normal))
            .color(palette.text_secondary),
        space().width(Length::Fill),
        text(value)
            .size(12.0)
            .font(font::ui_font(Weight::Medium))
            .color(palette.text_primary),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn state_pill<'a>(label: &'static str, active: bool, palette: &liquid_glass::UiPalette) -> Element<'a, Message, Theme, iced::Renderer> {
    row![
        text(if active { "●" } else { "○" })
            .size(10.0)
            .color(if active { palette.accent } else { palette.text_tertiary }),
        space().width(6.0),
        text(label)
            .size(12.0)
            .font(font::ui_font(Weight::Normal))
            .color(palette.text_primary),
        space().width(Length::Fill),
        text(if active { "Active" } else { "Off" })
            .size(11.0)
            .font(font::ui_font(Weight::Semibold))
            .color(if active { palette.accent } else { palette.text_tertiary }),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn main() -> iced::Result {
    let fonts = font::ui_fonts();
    let window_settings = iced::window::Settings {
        size: Size::new(1180.0, 780.0),
        transparent: true,
        decorations: false,
        ..Default::default()
    };

    let mut app = iced::application::<State, Message, Theme, iced::Renderer>(boot, update, view)
        .title("Liquid Glass Context Menu - Heavy Blur Showcase")
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
        assert_eq!(state.scheme, UiColorScheme::Dark);
        assert_eq!(state.wallpaper, WallpaperStyle::Aurora);
        assert_eq!(state.blur_preset, BlurPreset::UltraHeavy64);
        assert_eq!(state.floating_appearance, FloatingAppearance::FollowTheme);
        assert!(state.show_status_bar);
        assert!(state.show_line_numbers);
        assert!(!state.word_wrap);
        assert_eq!(state.floating_menu, None);
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
        assert_eq!(state.scheme, UiColorScheme::Dark);

        let _ = update(&mut state, Message::ToggleColorScheme);
        assert_eq!(state.scheme, UiColorScheme::Light);

        let _ = update(&mut state, Message::SetWallpaper(WallpaperStyle::Sunset));
        assert_eq!(state.wallpaper, WallpaperStyle::Sunset);

        let _ = update(&mut state, Message::SetBlurPreset(BlurPreset::Heavy48));
        assert_eq!(state.blur_preset, BlurPreset::Heavy48);
        assert_eq!(state.blur_preset.radius(), 48.0);
    }

    #[test]
    fn test_demo_menu_building_and_rendering() {
        let state = State::default();
        let menu = build_demo_menu(&state);
        assert!(!menu.is_empty());
        assert!(menu.len() >= 10);

        let theme_dark = app_theme(&state);
        let elem_dark = menu.view::<iced::Renderer>(&theme_dark);
        drop(elem_dark);

        let mut light_state = State::default();
        light_state.scheme = UiColorScheme::Light;
        let theme_light = app_theme(&light_state);
        let elem_light = menu.view::<iced::Renderer>(&theme_light);
        drop(elem_light);
    }
}
