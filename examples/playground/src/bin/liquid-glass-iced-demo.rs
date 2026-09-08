#[path = "../iced_backend.rs"]
mod iced_backend;

use iced::{
    Color, Element, Length, Subscription, Task, Theme,
    widget::{button, column, container, row, scrollable, space, stack, text},
};
use iced_backend::{
    FUSED_TOP_BAR_HEIGHT, Renderer, SIDEBAR_CONTENT_INSET, SIDEBAR_CONTENT_WIDTH,
    SIDEBAR_LIST_BOTTOM_INSET, SIDEBAR_SCROLLBAR_BOTTOM_INSET, SIDEBAR_SEARCH_HEIGHT,
    SIDEBAR_SEARCH_TOP, SIDEBAR_WIDTH, TOP_BAR_BUTTON_SIZE,
};
use bmol_window_shell::{window_metrics, window_rim};
use liquid_glass::{
    GlassAccessibility, GlassId, Rect, ScrollbarConfig, UiColorScheme, UiIcon, UiTheme,
    ui::{components, font, spring_scroll_view_with_config},
};

struct State {
    window_id: Option<iced::window::Id>,
    window_focused: bool,
    active_section: Section,
    appearance: Appearance,
    accent: Accent,
    search: String,
    auto_updates: bool,
    notifications: bool,
    reduce_motion: bool,
    reduce_transparency: bool,
    increased_contrast: bool,
    volume: f32,
    system_scheme: UiColorScheme,
    traffic_lights: liquid_glass::TrafficLightsState,
}

impl Default for State {
    fn default() -> Self {
        let initial_dark = bmol_window_shell::is_system_dark_mode();
        let initial_scheme = if initial_dark {
            UiColorScheme::Dark
        } else {
            UiColorScheme::Light
        };
        Self {
            window_id: None,
            window_focused: true,
            active_section: Section::General,
            appearance: Appearance::Automatic,
            accent: Accent::Blue,
            search: String::new(),
            auto_updates: true,
            notifications: true,
            reduce_motion: false,
            reduce_transparency: false,
            increased_contrast: false,
            volume: 64.0,
            system_scheme: initial_scheme,
            traffic_lights: liquid_glass::TrafficLightsState::new(),
        }
    }
}

