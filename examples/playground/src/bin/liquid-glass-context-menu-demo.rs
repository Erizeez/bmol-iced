//! Standalone showcase for authentic Apple-style Liquid Glass Context Menus.
//!
//! Demonstrates the translucent frosted glass context menu over vibrant, high-contrast
//! colorful wallpapers (Aurora, Sunset, Oceanic, Tahoe). Conforms strictly to macOS HIG
//! and `bmol-designs::menu_metrics`:
//! - Continuous 8.0 pt squircle container curvature
//! - 1px fine edge highlight rim with deep elevation drop shadows
//! - Smooth Accent hover pill with crisp white text inversion (4.5 pt radius)
//! - Full support for shortcuts, icons, checkable toggle states, separators, and destructive actions
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

/// Canvas program rendering vibrant, blurred gradient orbs on the background.
struct WallpaperCanvas {
    style: WallpaperStyle,
    is_dark: bool,
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
            (WallpaperStyle::Aurora, true) => Color::from_rgb(0.05, 0.04, 0.12),
            (WallpaperStyle::Aurora, false) => Color::from_rgb(0.92, 0.90, 0.98),
            (WallpaperStyle::Sunset, true) => Color::from_rgb(0.12, 0.04, 0.07),
            (WallpaperStyle::Sunset, false) => Color::from_rgb(0.98, 0.91, 0.88),
            (WallpaperStyle::Oceanic, true) => Color::from_rgb(0.02, 0.06, 0.14),
            (WallpaperStyle::Oceanic, false) => Color::from_rgb(0.88, 0.95, 0.98),
            (WallpaperStyle::Tahoe, true) => Color::from_rgb(0.04, 0.08, 0.16),
            (WallpaperStyle::Tahoe, false) => Color::from_rgb(0.85, 0.92, 0.99),
        };
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), base_bg);

        // Draw multiple large soft glowing color orbs
        let (w, h) = (bounds.width, bounds.height);
        let orbs: &[(Point, f32, Color)] = match self.style {
            WallpaperStyle::Aurora => &[
                (Point::new(w * 0.25, h * 0.3), 320.0, Color::from_rgba(0.55, 0.15, 0.95, 0.55)),
                (Point::new(w * 0.70, h * 0.4), 380.0, Color::from_rgba(0.0, 0.85, 0.95, 0.50)),
                (Point::new(w * 0.45, h * 0.7), 340.0, Color::from_rgba(0.98, 0.20, 0.65, 0.52)),
                (Point::new(w * 0.85, h * 0.8), 280.0, Color::from_rgba(0.20, 0.45, 1.00, 0.48)),
            ],
            WallpaperStyle::Sunset => &[
                (Point::new(w * 0.20, h * 0.35), 360.0, Color::from_rgba(0.98, 0.45, 0.05, 0.58)),
                (Point::new(w * 0.65, h * 0.30), 340.0, Color::from_rgba(0.92, 0.12, 0.35, 0.55)),
                (Point::new(w * 0.50, h * 0.75), 390.0, Color::from_rgba(1.00, 0.75, 0.10, 0.52)),
                (Point::new(w * 0.80, h * 0.70), 300.0, Color::from_rgba(0.65, 0.10, 0.60, 0.45)),
            ],
            WallpaperStyle::Oceanic => &[
                (Point::new(w * 0.30, h * 0.35), 380.0, Color::from_rgba(0.05, 0.78, 0.82, 0.55)),
                (Point::new(w * 0.75, h * 0.45), 350.0, Color::from_rgba(0.12, 0.45, 0.95, 0.52)),
                (Point::new(w * 0.40, h * 0.80), 320.0, Color::from_rgba(0.05, 0.88, 0.55, 0.48)),
                (Point::new(w * 0.85, h * 0.25), 260.0, Color::from_rgba(0.35, 0.20, 0.90, 0.40)),
            ],
            WallpaperStyle::Tahoe => &[
                (Point::new(w * 0.25, h * 0.40), 370.0, Color::from_rgba(0.15, 0.55, 0.95, 0.55)),
                (Point::new(w * 0.70, h * 0.35), 330.0, Color::from_rgba(0.40, 0.80, 1.00, 0.52)),
                (Point::new(w * 0.55, h * 0.80), 360.0, Color::from_rgba(0.10, 0.75, 0.70, 0.48)),
                (Point::new(w * 0.15, h * 0.85), 280.0, Color::from_rgba(0.30, 0.40, 0.85, 0.45)),
            ],
        };

        for &(center, radius, color) in orbs {
            let num_rings = 7;
            for i in (1..=num_rings).rev() {
                let r = radius * (i as f32 / num_rings as f32);
                let alpha_scale = (1.0 - (i as f32 / (num_rings as f32 + 1.0))).powf(1.6);
                let ring_color = Color {
                    a: color.a * alpha_scale * if self.is_dark { 0.75 } else { 0.55 },
                    ..color
                };
                let circle = Path::circle(center, r);
                frame.fill(&circle, ring_color);
            }
        }

        // Draw decorative thin grid lines on background
        let grid_stroke = Stroke::default().with_width(1.0).with_color(
            if self.is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.03)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.04)
            }
        );
        let mut x = 0.0;
        while x < w {
            frame.stroke(&Path::line(Point::new(x, 0.0), Point::new(x, h)), grid_stroke);
            x += 80.0;
        }
        let mut y = 0.0;
        while y < h {
            frame.stroke(&Path::line(Point::new(0.0, y), Point::new(w, y)), grid_stroke);
            y += 80.0;
        }

        vec![frame.into_geometry()]
    }
}

