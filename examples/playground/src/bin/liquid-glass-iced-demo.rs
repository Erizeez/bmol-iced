#[path = "../iced_backend.rs"]
mod iced_backend;

use iced::{
    Color, Element, Length, Subscription, Task, Theme,
    widget::{button, column, container, row, scrollable, space, stack, text},
};
use iced_backend::{CONTENT_TOP_INSET, Renderer};
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

fn subscription(_state: &State) -> Subscription<Message> {
    iced::system::theme_changes().map(Message::SystemThemeChanged)
}

fn app_theme(state: &State) -> Theme {
    UiTheme::new(state.color_scheme()).iced_theme()
}

fn view(state: &State) -> AppElement<'_> {
    let sidebar_scroll = scrollable(column![
        // Reserve the search field's vertical footprint inside the
        // scrollable content. The first row starts below the field at the
        // initial offset, but this space scrolls away with the list so
        // rows can still pass behind the floating search field.
        space().height(Length::Fixed(CONTENT_TOP_INSET + 58.0)),
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
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::default()))
    .style(components::sidebar_scrollable_style);

    let search_overlay = container(components::search_field(
        GlassId(11),
        Rect::new(0.0, 0.0, 212.0, 36.0),
        state.color_scheme(),
        &state.search,
        Message::SearchChanged,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(iced::Padding::new(0.0).top(CONTENT_TOP_INSET + 10.0));

    let sidebar: AppElement<'_> = container(stack![sidebar_scroll, search_overlay])
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