impl State {
    fn color_scheme(&self) -> UiColorScheme {
        match self.appearance {
            Appearance::Automatic => self.system_scheme,
            Appearance::Light => UiColorScheme::Light,
            Appearance::Dark => UiColorScheme::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Section {
    #[default]
    AppleAccount,
    FamilySharing,
    Wifi,
    Bluetooth,
    Network,
    Vpn,
    Battery,
    General,
    Accessibility,
    MenuBar,
    Spotlight,
    Wallpaper,
    Appearance,
    Displays,
    Dock,
    Siri,
    Notifications,
    Sound,
    Focus,
    ScreenTime,
    LockScreen,
    Privacy,
    TouchId,
    UsersGroups,
    InternetAccounts,
    Wallet,
    GameCenter,
    ICloud,
    AirPods,
    Keyboard,
    Trackpad,
    GameController,
    PrintersScanners,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Appearance {
    #[default]
    Automatic,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Accent {
    #[default]
    Blue,
    Purple,
    Green,
    Red,
}

impl Accent {
    const ALL: [Self; 4] = [Self::Blue, Self::Purple, Self::Green, Self::Red];

    fn color(self) -> Color {
        match self {
            Self::Blue => Color::from_rgb(0.20, 0.48, 1.0),
            Self::Purple => Color::from_rgb(0.60, 0.35, 0.95),
            Self::Green => Color::from_rgb(0.12, 0.72, 0.54),
            Self::Red => Color::from_rgb(0.95, 0.38, 0.40),
        }
    }
}

impl std::fmt::Display for Accent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Green => "Green",
            Self::Red => "Red",
        })
    }
}

#[derive(Debug, Clone)]
enum Message {
    WindowReady(Option<iced::window::Id>),
    WindowEvent((iced::window::Id, iced::window::Event)),
    ResizeWindow(iced::window::Direction),
    DragWindow,
    ToggleMaximize,
    ControlPressed {
        #[allow(dead_code)]
        id: GlassId,
        action: liquid_glass::ControlAction,
        execute: bool,
    },
    ControlPressStarted { id: GlassId },
    ControlPressVisualCancelled { id: GlassId },
    ControlPressEnded { id: GlassId },
    ControlGroupHover(bool),
    AnimationTick,
    SectionSelected(Section),
    AppearanceSelected(Appearance),
    AccentSelected(Accent),
    SearchChanged(String),
    AutoUpdatesChanged(bool),
    NotificationsChanged(bool),
    ReduceMotionChanged(bool),
    ReduceTransparencyChanged(bool),
    IncreasedContrastChanged(bool),
    VolumeChanged(f32),
    SystemThemeChanged(iced::theme::Mode),
    PollSystemTheme,
}

type AppElement<'a> = Element<'a, Message, Theme, Renderer>;

fn boot() -> (State, Task<Message>) {
    let state = State::default();
    let is_dark = state.color_scheme() == UiColorScheme::Dark;
    let rim_insets: f32 = window_rim::rim_thickness(is_dark);
    let symmetric_margin = (FUSED_TOP_BAR_HEIGHT - liquid_glass::WINDOW_CONTROL_NATIVE_SIZE) * 0.5;
    let origin_x = rim_insets + symmetric_margin;
    let origin_y = rim_insets + symmetric_margin;
    iced_backend::set_window_control_origin(origin_x, origin_y);
    iced_backend::set_color_scheme(state.color_scheme());
    iced_backend::set_accessibility(state.accessibility());
    iced_backend::set_window_inactive(!state.window_focused);
    (
        state,
        Task::batch([
            iced::system::theme().map(Message::SystemThemeChanged),
            liquid_glass::IcedWindowController::latest().map(Message::WindowReady),
        ]),
    )
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    let mut updates_color_scheme =
        matches!(&message, Message::AppearanceSelected(_) | Message::SystemThemeChanged(_));
    let updates_accessibility = matches!(
        &message,
        Message::ReduceMotionChanged(_)
            | Message::ReduceTransparencyChanged(_)
            | Message::IncreasedContrastChanged(_)
    );

    let mut task = Task::none();
    match message {
        Message::WindowReady(Some(id)) => {
            state.window_id = Some(id);
            let is_dark = state.color_scheme() == UiColorScheme::Dark;
            let options = liquid_glass::NativeWindowOptions::new()
                .with_corner_radius(window_metrics::DEFAULT_CORNER_RADIUS as f64)
                .with_dark_mode(is_dark)
                .with_shadow(false)
                .with_edr(true)
                .with_stage_manager_guard(true);

            task = iced::window::run(id, move |w| {
                if let Ok(handle) = w.window_handle() {
                    let _ = liquid_glass::setup_native_window(handle.as_raw(), options);
                }
            })
            .discard();
        }
        Message::WindowReady(None) => {}
        Message::WindowEvent((id, event)) => {
            match event {
                iced::window::Event::Opened { .. } => {
                    if state.window_id.is_none() {
                        return update(state, Message::WindowReady(Some(id)));
                    }
                }
                iced::window::Event::Focused => {
                    if state.window_id == Some(id) || state.window_id.is_none() {
                        state.window_focused = true;
                        iced_backend::set_window_inactive(false);
                    }
                }
                iced::window::Event::Unfocused => {
                    if state.window_id == Some(id) {
                        state.window_focused = false;
                        state.traffic_lights.hover_target = 0.0;
                        state.traffic_lights.hover_progress = 0.0;
                        iced_backend::set_window_inactive(true);
                    }
                }
                iced::window::Event::Closed => {
                    if state.window_id == Some(id) {
                        state.window_id = None;
                    }
                }
                _ => {}
            }
        }
        Message::ResizeWindow(direction) => {
            if let Some(id) = state.window_id {
                task = iced::window::drag_resize(id, direction);
            }
        }
        Message::DragWindow => {
            if let Some(id) = state.window_id {
                task = iced::window::drag(id);
            }
        }
        Message::ToggleMaximize => {
            if let Some(id) = state.window_id {
                task = iced::window::toggle_maximize(id);
            }
        }
        Message::ControlPressed { action, execute, .. } => {
            if execute {
                if let Some(id) = state.window_id {
                    task = match action {
                        liquid_glass::ControlAction::Close => iced::window::close(id),
                        liquid_glass::ControlAction::Minimize => iced::window::minimize(id, true),
                        liquid_glass::ControlAction::Expand => iced::window::toggle_maximize(id),
                    };
                }
            }
        }
        Message::ControlPressStarted { id } => {
            if let Some(index) = liquid_glass::traffic_lights::slot_index(id) {
                state.traffic_lights.on_press_start(index);
                iced_backend::set_window_control_press_progress(id, 1.0);
            }
        }
        Message::ControlPressVisualCancelled { id } => {
            if let Some(index) = liquid_glass::traffic_lights::slot_index(id) {
                state.traffic_lights.on_press_cancel(index);
                iced_backend::set_window_control_press_progress(id, 0.0);
            }
        }
        Message::ControlPressEnded { id } => {
            if let Some(index) = liquid_glass::traffic_lights::slot_index(id) {
                state.traffic_lights.on_press_end(index);
                iced_backend::set_window_control_press_progress(id, 0.0);
            }
        }
        Message::ControlGroupHover(hovered) => {
            state.traffic_lights.on_group_hover(hovered);
            iced_backend::set_window_control_group_hover(0, hovered);
        }
        Message::AnimationTick => {
            state.traffic_lights.step(std::time::Instant::now());
            for &id in &liquid_glass::WINDOW_CONTROL_NATIVE_IDS {
                if let Some(index) = liquid_glass::traffic_lights::slot_index(id) {
                    iced_backend::set_window_control_scale(
                        id,
                        state.traffic_lights.press_springs[index].value(),
                    );
                }
            }
            iced_backend::set_window_control_group_progress(
                0,
                state.traffic_lights.hover_progress,
            );
        }
        Message::SectionSelected(section) => state.active_section = section,
        Message::AppearanceSelected(appearance) => state.appearance = appearance,
        Message::AccentSelected(accent) => state.accent = accent,
        Message::SearchChanged(search) => state.search = search,
        Message::AutoUpdatesChanged(enabled) => state.auto_updates = enabled,
        Message::NotificationsChanged(enabled) => state.notifications = enabled,
        Message::ReduceMotionChanged(enabled) => state.reduce_motion = enabled,
        Message::ReduceTransparencyChanged(enabled) => state.reduce_transparency = enabled,
        Message::IncreasedContrastChanged(enabled) => state.increased_contrast = enabled,
        Message::VolumeChanged(volume) => state.volume = volume,
        Message::SystemThemeChanged(mode) => {
            state.system_scheme = UiColorScheme::from_mode(mode);
        }
        Message::PollSystemTheme => {
            let current_dark = bmol_window_shell::is_system_dark_mode();
            let current_scheme = if current_dark {
                UiColorScheme::Dark
            } else {
                UiColorScheme::Light
            };
            if state.system_scheme != current_scheme {
                state.system_scheme = current_scheme;
                if state.appearance == Appearance::Automatic {
                    updates_color_scheme = true;
                }
            }
        }
    }
    if updates_color_scheme {
        let scheme = state.color_scheme();
        let is_dark = scheme == UiColorScheme::Dark;
        let rim_insets: f32 = window_rim::rim_thickness(is_dark);
        let symmetric_margin = (FUSED_TOP_BAR_HEIGHT - liquid_glass::WINDOW_CONTROL_NATIVE_SIZE) * 0.5;
        let origin_x = rim_insets + symmetric_margin;
        let origin_y = rim_insets + symmetric_margin;
        iced_backend::set_window_control_origin(origin_x, origin_y);
        iced_backend::set_color_scheme(scheme);
        if let Some(id) = state.window_id {
            let options = liquid_glass::NativeWindowOptions::new()
                .with_corner_radius(window_metrics::DEFAULT_CORNER_RADIUS as f64)
                .with_dark_mode(is_dark)
                .with_shadow(false)
                .with_edr(true)
                .with_stage_manager_guard(true);

            let native_task = iced::window::run(id, move |w| {
                if let Ok(handle) = w.window_handle() {
                    let _ = liquid_glass::setup_native_window(handle.as_raw(), options);
                }
            })
            .discard();
            task = Task::batch([task, native_task]);
        }
    }
    if updates_accessibility {
        iced_backend::set_accessibility(state.accessibility());
    }
    task
}

impl State {
    fn accessibility(&self) -> GlassAccessibility {
        GlassAccessibility {
            reduced_transparency: self.reduce_transparency,
            increased_contrast: self.increased_contrast,
            reduced_motion: self.reduce_motion,
        }
    }
}

fn subscription(state: &State) -> Subscription<Message> {
    let window_events = iced::window::events().map(Message::WindowEvent);
    let theme_changes = iced::system::theme_changes().map(Message::SystemThemeChanged);
    let poll_theme = iced::time::every(std::time::Duration::from_millis(250)).map(|_| Message::PollSystemTheme);
    if state.traffic_lights.is_animating() {
        Subscription::batch([
            window_events,
            theme_changes,
            poll_theme,
            iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::AnimationTick),
        ])
    } else {
        Subscription::batch([window_events, theme_changes, poll_theme])
    }
}