/// Application state.
#[derive(Debug)]
pub struct State {
    pub scheme: UiColorScheme,
    pub wallpaper: WallpaperStyle,
    pub show_status_bar: bool,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub cursor_pos: Point,
    pub floating_menu: Option<Point>,
    pub last_action: Option<String>,
    pub demo_menu: ContextMenu<Message>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scheme: UiColorScheme::Dark,
            wallpaper: WallpaperStyle::Aurora,
            show_status_bar: true,
            show_line_numbers: true,
            word_wrap: false,
            cursor_pos: Point::new(300.0, 250.0),
            floating_menu: None,
            last_action: None,
            demo_menu: ContextMenu::new(),
        }
    }
}

impl State {
    pub fn rebuild_menu(&mut self) {
        self.demo_menu = build_demo_menu(self);
    }
}

pub fn boot() -> (State, Task<Message>) {
    let mut state = State::default();
    state.rebuild_menu();
    (state, Task::none())
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::RightClicked => {
            state.floating_menu = Some(state.cursor_pos);
            state.last_action = Some(format!("Menu Opened at ({:.0}, {:.0})", state.cursor_pos.x, state.cursor_pos.y));
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
            state.last_action = Some(format!("Menu Opened at ({:.0}, {:.0})", pos.x, pos.y));
            Task::none()
        }
        Message::TriggerAction(action) => {
            match action {
                MenuAction::ToggleStatusBar => {
                    state.show_status_bar = !state.show_status_bar;
                    state.last_action = Some(format!("Toggled Status Bar -> {}", if state.show_status_bar { "ON" } else { "OFF" }));
                    state.rebuild_menu();
                }
                MenuAction::ToggleLineNumbers => {
                    state.show_line_numbers = !state.show_line_numbers;
                    state.last_action = Some(format!("Toggled Line Numbers -> {}", if state.show_line_numbers { "ON" } else { "OFF" }));
                    state.rebuild_menu();
                }
                MenuAction::ToggleWordWrap => {
                    state.word_wrap = !state.word_wrap;
                    state.last_action = Some(format!("Toggled Word Wrap -> {}", if state.word_wrap { "ON" } else { "OFF" }));
                    state.rebuild_menu();
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
        Message::ToggleColorScheme => {
            state.scheme = match state.scheme {
                UiColorScheme::Light => UiColorScheme::Dark,
                UiColorScheme::Dark => UiColorScheme::Light,
            };
            state.rebuild_menu();
            state.last_action = Some(format!("Appearance: {:?}", state.scheme));
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
        text("macOS HIG 8pt Squircle & 1px Rim")
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
            Color::from_rgba(0.10, 0.10, 0.12, 0.70)
        } else {
            Color::from_rgba(0.96, 0.96, 0.98, 0.75)
        })),
        border: Border::default().width(1.0).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.10)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.08)
        }),
        ..container::Style::default()
    });

    // 2. Stage Canvas Wallpaper
    let wallpaper_widget = Canvas::new(WallpaperCanvas {
        style: state.wallpaper,
        is_dark,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    // 3. Central Showcase Layout
    let showcase_menu_element = state.demo_menu.view(&theme);

    // Info cards beside the menu
    let info_card = container(
        column![
            text("macOS HIG Menu Specifications")
                .size(16.0)
                .font(font::ui_font(Weight::Bold))
                .color(palette.text_primary),
            space().height(10.0),
            metric_row("Container Curvature", "8.0 pt continuous squircle", &palette),
            metric_row("Hover Capsule Pill", "4.5 pt rounded accent", &palette),
            metric_row("Item Row Height", "24.0 pt standard", &palette),
            metric_row("Leading Icon Slot", "18.0 pt width (14pt glyph)", &palette),
            metric_row("Edge Highlighting", "1.0 px physical rim shine", &palette),
            metric_row("Elevation Shadow", "24-28pt blur depth shadow", &palette),
            space().height(14.0),
            text("Active Toggle Settings")
                .size(14.0)
                .font(font::ui_font(Weight::Semibold))
                .color(palette.text_primary),
            space().height(6.0),
            state_pill("Show Status Bar", state.show_status_bar, &palette),
            state_pill("Show Line Numbers", state.show_line_numbers, &palette),
            state_pill("Auto Word Wrap", state.word_wrap, &palette),
            space().height(14.0),
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
        .spacing(6.0)
    )
    .width(Length::Fixed(320.0))
    .padding(Padding::from(18.0))
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.10, 0.10, 0.12, 0.65)
        } else {
            Color::from_rgba(0.98, 0.98, 1.0, 0.70)
        })),
        border: Border::default()
            .rounded(12.0)
            .width(1.0)
            .color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.15)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            }),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 20.0,
        },
        ..container::Style::default()
    });

    let center_content = row![
        showcase_menu_element,
        space().width(32.0),
        info_card,
    ]
    .align_y(Alignment::Center);

    let stage_content = container(center_content)
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
            Color::from_rgba(0.15, 0.15, 0.18, 0.85)
        } else {
            Color::from_rgba(0.95, 0.95, 0.98, 0.90)
        })),
        border: Border::default()
            .rounded(20.0)
            .width(1.0)
            .color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.16)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.12)
            }),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.30),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    });

    let toast_layer = container(toast_pill)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(Padding { top: 0.0, right: 0.0, bottom: 20.0, left: 0.0 });

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

        let floating_menu_view = state.demo_menu.view(&theme);

        // Clamp menu within stage bounds (assumed window ~1120x760, menu ~240x440)
        let clamped_x = pos.x.clamp(10.0, 860.0);
        let clamped_y = pos.y.clamp(60.0, 310.0);

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
        size: Size::new(1120.0, 760.0),
        transparent: true,
        decorations: false,
        ..Default::default()
    };

    let mut app = iced::application::<State, Message, Theme, iced::Renderer>(boot, update, view)
        .title("Liquid Glass Context Menu Showcase")
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
