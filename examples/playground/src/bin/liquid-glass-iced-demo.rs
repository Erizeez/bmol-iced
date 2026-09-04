#[path = "../iced_backend.rs"]
mod iced_backend;

use std::time::{Duration, Instant};

use iced::{
    Color, Element, Length, Point, Size, Subscription, Task, Theme, mouse,
    widget::{button, canvas, column, container, row, scrollable, space, stack, text},
};
use iced_backend::{CONTENT_TOP_INSET, Renderer, SIDEBAR_SEARCH_HEIGHT, SIDEBAR_SEARCH_TOP_MARGIN};
use liquid_glass::{
    GlassAccessibility, GlassId, Rect, UiColorScheme, UiIcon, UiTheme,
    ui::{components, font},
};

struct State {
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
    content_scroll: f32,
    sidebar_scroll: SidebarScrollMetrics,
    sidebar_scrollbar_last_activity: Option<Instant>,
    system_scheme: UiColorScheme,
}

impl Default for State {
    fn default() -> Self {
        Self {
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
            content_scroll: 0.0,
            sidebar_scroll: SidebarScrollMetrics::default(),
            sidebar_scrollbar_last_activity: None,
            system_scheme: UiColorScheme::Dark,
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
    ContentScrolled(f32),
    SidebarViewportChanged(SidebarScrollMetrics),
    SidebarScrollRequested(f32),
    SidebarScrollbarTick(Instant),
    SystemThemeChanged(iced::theme::Mode),
}

type AppElement<'a> = Element<'a, Message, Theme, Renderer>;

fn boot() -> (State, Task<Message>) {
    let state = State::default();
    iced_backend::set_color_scheme(state.color_scheme());
    iced_backend::set_accessibility(state.accessibility());
    (state, iced::system::theme().map(Message::SystemThemeChanged))
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
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
        Message::ContentScrolled(offset) => state.content_scroll = offset,
        Message::SidebarViewportChanged(metrics) => {
            let was_initialized = state.sidebar_scroll.content_height > 0.0;
            let moved =
                (metrics.absolute_offset - state.sidebar_scroll.absolute_offset).abs() > 0.01;
            state.sidebar_scroll = metrics;
            if was_initialized && moved {
                state.sidebar_scrollbar_last_activity = Some(Instant::now());
            }
        }
        Message::SidebarScrollRequested(relative_offset) => {
            state.sidebar_scrollbar_last_activity = Some(Instant::now());
            return iced::widget::operation::snap_to(
                sidebar_scroll_id(),
                scrollable::RelativeOffset { x: 0.0, y: relative_offset.clamp(0.0, 1.0) },
            );
        }
        Message::SidebarScrollbarTick(now) => {
            if sidebar_scrollbar_opacity(state.sidebar_scrollbar_last_activity, now) <= 0.0 {
                state.sidebar_scrollbar_last_activity = None;
            }
        }
        Message::SystemThemeChanged(mode) => {
            state.system_scheme = UiColorScheme::from_mode(mode);
        }
    }
    iced_backend::set_color_scheme(state.color_scheme());
    iced_backend::set_accessibility(state.accessibility());
    Task::none()
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
    let scrollbar_fade = if state.sidebar_scrollbar_last_activity.is_some() {
        iced::time::every(Duration::from_millis(16)).map(Message::SidebarScrollbarTick)
    } else {
        Subscription::none()
    };
    Subscription::batch([
        iced::system::theme_changes().map(Message::SystemThemeChanged),
        scrollbar_fade,
    ])
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
            CONTENT_TOP_INSET
                + SIDEBAR_SEARCH_TOP_MARGIN
                + SIDEBAR_SEARCH_HEIGHT
                + SIDEBAR_FIRST_ROW_GAP,
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
    // The selection pill ends exactly one logical pixel before the
    // visible scrollbar thumb. The transparent remainder belongs to the
    // scrollbar's wider hit target, not to list-row painting.
    .padding(iced::Padding::new(0.0).right(sidebar_list_right_inset()));

    let sidebar_scroll = scrollable(sidebar_list)
        .width(Length::Fill)
        .height(Length::Fill)
        // The content viewport intentionally extends behind the floating search
        // field so its pixels remain available to blur and glass refraction. Its
        // scrollbar is a separate visual/interaction layer below.
        .id(sidebar_scroll_id())
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::hidden()))
        .style(components::sidebar_scrollable_style)
        .on_scroll(|viewport| {
            Message::SidebarViewportChanged(SidebarScrollMetrics::from_viewport(viewport))
        });

    let search_overlay = container(components::search_field(
        GlassId(11),
        Rect::new(0.0, 0.0, 212.0, SIDEBAR_SEARCH_HEIGHT),
        state.color_scheme(),
        &state.search,
        Message::SearchChanged,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(iced::Padding::new(0.0).top(CONTENT_TOP_INSET + SIDEBAR_SEARCH_TOP_MARGIN));

    // Unlike the optical content overscan above, this is the actual scrollbar
    // viewport. Its rail starts below the search field and never enters the
    // titlebar/search occlusion region.
    let scrollbar_overlay = container(
        canvas(SidebarScrollbar {
            metrics: state.sidebar_scroll,
            color_scheme: state.color_scheme(),
            opacity: sidebar_scrollbar_opacity(
                state.sidebar_scrollbar_last_activity,
                Instant::now(),
            ),
        })
        .width(Length::Fixed(SIDEBAR_SCROLLBAR_SLOT_WIDTH))
        .height(Length::Fill),
    )
    .align_right(Length::Fill)
    .height(Length::Fill)
    .padding(iced::Padding {
        top: sidebar_scrollbar_top(),
        right: 0.0,
        bottom: 2.0,
        left: 0.0,
    });

    let sidebar: AppElement<'_> =
        container(stack![sidebar_scroll, search_overlay, scrollbar_overlay])
            .width(Length::Fixed(232.0))
            .height(Length::Fill)
            .padding(iced::Padding { top: 0.0, right: 10.0, bottom: 10.0, left: 10.0 })
            .style(components::transparent_surface)
            .into();

    let toolbar = components::glass_surface(
        GlassId(10),
        Rect::new(0.0, 0.0, 1088.0, 56.0),
        liquid_glass::GlassRole::Toolbar,
        state.color_scheme(),
        iced::Padding::new(8.0),
        None,
        row![
            compositor_navigation(state.color_scheme()),
            space().width(Length::Fill),
            text(section_title(state.active_section))
                .size(font::size::HEADLINE)
                .font(font::ui_font(iced::font::Weight::Semibold)),
            space().width(Length::Fill),
            button(components::icon_tinted(UiIcon::Question, 16.0, |palette| {
                palette.text_secondary
            }))
            .on_press(Message::SectionSelected(state.active_section))
            .width(Length::Fixed(36.0))
            .style(components::toolbar_button_style),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    );

    let content_body = container(settings_page(state))
        .width(Length::Fill)
        .max_width(720.0)
        .padding(iced::Padding::new(22.0).top(78.0));
    let content_scroll = scrollable(content_body)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(components::scrollable_style)
        .on_scroll(|viewport| Message::ContentScrolled(viewport.absolute_offset().y));
    let content = container(content_scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([0, 34])
        .center_x(Length::Fill)
        .style(components::compositor_content_surface);

    let main = container(stack![content, toolbar]).width(Length::Fill).height(Length::Fill);

    let window_content = container(row![sidebar, main]).width(Length::Fill).height(Length::Fill);
    window_content.into()
}

const SIDEBAR_SCROLL_ID: &str = "settings-sidebar-scroll";
const SIDEBAR_SCROLLBAR_GAP: f32 = 8.0;
const SIDEBAR_FIRST_ROW_GAP: f32 = 12.0;
const SIDEBAR_SCROLLBAR_MIN_THUMB: f32 = 36.0;
const SIDEBAR_SCROLLBAR_SLOT_WIDTH: f32 = 15.0;
const SIDEBAR_SCROLLBAR_THUMB_WIDTH: f32 = 8.0 * 0.70;
const SIDEBAR_SELECTION_SCROLLBAR_GAP: f32 = 1.0;
const SIDEBAR_SCROLLBAR_HOLD: Duration = Duration::from_millis(650);
const SIDEBAR_SCROLLBAR_FADE: Duration = Duration::from_millis(250);

fn sidebar_scroll_id() -> iced::widget::Id {
    iced::widget::Id::new(SIDEBAR_SCROLL_ID)
}

fn sidebar_scrollbar_top() -> f32 {
    CONTENT_TOP_INSET + SIDEBAR_SEARCH_TOP_MARGIN + SIDEBAR_SEARCH_HEIGHT + SIDEBAR_SCROLLBAR_GAP
}

fn sidebar_list_right_inset() -> f32 {
    SIDEBAR_SCROLLBAR_THUMB_WIDTH + SIDEBAR_SELECTION_SCROLLBAR_GAP
}

fn sidebar_scrollbar_opacity(last_activity: Option<Instant>, now: Instant) -> f32 {
    let Some(last_activity) = last_activity else {
        return 0.0;
    };
    let elapsed = now.saturating_duration_since(last_activity);
    if elapsed <= SIDEBAR_SCROLLBAR_HOLD {
        1.0
    } else {
        1.0 - (elapsed - SIDEBAR_SCROLLBAR_HOLD).as_secs_f32()
            / SIDEBAR_SCROLLBAR_FADE.as_secs_f32()
    }
    .clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy, Default)]
struct SidebarScrollMetrics {
    absolute_offset: f32,
    viewport_height: f32,
    content_height: f32,
}

impl SidebarScrollMetrics {
    fn from_viewport(viewport: scrollable::Viewport) -> Self {
        Self {
            absolute_offset: viewport.absolute_offset().y,
            viewport_height: viewport.bounds().height,
            content_height: viewport.content_bounds().height,
        }
    }

    fn relative_offset(self) -> f32 {
        let scroll_range = (self.content_height - self.viewport_height).max(0.0);
        if scroll_range <= f32::EPSILON {
            0.0
        } else {
            (self.absolute_offset / scroll_range).clamp(0.0, 1.0)
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SidebarScrollbar {
    metrics: SidebarScrollMetrics,
    color_scheme: UiColorScheme,
    opacity: f32,
}

#[derive(Debug, Default)]
struct SidebarScrollbarState {
    grabbed_at: Option<f32>,
}

impl SidebarScrollbar {
    fn thumb_bounds(self, bounds: iced::Rectangle) -> Option<iced::Rectangle> {
        if self.metrics.content_height <= self.metrics.viewport_height || bounds.height <= 0.0 {
            return None;
        }

        let visible_ratio =
            (self.metrics.viewport_height / self.metrics.content_height).clamp(0.0, 1.0);
        let height =
            (bounds.height * visible_ratio).max(SIDEBAR_SCROLLBAR_MIN_THUMB).min(bounds.height);
        let y = (bounds.height - height) * self.metrics.relative_offset();

        Some(iced::Rectangle::new(
            Point::new(bounds.width - SIDEBAR_SCROLLBAR_THUMB_WIDTH, y),
            Size::new(SIDEBAR_SCROLLBAR_THUMB_WIDTH, height),
        ))
    }

    fn requested_offset(self, bounds: iced::Rectangle, pointer_y: f32, grabbed_at: f32) -> f32 {
        let Some(thumb) = self.thumb_bounds(bounds) else {
            return 0.0;
        };
        let travel = bounds.height - thumb.height;
        if travel <= f32::EPSILON {
            0.0
        } else {
            ((pointer_y - grabbed_at) / travel).clamp(0.0, 1.0)
        }
    }
}

impl canvas::Program<Message, Theme, Renderer> for SidebarScrollbar {
    type State = SidebarScrollbarState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: iced::Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if self.opacity <= 0.0 {
            return None;
        }
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pointer = cursor.position_in(bounds)?;
                let thumb = self.thumb_bounds(bounds)?;
                let grabbed_at =
                    if thumb.contains(pointer) { pointer.y - thumb.y } else { thumb.height * 0.5 };
                state.grabbed_at = Some(grabbed_at);
                Some(
                    canvas::Action::publish(Message::SidebarScrollRequested(
                        self.requested_offset(bounds, pointer.y, grabbed_at),
                    ))
                    .and_capture(),
                )
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let grabbed_at = state.grabbed_at?;
                let pointer = cursor.position_in(bounds)?;
                Some(
                    canvas::Action::publish(Message::SidebarScrollRequested(
                        self.requested_offset(bounds, pointer.y, grabbed_at),
                    ))
                    .and_capture(),
                )
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.grabbed_at.take().is_some() =>
            {
                Some(canvas::Action::capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        if self.opacity <= 0.0 {
            return Vec::new();
        }
        let Some(thumb) = self.thumb_bounds(bounds) else {
            return Vec::new();
        };
        let hovered = cursor.position_in(bounds).is_some_and(|position| thumb.contains(position));
        let active = state.grabbed_at.is_some();

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let path = canvas::Path::rounded_rectangle(
            thumb.position(),
            thumb.size(),
            (thumb.width * 0.5).into(),
        );
        let emphasis = if active {
            0.62
        } else if hovered {
            0.48
        } else {
            0.34
        };
        let color = match self.color_scheme {
            UiColorScheme::Light => Color::from_rgba(0.18, 0.18, 0.20, emphasis * self.opacity),
            UiColorScheme::Dark => {
                Color::from_rgba(0.92, 0.92, 0.94, (emphasis + 0.08) * self.opacity)
            }
        };
        frame.fill(&path, color);
        vec![frame.into_geometry()]
    }
}

fn sidebar_group(sections: &[Section], active: Section) -> AppElement<'static> {
    container(
        column(
            sections
                .iter()
                .copied()
                .map(|section| components::glass_foreground(sidebar_entry(section, active)))
                .collect::<Vec<_>>(),
        )
        .spacing(2),
    )
    .width(Length::Fill)
    .into()
}

fn sidebar_gap() -> AppElement<'static> {
    space().height(Length::Fixed(8.0)).into()
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
    let window_config = liquid_glass::WindowConfig::desktop_backdrop();
    let window_settings = iced::window::Settings {
        transparent: window_config.transparent,
        // winit requests compositor blur without adding a view above the
        // wgpu CAMetalLayer, so Iced content remains visible.
        blur: window_config.blur && (cfg!(target_os = "linux") || cfg!(target_os = "macos")),
        #[cfg(target_os = "macos")]
        platform_specific: iced::window::settings::PlatformSpecific {
            // Let the Iced surface extend beneath the native titlebar. The
            // sidebar compositor now owns the background behind the traffic
            // lights, while content keeps its inset through layout padding.
            titlebar_transparent: true,
            title_hidden: true,
            fullsize_content_view: true,
            ..iced::window::settings::PlatformSpecific::default()
        },
        #[cfg(not(target_os = "macos"))]
        platform_specific: iced::window::settings::PlatformSpecific::default(),
        ..iced::window::Settings::default()
    };

    let mut app = iced::application::<State, Message, Theme, Renderer>(boot, update, view)
        .title("System Settings")
        .theme(app_theme)
        .subscription(subscription)
        .window(window_settings)
        .window_size(iced::Size::new(1320.0, 760.0));
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
    use super::*;

    fn scrollbar_at(absolute_offset: f32) -> SidebarScrollbar {
        SidebarScrollbar {
            metrics: SidebarScrollMetrics {
                absolute_offset,
                viewport_height: 600.0,
                content_height: 1_000.0,
            },
            color_scheme: UiColorScheme::Light,
            opacity: 1.0,
        }
    }

    #[test]
    fn rail_starts_below_the_search_field() {
        assert_eq!(SIDEBAR_SEARCH_HEIGHT, 28.0);
        assert_eq!(
            sidebar_scrollbar_top(),
            CONTENT_TOP_INSET
                + SIDEBAR_SEARCH_TOP_MARGIN
                + SIDEBAR_SEARCH_HEIGHT
                + SIDEBAR_SCROLLBAR_GAP
        );
    }

    #[test]
    fn thumb_stays_inside_its_independent_rail() {
        let rail =
            iced::Rectangle::new(Point::ORIGIN, Size::new(SIDEBAR_SCROLLBAR_SLOT_WIDTH, 500.0));
        let top = scrollbar_at(0.0).thumb_bounds(rail).expect("overflowing list has a thumb");
        let bottom = scrollbar_at(400.0).thumb_bounds(rail).expect("overflowing list has a thumb");

        assert_eq!(top.x, SIDEBAR_SCROLLBAR_SLOT_WIDTH - SIDEBAR_SCROLLBAR_THUMB_WIDTH);
        assert_eq!(top.width, SIDEBAR_SCROLLBAR_THUMB_WIDTH);
        assert_eq!(top.y, 0.0);
        assert!((bottom.y + bottom.height - rail.height).abs() <= f32::EPSILON);
    }

    #[test]
    fn selection_ends_one_pixel_before_the_thumb() {
        let content_width = 212.0;
        let rail_left = content_width - SIDEBAR_SCROLLBAR_SLOT_WIDTH;
        let thumb_left = rail_left + SIDEBAR_SCROLLBAR_SLOT_WIDTH - SIDEBAR_SCROLLBAR_THUMB_WIDTH;
        let selection_right = content_width - sidebar_list_right_inset();

        assert_eq!(thumb_left - selection_right, SIDEBAR_SELECTION_SCROLLBAR_GAP);
    }

    #[test]
    fn scrollbar_is_hidden_until_the_list_moves() {
        assert_eq!(sidebar_scrollbar_opacity(None, Instant::now()), 0.0);
    }

    #[test]
    fn scrollbar_holds_then_fades_out() {
        let started = Instant::now();
        assert_eq!(sidebar_scrollbar_opacity(Some(started), started), 1.0);
        assert_eq!(sidebar_scrollbar_opacity(Some(started), started + SIDEBAR_SCROLLBAR_HOLD), 1.0);
        let halfway = started + SIDEBAR_SCROLLBAR_HOLD + SIDEBAR_SCROLLBAR_FADE / 2;
        assert!((sidebar_scrollbar_opacity(Some(started), halfway) - 0.5).abs() < 0.01);
        assert_eq!(
            sidebar_scrollbar_opacity(
                Some(started),
                started + SIDEBAR_SCROLLBAR_HOLD + SIDEBAR_SCROLLBAR_FADE,
            ),
            0.0
        );
    }

    #[test]
    fn rail_clicks_map_to_clamped_relative_offsets() {
        let rail =
            iced::Rectangle::new(Point::ORIGIN, Size::new(SIDEBAR_SCROLLBAR_SLOT_WIDTH, 500.0));
        let scrollbar = scrollbar_at(0.0);
        let thumb = scrollbar.thumb_bounds(rail).expect("overflowing list has a thumb");

        assert_eq!(scrollbar.requested_offset(rail, -20.0, thumb.height * 0.5), 0.0);
        assert_eq!(scrollbar.requested_offset(rail, 520.0, thumb.height * 0.5), 1.0);
    }
}