fn app_theme(state: &State) -> Theme {
    UiTheme::new(state.color_scheme()).iced_theme()
}

fn view(state: &State) -> AppElement<'_> {
    let sidebar_list = column![
        // Reserve the search field's vertical footprint inside the
        // scrollable content. The first row starts below the field at the
        // initial offset, but this space scrolls away with the list so
        // rows can still pass behind the floating search field.
        space().height(Length::Fixed(
            SIDEBAR_SEARCH_TOP + SIDEBAR_SEARCH_HEIGHT + SIDEBAR_FIRST_ROW_GAP,
        )),
        sidebar_group(&[Section::AppleAccount, Section::FamilySharing], state.active_section),
        sidebar_gap(),
        sidebar_group(
            &[Section::Wifi, Section::Bluetooth, Section::Network, Section::Vpn, Section::Battery,],
            state.active_section,
        ),
        sidebar_gap(),
        sidebar_group(
            &[
                Section::General,
                Section::Accessibility,
                Section::MenuBar,
                Section::Spotlight,
                Section::Wallpaper,
                Section::Appearance,
                Section::Displays,
                Section::Dock,
                Section::Siri,
            ],
            state.active_section,
        ),
        sidebar_gap(),
        sidebar_group(
            &[Section::Notifications, Section::Sound, Section::Focus, Section::ScreenTime,],
            state.active_section,
        ),
        sidebar_gap(),
        sidebar_group(
            &[Section::LockScreen, Section::Privacy, Section::TouchId, Section::UsersGroups,],
            state.active_section,
        ),
        sidebar_gap(),
        sidebar_group(
            &[Section::InternetAccounts, Section::Wallet, Section::GameCenter, Section::ICloud,],
            state.active_section,
        ),
        sidebar_gap(),
        sidebar_group(
            &[
                Section::AirPods,
                Section::Keyboard,
                Section::Trackpad,
                Section::GameController,
                Section::PrintersScanners,
            ],
            state.active_section,
        ),
    ]
    // The scroll viewport spans the sidebar. These are list-content insets,
    // not outer margins, so the independent scrollbar can stay at the edge.
    .padding(iced::Padding::new(0.0).left(SIDEBAR_CONTENT_INSET).right(SIDEBAR_CONTENT_INSET));

    // The scroll widget owns both the clipped list and the custom scrollbar.
    // Route the whole widget through the compositor overlay so its scrollbar
    // is composited after foreground list rows; the rows wrapped in
    // `glass_foreground` still keep their own foreground layer.
    let scroll_config = ScrollbarConfig::sidebar(
        sidebar_scrollbar_top(),
        SIDEBAR_SCROLLBAR_BOTTOM_INSET,
        SIDEBAR_SCROLLBAR_EDGE_INSET,
    );
    let sidebar_scroll = container(components::glass_overlay(spring_scroll_view_with_config(
        sidebar_list,
        state.color_scheme(),
        scroll_config,
    )))
    .width(Length::Fill)
    .height(Length::Fill);

    let search_overlay = container(components::search_field(
        GlassId(11),
        Rect::new(0.0, 0.0, SIDEBAR_CONTENT_WIDTH, SIDEBAR_SEARCH_HEIGHT),
        state.color_scheme(),
        &state.search,
        Message::SearchChanged,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(iced::Padding {
        top: SIDEBAR_SEARCH_TOP,
        right: SIDEBAR_CONTENT_INSET,
        bottom: 0.0,
        left: SIDEBAR_CONTENT_INSET,
    });

    let is_dark = state.color_scheme() == UiColorScheme::Dark;
    let rim_insets: f32 = window_rim::rim_thickness(is_dark);
    let symmetric_margin = (FUSED_TOP_BAR_HEIGHT - liquid_glass::WINDOW_CONTROL_NATIVE_SIZE) * 0.5;
    let slop = liquid_glass::traffic_lights::control_hover_slop(liquid_glass::WINDOW_CONTROL_NATIVE_SIZE);
    let leading_spacer_w = (symmetric_margin - slop).max(0.0);
    let origin_x = rim_insets + symmetric_margin;
    let origin_y = rim_insets + symmetric_margin;
    iced_backend::set_window_control_origin(origin_x, origin_y);

    let traffic_lights = row![
        column![].width(Length::Fixed(leading_spacer_w)),
        liquid_glass::control_group(
            liquid_glass::WINDOW_CONTROL_NATIVE_IDS,
            liquid_glass::WINDOW_CONTROL_NATIVE_SIZE,
            liquid_glass::WINDOW_CONTROL_GAP,
            state.color_scheme(),
            true,
            false,
            true,
            !state.window_focused,
            state.traffic_lights.hover_progress,
            state.traffic_lights.expand_behavior,
            [
                state.traffic_lights.press_springs[0].value(),
                state.traffic_lights.press_springs[1].value(),
                state.traffic_lights.press_springs[2].value(),
            ],
            |id, action| Message::ControlPressed { id, action, execute: true },
            |id| Message::ControlPressStarted { id },
            |id| Message::ControlPressVisualCancelled { id },
            |id| Message::ControlPressEnded { id },
            Message::ControlGroupHover,
        ),
        space::horizontal().width(Length::Fill),
    ]
    .align_y(iced::Alignment::Center)
    .height(Length::Fixed(FUSED_TOP_BAR_HEIGHT))
    .width(Length::Fill);

    let draggable_sidebar_top = container(
        liquid_glass::loyal_drag_bar(
            FUSED_TOP_BAR_HEIGHT,
            traffic_lights,
            Message::DragWindow,
            Some(Message::ToggleMaximize),
        )
    )
    .width(Length::Fixed(SIDEBAR_WIDTH))
    .height(Length::Fixed(FUSED_TOP_BAR_HEIGHT));

    let sidebar: AppElement<'_> = container(stack![sidebar_scroll, search_overlay, draggable_sidebar_top])
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: SIDEBAR_LIST_BOTTOM_INSET,
            left: 0.0,
        })
        .style(components::transparent_surface)
        .into();

    let toolbar_text_color = match state.color_scheme() {
        UiColorScheme::Light => Color::from_rgb8(76, 76, 76),
        UiColorScheme::Dark => Color::from_rgb8(232, 232, 232),
    };
    let toolbar = components::glass_surface(
        GlassId(10),
        Rect::new(0.0, 0.0, 1088.0, FUSED_TOP_BAR_HEIGHT),
        liquid_glass::GlassRole::Toolbar,
        state.color_scheme(),
        iced::Padding::new(8.0),
        None,
        row![
            compositor_navigation(state.color_scheme()),
            space().width(Length::Fill),
            button(components::icon_tinted(UiIcon::Question, 16.0, move |_| {
                toolbar_text_color
            }))
            .on_press(Message::SectionSelected(state.active_section))
            .width(Length::Fixed(TOP_BAR_BUTTON_SIZE))
            .height(Length::Fixed(TOP_BAR_BUTTON_SIZE))
            .padding(6.0)
            .style(components::toolbar_button_style),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    );
    // Keep the section title out of the material's widget tree entirely. It
    // belongs immediately after the navigation capsule and is emitted as an
    // independent final overlay, so no backdrop or toolbar blur pass can
    // sample a duplicate copy of the glyphs.
    let toolbar_title = components::glass_overlay(
        container(
            text(section_title(state.active_section))
                .size(font::size::TOOLBAR_TITLE)
                .font(font::toolbar_title_font())
                .style(move |_| text::Style { color: Some(toolbar_text_color) }),
        )
        .width(Length::Fill)
        .height(Length::Fixed(FUSED_TOP_BAR_HEIGHT))
        .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 88.0 })
        .align_y(iced::Alignment::Center),
    );

    let toolbar_content = stack![toolbar, toolbar_title];
    let draggable_toolbar = container(
        liquid_glass::loyal_drag_bar(
            FUSED_TOP_BAR_HEIGHT,
            toolbar_content,
            Message::DragWindow,
            Some(Message::ToggleMaximize),
        )
    )
    .width(Length::Fill)
    .height(Length::Fixed(FUSED_TOP_BAR_HEIGHT));

    let content_body = container(settings_page(state))
        .width(Length::Fill)
        .max_width(720.0)
        .padding(iced::Padding::new(22.0).top(78.0));
    let content_scroll = scrollable(content_body)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(components::scrollable_style);
    let content = container(content_scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([0, 34])
        .center_x(Length::Fill)
        .style(components::compositor_content_surface);

    let main =
        container(stack![content, draggable_toolbar]).width(Length::Fill).height(Length::Fill);

    let window_content = container(row![sidebar, main]).width(Length::Fill).height(Length::Fill);
    let is_dark = state.color_scheme() == UiColorScheme::Dark;
    let rimmed = liquid_glass::wrap_window_rim(
        window_content,
        liquid_glass::WindowRimConfig::new(is_dark)
            .with_corner_radius(window_metrics::DEFAULT_CORNER_RADIUS),
    );
    liquid_glass::wrap_border_resizer(rimmed, false, Message::ResizeWindow)
}

