//! Application state, message dispatch, event lifecycle, and subscriptions.

use std::time::Instant;

use bmol_designs::popover_metrics::{PopoverArrowConfig, PopoverArrowEdge, PopoverArrowPreset};
use bmol_window_shell::{
    TrafficLightsEvent, WindowChromeConfig, WindowShellController, is_system_dark_mode,
    window_metrics,
};
use iced::{Point, Size, Subscription, Task, Theme, window};
use liquid_glass::{ContextMenu, ControlAction, UiColorScheme, UiPalette, UiTheme};

use crate::menu_content::{
    FloatingAppearance, MenuAction, MenuContentPreset, build_demo_menu, demo_metrics,
};
use crate::vibrancy::{BlurPreset, WallpaperStyle};

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
    TrafficLights(TrafficLightsEvent),
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

#[derive(Debug)]
pub struct State {
    pub controller: WindowShellController,
    pub theme: Theme,
    pub palette: UiPalette,
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
        let scheme = if is_dark { UiColorScheme::Dark } else { UiColorScheme::Light };
        let theme = UiTheme::new(scheme).iced_theme();
        let palette = UiTheme::new(scheme).palette();

        Self {
            controller,
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
        if self.controller.is_dark { UiColorScheme::Dark } else { UiColorScheme::Light }
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
        Message::WindowControl(action) => state.controller.handle_control_action(action),
        Message::TrafficLights(event) => state.controller.handle_traffic_lights(event),
        Message::AnimationFrame(now) => {
            state.controller.step(now);
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
            let offset = Point::new(state.cursor_pos.x - origin.x, state.cursor_pos.y - origin.y);
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
                    pos.x,
                    pos.y
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
            state.arrow_offset = if state.menu_preset == MenuContentPreset::DockReference1To1
                && edge == PopoverArrowEdge::Bottom
            {
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
                    state.last_action =
                        Some("已切换为: 🍎 1:1 原生 macOS Dock 菜单模式 (偏左圆润触角)".into());
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
                    state.last_action = Some(
                        "已切换触角预设: 🎯 原生 Dock 气泡细触角 (20×7 pt, 细腰紧凑温润)".into(),
                    );
                    PopoverArrowPreset::TooltipNarrow
                }
                PopoverArrowPreset::TooltipNarrow => {
                    state.last_action = Some(
                        "已切换触角预设: 📐 系统原生 AppKit NSPopover (27.5×13 pt, 标准挑角)"
                            .into(),
                    );
                    PopoverArrowPreset::AppKitStandard
                }
                PopoverArrowPreset::AppKitStandard => {
                    state.last_action =
                        Some("已切换触角预设: 🔹 微型提示指针 (12×5 pt, 超紧凑)".into());
                    PopoverArrowPreset::SubtleCompact
                }
                PopoverArrowPreset::SubtleCompact => {
                    state.last_action = Some(
                        "已切换触角预设: 🍎 原生 macOS 菜单触角 (21×9 pt, 柔和高阶喇叭口)".into(),
                    );
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
            iced::Event::Window(w_event) => Some(Message::WindowEvent((id, w_event))),
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right)) => {
                Some(Message::RightClicked)
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                Some(Message::EndDragCard)
            }
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
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

#[must_use]
pub fn app_theme(state: &State) -> Theme {
    state.theme.clone()
}
