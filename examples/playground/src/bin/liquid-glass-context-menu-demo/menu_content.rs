//! Context menu action definitions, presets, layout metrics, and builder.

use bmol_designs::menu_metrics;
use bmol_designs::popover_metrics::DOCK_MENU_ARROW_CENTER_OFFSET;
use liquid_glass::{ContextMenu, ControlAction, MenuItem, UiColorScheme, UiIcon};

use crate::state::{Message, State};

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
            Self::DockReference1To1 => {
                (demo_metrics::DOCK_REF_WIDTH, demo_metrics::DOCK_REF_HEIGHT)
            }
        }
    }

    #[must_use]
    pub const fn corner_radius(self) -> f32 {
        match self {
            Self::FullShowcase => demo_metrics::MENU_CORNER_RADIUS,
            Self::DockReference1To1 => demo_metrics::DOCK_REF_CORNER_RADIUS,
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

    /// 1:1 macOS Dock reference arrow left anchor offset: 27.0 pt / 154.0 pt ≈ 0.1753.
    /// (Places arrow apex at native macOS Dock menu default 27.0 pt from left card boundary)
    pub const DOCK_REF_ARROW_OFFSET: f32 = super::DOCK_MENU_ARROW_CENTER_OFFSET / DOCK_REF_WIDTH;

    /// Corner radius of the 1:1 macOS Dock reference menu (strictly 10.0 pt continuous squircle, matching native macOS).
    pub const DOCK_REF_CORNER_RADIUS: f32 = menu_metrics::CONTAINER_CORNER_RADIUS_CLASSIC;
}

/// Builds the comprehensive Apple-style Context Menu.
#[must_use]
pub fn build_demo_menu(state: &State) -> ContextMenu<Message> {
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
            .item(MenuItem::action("隐藏").on_press(Message::TriggerAction(MenuAction::Cut)))
            .item(MenuItem::action("退出").on_press(Message::WindowControl(ControlAction::Close)));
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