const SIDEBAR_SCROLLBAR_GAP: f32 = 8.0;
const SIDEBAR_FIRST_ROW_GAP: f32 = 12.0;
const SIDEBAR_SCROLLBAR_THUMB_WIDTH: f32 = 6.0;
const SIDEBAR_SCROLLBAR_ITEM_GAP: f32 = 1.0;
const SIDEBAR_SCROLLBAR_EDGE_INSET: f32 =
    SIDEBAR_CONTENT_INSET - SIDEBAR_SCROLLBAR_ITEM_GAP - SIDEBAR_SCROLLBAR_THUMB_WIDTH;

fn sidebar_scrollbar_top() -> f32 {
    SIDEBAR_SEARCH_TOP + SIDEBAR_SEARCH_HEIGHT + SIDEBAR_SCROLLBAR_GAP
}

fn sidebar_group(sections: &[Section], active: Section) -> AppElement<'static> {
    components::sidebar_group(
        sections
            .iter()
            .copied()
            .map(|section| sidebar_entry(section, active)),
    )
}

fn sidebar_gap() -> AppElement<'static> {
    components::sidebar_gap()
}

fn sidebar_entry(section: Section, active: Section) -> AppElement<'static> {
    let (icon, chip, label) = sidebar_metadata(section);
    components::sidebar_item(
        icon,
        chip,
        label,
        section == active,
        Message::SectionSelected(section),
    )
}

fn sidebar_metadata(section: Section) -> (UiIcon, Color, &'static str) {
    match section {
        Section::AppleAccount => {
            (UiIcon::SystemAppleAccount, Color::from_rgb(0.56, 0.56, 0.58), "Apple 账户")
        }
        Section::FamilySharing => {
            (UiIcon::SystemFamilySharing, Color::from_rgb(0.18, 0.48, 0.96), "家人共享")
        }
        Section::Wifi => (UiIcon::SystemWifi, Color::from_rgb(0.16, 0.48, 0.96), "Wi-Fi"),
        Section::Bluetooth => (UiIcon::SystemBluetooth, Color::from_rgb(0.14, 0.48, 0.96), "蓝牙"),
        Section::Network => (UiIcon::SystemNetwork, Color::from_rgb(0.18, 0.68, 0.34), "网络"),
        Section::Vpn => (UiIcon::SystemVpn, Color::from_rgb(0.20, 0.48, 0.94), "VPN"),
        Section::Battery => (UiIcon::SystemBattery, Color::from_rgb(0.18, 0.70, 0.34), "电池"),
        Section::General => (UiIcon::SystemGeneral, Color::from_rgb(0.56, 0.56, 0.58), "通用"),
        Section::Accessibility => {
            (UiIcon::SystemAccessibility, Color::from_rgb(0.18, 0.48, 0.96), "无障碍")
        }
        Section::MenuBar => (UiIcon::SystemMenuBar, Color::from_rgb(0.38, 0.40, 0.44), "菜单栏"),
        Section::Spotlight => (UiIcon::SystemSpotlight, Color::from_rgb(0.20, 0.48, 0.96), "聚焦"),
        Section::Wallpaper => (UiIcon::SystemWallpaper, Color::from_rgb(0.18, 0.50, 0.96), "墙纸"),
        Section::Appearance => {
            (UiIcon::SystemAppearance, Color::from_rgb(0.11, 0.11, 0.13), "外观")
        }
        Section::Displays => (UiIcon::SystemDisplays, Color::from_rgb(0.18, 0.62, 0.84), "显示器"),
        Section::Dock => (UiIcon::SystemDock, Color::from_rgb(0.20, 0.48, 0.96), "桌面与程序坞"),
        Section::Siri => (UiIcon::SystemSiri, Color::from_rgb(0.68, 0.35, 0.92), "Siri"),
        Section::Notifications => {
            (UiIcon::SystemNotifications, Color::from_rgb(1.0, 0.27, 0.23), "通知")
        }
        Section::Sound => (UiIcon::SystemSound, Color::from_rgb(0.96, 0.38, 0.22), "声效"),
        Section::Focus => (UiIcon::SystemFocus, Color::from_rgb(0.48, 0.32, 0.90), "专注模式"),
        Section::ScreenTime => {
            (UiIcon::SystemScreenTime, Color::from_rgb(0.48, 0.34, 0.90), "屏幕时间")
        }
        Section::LockScreen => {
            (UiIcon::SystemLockScreen, Color::from_rgb(0.42, 0.44, 0.48), "锁屏")
        }
        Section::Privacy => {
            (UiIcon::SystemPrivacySecurity, Color::from_rgb(0.04, 0.52, 1.0), "隐私与安全")
        }
        Section::TouchId => {
            (UiIcon::SystemTouchId, Color::from_rgb(0.18, 0.48, 0.96), "触控 ID 与密码")
        }
        Section::UsersGroups => {
            (UiIcon::SystemUsersGroups, Color::from_rgb(0.20, 0.48, 0.96), "用户与群组")
        }
        Section::InternetAccounts => {
            (UiIcon::SystemInternetAccounts, Color::from_rgb(0.18, 0.48, 0.96), "互联网账户")
        }
        Section::Wallet => {
            (UiIcon::SystemWallet, Color::from_rgb(0.12, 0.12, 0.14), "钱包与 Apple Pay")
        }
        Section::GameCenter => {
            (UiIcon::SystemGameCenter, Color::from_rgb(0.20, 0.48, 0.96), "Game Center")
        }
        Section::ICloud => (UiIcon::SystemICloud, Color::from_rgb(0.18, 0.56, 0.96), "iCloud"),
        Section::AirPods => (UiIcon::SystemAirPods, Color::from_rgb(0.52, 0.54, 0.58), "AirPods"),
        Section::Keyboard => (UiIcon::SystemKeyboard, Color::from_rgb(0.42, 0.44, 0.48), "键盘"),
        Section::Trackpad => (UiIcon::SystemTrackpad, Color::from_rgb(0.42, 0.44, 0.48), "触控板"),
        Section::GameController => {
            (UiIcon::SystemGameController, Color::from_rgb(0.42, 0.32, 0.82), "游戏控制器")
        }
        Section::PrintersScanners => {
            (UiIcon::SystemPrintersScanners, Color::from_rgb(0.42, 0.44, 0.48), "打印机与扫描仪")
        }
    }
}

fn section_title(section: Section) -> &'static str {
    sidebar_metadata(section).2
}

fn section_description(section: Section) -> &'static str {
    match section {
        Section::General => "你的 Mac，你的规则。调整日常使用体验。",
        Section::Appearance => "个性化系统的外观、颜色和动态效果。",
        Section::Notifications => "决定哪些 App 可以提醒你，以及提醒方式。",
        Section::Privacy => "管理数据访问权限，保护你的隐私与安全。",
        _ => "在这里查看和调整相关的系统设置。",
    }
}

#[allow(clippy::too_many_lines)]
fn settings_page(state: &State) -> AppElement<'_> {
    let description: AppElement<'_> =
        text(section_description(state.active_section)).size(font::size::BODY).into();
    match state.active_section {
        Section::General => column![
            description,
            components::section_heading("General", "The essentials for this Mac"),
            components::settings_group(vec![
                components::setting_link_with_icon(
                    UiIcon::Laptop,
                    "About This Mac",
                    Some("Apple M3 Pro · 18 GB memory".into()),
                    Message::SectionSelected(Section::General),
                ),
                components::setting_link_with_icon(
                    UiIcon::SoftwareUpdateBadge,
                    "Software Update",
                    Some("macOS is up to date".into()),
                    Message::SectionSelected(Section::General),
                ),
                components::setting_toggle(
                    "Automatic Updates",
                    Some("Install security responses and system files automatically".into()),
                    state.auto_updates,
                    Message::AutoUpdatesChanged,
                ),
            ]),
            components::section_heading(
                "Input & Output",
                "The small details that shape every interaction"
            ),
            components::settings_group(vec![
                components::setting_slider(
                    "Sound volume",
                    format!("{}%", state.volume.round()),
                    state.volume,
                    0.0..=100.0,
                    Message::VolumeChanged,
                ),
                components::setting_link_with_icon(
                    UiIcon::Keyboard,
                    "Keyboard",
                    Some("Key repeat and modifier keys".into()),
                    Message::SectionSelected(Section::General),
                ),
                components::setting_link_with_icon(
                    UiIcon::Mouse,
                    "Trackpad",
                    Some("Point & click, scroll & zoom".into()),
                    Message::SectionSelected(Section::General),
                ),
            ]),
            components::section_heading(
                "Connectivity",
                "Keep nearby devices and services within reach"
            ),
            components::settings_group(vec![
                components::setting_link(
                    "AirDrop",
                    Some("Contacts Only".into()),
                    Message::SectionSelected(Section::General)
                ),
                components::setting_link_with_icon(
                    UiIcon::Bluetooth,
                    "Bluetooth",
                    Some("On · 3 devices connected".into()),
                    Message::SectionSelected(Section::General),
                ),
                components::setting_link(
                    "Handoff",
                    Some("Allow handoff between this Mac and your devices".into()),
                    Message::SectionSelected(Section::General),
                ),
            ]),
        ]
        .spacing(14)
        .into(),
        Section::Appearance => column![
            description,
            components::section_heading("Appearance", "Personalize the way your system looks"),
            components::settings_group(vec![
                components::setting_row(
                    "Appearance",
                    Some("Choose how windows, controls, and menus look".into()),
                    appearance_picker(state.appearance),
                ),
                components::setting_row(
                    "Accent color",
                    Some("Used for buttons, links, and selected items".into()),
                    row![
                        components::accent_dot(state.accent.color()),
                        iced::widget::pick_list(
                            Accent::ALL.to_vec(),
                            Some(state.accent),
                            Message::AccentSelected,
                        )
                        .style(components::pick_list_style)
                        .menu_style(components::menu_style),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
                ),
                components::setting_pick_list(
                    "Sidebar icon size",
                    Some("Space reserved for sidebar icons".into()),
                    vec!["Small", "Medium", "Large"],
                    Some("Small"),
                    |_| Message::SectionSelected(Section::Appearance),
                ),
            ]),
            components::section_heading(
                "Motion",
                "Keep feedback expressive without getting in the way"
            ),
            components::settings_group(vec![
                components::setting_toggle(
                    "Reduce motion",
                    Some("Reduce animation and parallax effects".into()),
                    state.reduce_motion,
                    Message::ReduceMotionChanged,
                ),
                components::setting_toggle(
                    "Reduce transparency",
                    Some("Use a more opaque, frosted glass treatment".into()),
                    state.reduce_transparency,
                    Message::ReduceTransparencyChanged,
                ),
                components::setting_toggle(
                    "Increase contrast",
                    Some("Strengthen glass edges and text separation".into()),
                    state.increased_contrast,
                    Message::IncreasedContrastChanged,
                ),
                components::setting_link(
                    "Desktop & Dock",
                    Some("Size, position, and behavior".into()),
                    Message::SectionSelected(Section::Appearance),
                ),
                components::setting_link(
                    "Screen Saver",
                    Some("Idle visuals and timing".into()),
                    Message::SectionSelected(Section::Appearance),
                ),
            ]),
        ]
        .spacing(14)
        .into(),
        Section::Notifications => column![
            description,
            components::section_heading("Notifications", "Choose how apps alert you"),
            components::settings_group(vec![
                components::setting_toggle(
                    "Allow notifications",
                    Some("Show alerts and banners from supported apps".into()),
                    state.notifications,
                    Message::NotificationsChanged,
                ),
                components::setting_pick_list(
                    "Notification style",
                    None,
                    vec!["None", "Banners", "Alerts"],
                    Some("Banners"),
                    |_| Message::SectionSelected(Section::Notifications),
                ),
                components::setting_link(
                    "Scheduled Summary",
                    Some("Deliver a summary at 8:00 AM".into()),
                    Message::SectionSelected(Section::Notifications),
                ),
            ]),
            components::section_heading(
                "App Notifications",
                "Each app can have its own delivery rules"
            ),
            components::settings_group(vec![
                components::setting_link(
                    "Mail",
                    Some("Banners · Sounds".into()),
                    Message::SectionSelected(Section::Notifications)
                ),
                components::setting_link(
                    "Calendar",
                    Some("Alerts · Badges".into()),
                    Message::SectionSelected(Section::Notifications)
                ),
                components::setting_link(
                    "Messages",
                    Some("Banners · Sounds · Badges".into()),
                    Message::SectionSelected(Section::Notifications),
                ),
            ]),
        ]
        .spacing(14)
        .into(),
        Section::Privacy => column![
            description,
            components::section_heading("Privacy & Security", "Control access to your data"),
            components::settings_group(vec![
                components::setting_link(
                    "Location Services",
                    Some("On for 6 apps".into()),
                    Message::SectionSelected(Section::Privacy),
                ),
                components::setting_link(
                    "File and Folder Access",
                    Some("Review permissions".into()),
                    Message::SectionSelected(Section::Privacy),
                ),
                components::setting_link(
                    "Analytics & Improvements",
                    Some("Share diagnostics: Off".into()),
                    Message::SectionSelected(Section::Privacy),
                ),
            ]),
            components::section_heading("Security", "Keep your device and account protected"),
            components::settings_group(vec![
                components::setting_link_with_icon(
                    UiIcon::FileVault,
                    "FileVault",
                    Some("Disk encryption is on".into()),
                    Message::SectionSelected(Section::Privacy),
                ),
                components::setting_link(
                    "Lockdown Mode",
                    Some("Off".into()),
                    Message::SectionSelected(Section::Privacy)
                ),
                components::setting_link(
                    "Passwords",
                    Some("Manage saved credentials".into()),
                    Message::SectionSelected(Section::Privacy),
                ),
            ]),
            components::progress_indicator::<Message, Renderer>(72.0),
        ]
        .spacing(14)
        .into(),
        _ => generic_settings_page(state),
    }
}

fn generic_settings_page(state: &State) -> AppElement<'_> {
    let section = state.active_section;
    let (icon, _, title) = sidebar_metadata(section);
    column![
        text(section_description(section)).size(font::size::BODY),
        components::section_heading(title, "系统设置中的相关选项"),
        components::settings_group(vec![
            components::setting_link_with_icon(
                icon,
                format!("{title} 设置"),
                Some("查看和管理此部分的设置".into()),
                Message::SectionSelected(section),
            ),
            components::setting_link(
                "高级设置",
                Some("更多系统级选项和详细配置".into()),
                Message::SectionSelected(section),
            ),
            components::setting_link(
                "关于此设置",
                Some("了解相关功能和可用选项".into()),
                Message::SectionSelected(section),
            ),
        ]),
    ]
    .spacing(14)
    .into()
}

fn appearance_picker(selected: Appearance) -> AppElement<'static> {
    row![
        segmented_button("Auto", Appearance::Automatic, selected),
        segmented_button("Light", Appearance::Light, selected),
        segmented_button("Dark", Appearance::Dark, selected),
    ]
    .spacing(2)
    .into()
}

fn segmented_button(
    label: &'static str,
    appearance: Appearance,
    selected: Appearance,
) -> AppElement<'static> {
    button(text(label).size(font::size::CAPTION))
        .on_press(Message::AppearanceSelected(appearance))
        .style(move |theme, status| {
            components::segmented_style(theme, status, appearance == selected)
        })
        .into()
}

fn compositor_navigation(scheme: UiColorScheme) -> AppElement<'static> {
    use liquid_glass::{GlassSegment, GlassSegmentContent, GlassSegmentedControl};

    let mut overlay_material = liquid_glass::GlassMaterial::clear();
    overlay_material.tint = liquid_glass::Color::transparent();
    let control = GlassSegmentedControl::new(
        GlassId(12),
        Rect::new(0.0, 0.0, 72.0, 36.0),
        [
            GlassSegment::new(
                GlassSegmentContent::ChevronLeft,
                Message::SectionSelected(Section::General),
            ),
            GlassSegment::new(
                GlassSegmentContent::ChevronRight,
                Message::SectionSelected(Section::General),
            )
            .enabled(false),
        ],
    )
    .material(overlay_material)
    .chrome(UiTheme::new(scheme).compositor_chrome(liquid_glass::GlassRole::FloatingControl))
    .into_element::<Theme, Renderer>();
    // The glyphs are above the separately rendered navigation material. The
    // material itself is inserted by the compositor after toolbar text.
    components::glass_overlay(control)
}

fn main() -> iced::Result {
    let fonts = font::ui_fonts();
    let window_settings = iced::window::Settings {
        size: iced::Size::new(1320.0, 760.0),
        transparent: true,
        blur: true,
        decorations: false,
        ..Default::default()
    };

    let mut app = iced::application::<State, Message, Theme, Renderer>(boot, update, view)
        .title("System Settings")
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
mod sidebar_scrollbar_tests {
    use std::time::Instant;
    use iced::{Point, Size};
    use liquid_glass::ui::{
        SpringScrollState, SpringScrollView,
        scroll_view::scrollbar_opacity,
    };
    use super::*;

    const TEST_SLOT_WIDTH: f32 = 15.0;

    fn sidebar_config() -> ScrollbarConfig {
        ScrollbarConfig::sidebar(
            sidebar_scrollbar_top(),
            SIDEBAR_SCROLLBAR_BOTTOM_INSET,
            SIDEBAR_SCROLLBAR_EDGE_INSET,
        )
    }

    fn scrollbar_at(
        absolute_offset: f32,
    ) -> (SpringScrollView<'static, Message, Theme, Renderer>, SpringScrollState) {
        let view = SpringScrollView::new(space(), UiColorScheme::Light).config(sidebar_config());
        let state = SpringScrollState {
            offset: absolute_offset,
            viewport_height: 600.0,
            content_height: 1_000.0,
            ..SpringScrollState::default()
        };
        (view, state)
    }

    fn rail() -> iced::Rectangle {
        iced::Rectangle::new(
            Point::ORIGIN,
            Size::new(
                TEST_SLOT_WIDTH,
                sidebar_scrollbar_top() + 500.0 + SIDEBAR_SCROLLBAR_BOTTOM_INSET,
            ),
        )
    }

    fn state_at(absolute_offset: f32) -> SpringScrollState {
        SpringScrollState {
            offset: absolute_offset,
            viewport_height: 600.0,
            content_height: 1_000.0,
            ..SpringScrollState::default()
        }
    }

    #[test]
    fn rail_starts_below_the_search_field() {
        assert_eq!(SIDEBAR_SEARCH_HEIGHT, 28.0);
        assert_eq!(
            sidebar_scrollbar_top(),
            SIDEBAR_SEARCH_TOP + SIDEBAR_SEARCH_HEIGHT + SIDEBAR_SCROLLBAR_GAP
        );
        assert_eq!(
            SIDEBAR_SEARCH_TOP,
            FUSED_TOP_BAR_HEIGHT + iced_backend::SIDEBAR_SEARCH_TOP_MARGIN
        );
        assert_eq!(iced_backend::SIDEBAR_SEARCH_TOP_MARGIN, 10.0);
    }

    #[test]
    fn thumb_stays_inside_its_independent_rail() {
        let rail = rail();
        let (scrollbar, top_state) = scrollbar_at(0.0);
        let (_, bottom_state) = scrollbar_at(400.0);
        let top = scrollbar.thumb_bounds(&top_state, rail).expect("overflowing list has a thumb");
        let bottom =
            scrollbar.thumb_bounds(&bottom_state, rail).expect("overflowing list has a thumb");

        assert_eq!(
            top.x,
            TEST_SLOT_WIDTH
                - SIDEBAR_SCROLLBAR_EDGE_INSET
                - SIDEBAR_SCROLLBAR_THUMB_WIDTH
        );
        assert_eq!(top.width, SIDEBAR_SCROLLBAR_THUMB_WIDTH);
        assert_eq!(top.y, sidebar_scrollbar_top());
        assert_eq!(bottom.y + bottom.height, rail.height - SIDEBAR_SCROLLBAR_BOTTOM_INSET);
    }

    #[test]
    fn list_items_keep_the_measured_scrollbar_gap() {
        assert_eq!(SIDEBAR_CONTENT_WIDTH, SIDEBAR_WIDTH - SIDEBAR_CONTENT_INSET * 2.0);
        assert_eq!(SIDEBAR_CONTENT_WIDTH, 212.0);
        assert_eq!(SIDEBAR_CONTENT_INSET, 10.0);
        let item_right = SIDEBAR_WIDTH - SIDEBAR_CONTENT_INSET;
        let thumb_left =
            SIDEBAR_WIDTH - SIDEBAR_SCROLLBAR_EDGE_INSET - SIDEBAR_SCROLLBAR_THUMB_WIDTH;
        assert!((thumb_left - item_right - SIDEBAR_SCROLLBAR_ITEM_GAP).abs() <= f32::EPSILON);
        assert!((SIDEBAR_SCROLLBAR_EDGE_INSET - 3.0).abs() <= f32::EPSILON);
        assert_eq!(SIDEBAR_LIST_BOTTOM_INSET, 0.0);
        assert_eq!(SIDEBAR_SCROLLBAR_BOTTOM_INSET, 3.0);
    }

    #[test]
    fn overscroll_compresses_the_thumb_at_both_edges() {
        let (view, _) = scrollbar_at(0.0);
        let rail = rail();
        let mut top = state_at(0.0);
        top.offset = -60.0;
        let top_thumb = view.thumb_bounds(&top, rail).expect("top thumb");

        let mut bottom = state_at(400.0);
        bottom.offset = bottom.scroll_range() + 60.0;
        let bottom_thumb = view.thumb_bounds(&bottom, rail).expect("bottom thumb");
        let normal = view.thumb_bounds(&state_at(200.0), rail).expect("normal thumb");

        assert!(top_thumb.height < normal.height);
        assert!(bottom_thumb.height < normal.height);
        assert_eq!(top_thumb.y, sidebar_scrollbar_top());
        assert_eq!(
            bottom_thumb.y + bottom_thumb.height,
            rail.height - SIDEBAR_SCROLLBAR_BOTTOM_INSET
        );
    }

    #[test]
    fn scrollbar_is_hidden_until_the_list_moves() {
        let config = sidebar_config();
        assert_eq!(scrollbar_opacity(None, Instant::now(), config.hold_duration, config.fade_duration), 0.0);
    }

    #[test]
    fn scrollbar_holds_then_fades_out() {
        let started = Instant::now();
        let config = sidebar_config();
        assert_eq!(scrollbar_opacity(Some(started), started, config.hold_duration, config.fade_duration), 1.0);
        assert_eq!(scrollbar_opacity(Some(started), started + config.hold_duration, config.hold_duration, config.fade_duration), 1.0);
        let halfway = started + config.hold_duration + config.fade_duration / 2;
        assert!((scrollbar_opacity(Some(started), halfway, config.hold_duration, config.fade_duration) - 0.5).abs() < 0.01);
        assert_eq!(
            scrollbar_opacity(
                Some(started),
                started + config.hold_duration + config.fade_duration,
                config.hold_duration,
                config.fade_duration,
            ),
            0.0
        );
    }

    #[test]
    fn rail_clicks_map_to_clamped_relative_offsets() {
        let rail = rail();
        let (scrollbar, state) = scrollbar_at(0.0);
        let thumb = scrollbar.thumb_bounds(&state, rail).expect("overflowing list has a thumb");

        assert_eq!(
            scrollbar.requested_offset(
                &state,
                rail,
                sidebar_scrollbar_top() - 20.0,
                thumb.height * 0.5
            ),
            0.0
        );
        assert_eq!(
            scrollbar.requested_offset(&state, rail, rail.height + 20.0, thumb.height * 0.5),
            400.0
        );
    }

    #[test]
    fn offset_is_clamped_to_content_range() {
        let mut state = state_at(0.0);
        assert!(state.set_offset(999.0));
        assert_eq!(state.offset, 400.0);
        assert!(state.set_offset(-1.0));
        assert_eq!(state.offset, 0.0);
    }
}

#[cfg(test)]
mod window_and_traffic_lights_tests {
    use super::*;

    #[test]
    fn test_window_ready_attaches_window_id() {
        let mut state = State::default();
        assert_eq!(state.window_id, None);
        assert!(state.window_focused);

        let test_id = iced::window::Id::unique();
        let _ = update(&mut state, Message::WindowReady(Some(test_id)));
        assert_eq!(state.window_id, Some(test_id));
    }

    #[test]
    fn test_window_focus_and_unfocus_events() {
        let mut state = State::default();
        let test_id = iced::window::Id::unique();
        let _ = update(&mut state, Message::WindowReady(Some(test_id)));

        state.traffic_lights.on_group_hover(true);
        assert_eq!(state.traffic_lights.hover_target, 1.0);

        let _ = update(&mut state, Message::WindowEvent((test_id, iced::window::Event::Unfocused)));
        assert!(!state.window_focused);
        assert_eq!(state.traffic_lights.hover_target, 0.0);

        let _ = update(&mut state, Message::WindowEvent((test_id, iced::window::Event::Focused)));
        assert!(state.window_focused);
    }

    #[test]
    fn test_traffic_lights_hover_state() {
        let mut state = State::default();
        assert_eq!(state.traffic_lights.hover_target, 0.0);

        let _ = update(&mut state, Message::ControlGroupHover(true));
        assert_eq!(state.traffic_lights.hover_target, 1.0);

        let _ = update(&mut state, Message::ControlGroupHover(false));
        assert_eq!(state.traffic_lights.hover_target, 0.0);
    }
}

