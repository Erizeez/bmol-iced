#[path = "../iced_backend.rs"]
mod iced_backend;

use iced::{
    Background, Border, Color, Element, Length, Shadow, Subscription, Task, Theme,
    widget::{
        button, column, container, progress_bar, row, rule, scrollable, slider, space, stack, text,
        text_input, toggler,
    },
};
use iced_backend::Renderer;
use liquid_glass::{
    GlassButton, GlassContainer, GlassId, GlassMaterial, GlassRole, GlassShape, Rect,
    UiColorScheme, UiPalette, UiTheme,
};

struct State {
    active_section: Section,
    appearance: Appearance,
    search: String,
    auto_updates: bool,
    notifications: bool,
    reduce_motion: bool,
    volume: f32,
    system_scheme: UiColorScheme,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_section: Section::General,
            appearance: Appearance::Automatic,
            search: String::new(),
            auto_updates: true,
            notifications: true,
            reduce_motion: false,
            volume: 64.0,
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
    General,
    Appearance,
    Notifications,
    Privacy,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Appearance {
    #[default]
    Automatic,
    Light,
    Dark,
}

#[derive(Debug, Clone)]
enum Message {
    SectionSelected(Section),
    AppearanceSelected(Appearance),
    SearchChanged(String),
    AutoUpdatesChanged(bool),
    NotificationsChanged(bool),
    ReduceMotionChanged(bool),
    VolumeChanged(f32),
    SystemThemeChanged(iced::theme::Mode),
}

type AppElement<'a> = Element<'a, Message, Theme, Renderer>;

fn boot() -> (State, Task<Message>) {
    let state = State::default();
    iced_backend::set_color_scheme(state.color_scheme());
    (state, iced::system::theme().map(Message::SystemThemeChanged))
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::SectionSelected(section) => state.active_section = section,
        Message::AppearanceSelected(appearance) => state.appearance = appearance,
        Message::SearchChanged(search) => state.search = search,
        Message::AutoUpdatesChanged(enabled) => state.auto_updates = enabled,
        Message::NotificationsChanged(enabled) => state.notifications = enabled,
        Message::ReduceMotionChanged(enabled) => state.reduce_motion = enabled,
        Message::VolumeChanged(volume) => state.volume = volume,
        Message::SystemThemeChanged(mode) => {
            state.system_scheme = UiColorScheme::from_mode(mode);
        }
    }
    iced_backend::set_color_scheme(state.color_scheme());
    Task::none()
}

fn subscription(_state: &State) -> Subscription<Message> {
    iced::system::theme_changes().map(Message::SystemThemeChanged)
}

fn app_theme(state: &State) -> Theme {
    UiTheme::new(state.color_scheme()).iced_theme()
}

#[allow(clippy::too_many_lines)]
fn view(state: &State) -> AppElement<'_> {
    let sidebar: AppElement<'_> = container(column![
        glass_search(&state.search, state.color_scheme()),
        space().height(Length::Fixed(10.0)),
        sidebar_button("⚙︎   General", Section::General, state.active_section),
        sidebar_button("◐   Appearance", Section::Appearance, state.active_section),
        sidebar_button("◉   Notifications", Section::Notifications, state.active_section),
        sidebar_button("◆   Privacy & Security", Section::Privacy, state.active_section),
        space().height(Length::Fill),
        text("Liquid Glass UI").size(11),
    ])
    .width(Length::Fixed(232.0))
    .height(Length::Fill)
    .padding([10, 10])
    .style(sidebar_style)
    .into();

    let toolbar = glass_surface_with_padding(
        GlassId(10),
        Rect::new(0.0, 0.0, 1087.0, 56.0),
        GlassShape::RoundedRect { radius: 0.0 },
        GlassRole::Toolbar,
        state.color_scheme(),
        8.0,
        row![
            compositor_button(
                GlassId(12),
                "‹",
                Message::SectionSelected(state.active_section),
                36.0,
                40.0,
                state.color_scheme(),
            ),
            compositor_button(
                GlassId(13),
                "›",
                Message::SectionSelected(state.active_section),
                36.0,
                40.0,
                state.color_scheme(),
            ),
            space().width(Length::Fill),
            text(section_title(state.active_section)).size(15),
            space().width(Length::Fill),
            button(text("?").size(14))
                .on_press(Message::SectionSelected(state.active_section))
                .width(Length::Fixed(36.0))
                .style(toolbar_button_style),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    );

    let content_body = container(settings_page(state)).width(Length::Fill).max_width(720.0);
    let content = container(scrollable(content_body).width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([22, 34])
        .center_x(Length::Fill)
        .style(content_style);

    let main = column![toolbar, content].width(Length::Fill).height(Length::Fill);

    container(row![sidebar, rule::vertical(1).style(split_rule_style), main])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn sidebar_button(label: &'static str, section: Section, active: Section) -> AppElement<'static> {
    let selected = section == active;
    button(text(label))
        .on_press(Message::SectionSelected(section))
        .width(Length::Fill)
        .style(move |theme, status| sidebar_button_style(theme, status, selected))
        .into()
}

fn section_title(section: Section) -> &'static str {
    match section {
        Section::General => "General",
        Section::Appearance => "Appearance",
        Section::Notifications => "Notifications",
        Section::Privacy => "Privacy & Security",
    }
}

fn section_description(section: Section) -> &'static str {
    match section {
        Section::General => "Your Mac, your rules. Make the everyday details feel right.",
        Section::Appearance => "A small set of controls with a clear, familiar hierarchy.",
        Section::Notifications => "Decide what deserves your attention and when.",
        Section::Privacy => "Review access without burying the important choices.",
    }
}

#[allow(clippy::too_many_lines)]
fn settings_page(state: &State) -> AppElement<'_> {
    match state.active_section {
        Section::General => column![
            text(section_description(state.active_section)).size(13),
            section_heading("General", "The essentials for this Mac"),
            settings_group(
                setting_link("About This Mac", "Apple M3 Pro · 18 GB memory", "›"),
                setting_link("Software Update", "macOS is up to date", "›"),
                setting_toggle(
                    "Automatic Updates",
                    "Install security responses and system files automatically",
                    toggler(state.auto_updates).on_toggle(Message::AutoUpdatesChanged),
                ),
            ),
            section_heading("Input & Output", "The small details that shape every interaction"),
            settings_group(
                setting_control(
                    "Sound volume",
                    format!("{}%", state.volume.round()),
                    slider(0.0..=100.0, state.volume, Message::VolumeChanged)
                        .width(Length::Fixed(170.0)),
                ),
                setting_link("Keyboard", "Key repeat and modifier keys", "›"),
                setting_link("Trackpad", "Point & click, scroll & zoom", "›"),
            ),
        ]
        .spacing(14)
        .into(),
        Section::Appearance => column![
            text(section_description(state.active_section)).size(13),
            section_heading("Appearance", "Personalize the way your system looks"),
            settings_group(
                setting_control(
                    "Appearance",
                    "Choose how windows, controls, and menus look",
                    appearance_picker(state.appearance),
                ),
                setting_control(
                    "Accent color",
                    "Used for buttons, links, and selected items",
                    accent_picker(),
                ),
                setting_control(
                    "Sidebar icon size",
                    "Small",
                    button(text("Small  ›"))
                        .on_press(Message::SectionSelected(Section::Appearance))
                        .style(list_button_style),
                ),
            ),
            section_heading("Motion", "Keep feedback expressive without getting in the way"),
            settings_group(
                setting_toggle(
                    "Reduce motion",
                    "Reduce animation and parallax effects",
                    toggler(state.reduce_motion).on_toggle(Message::ReduceMotionChanged),
                ),
                setting_link("Desktop & Dock", "Size, position, and behavior", "›"),
                setting_link("Screen Saver", "Idle visuals and timing", "›"),
            ),
        ]
        .spacing(14)
        .into(),
        Section::Notifications => column![
            text(section_description(state.active_section)).size(13),
            section_heading("Notifications", "Choose how apps alert you"),
            settings_group(
                setting_toggle(
                    "Allow notifications",
                    "Show alerts and banners from supported apps",
                    toggler(state.notifications).on_toggle(Message::NotificationsChanged),
                ),
                setting_control(
                    "Notification style",
                    "Banners",
                    button(text("Banners  ›"))
                        .on_press(Message::SectionSelected(Section::Notifications))
                        .style(list_button_style),
                ),
                setting_link("Scheduled Summary", "Deliver a summary at 8:00 AM", "›"),
            ),
            section_heading("App Notifications", "Each app can have its own delivery rules"),
            settings_group(
                setting_link("Mail", "Banners · Sounds", "›"),
                setting_link("Calendar", "Alerts · Badges", "›"),
                setting_link("Messages", "Banners · Sounds · Badges", "›"),
            ),
        ]
        .spacing(14)
        .into(),
        Section::Privacy => column![
            text(section_description(state.active_section)).size(13),
            section_heading("Privacy & Security", "Control access to your data"),
            settings_group(
                setting_link("Location Services", "On for 6 apps", "›"),
                setting_link("File and Folder Access", "Review permissions", "›"),
                setting_link("Analytics & Improvements", "Share diagnostics: Off", "›"),
            ),
            section_heading("Security", "Keep your device and account protected"),
            settings_group(
                setting_link("FileVault", "Disk encryption is on", "›"),
                setting_link("Lockdown Mode", "Off", "›"),
                setting_link("Passwords", "Manage saved credentials", "›"),
            ),
            container(progress_bar(0.0..=100.0, 72.0))
                .width(Length::Fill)
                .height(Length::Fixed(4.0))
                .style(progress_style),
        ]
        .spacing(14)
        .into(),
    }
}

fn section_heading(title: &'static str, subtitle: &'static str) -> AppElement<'static> {
    column![
        text(title).size(20),
        text(subtitle)
            .size(12)
            .style(|theme| text::Style { color: Some(palette(theme).text_secondary) })
    ]
    .spacing(3)
    .padding(iced::Padding::new(8.0).left(4.0))
    .into()
}

fn settings_group(
    first: AppElement<'static>,
    second: AppElement<'static>,
    third: impl Into<AppElement<'static>>,
) -> AppElement<'static> {
    container(column![
        first,
        rule::horizontal(1).style(rule_style),
        second,
        rule::horizontal(1).style(rule_style),
        third.into()
    ])
    .width(Length::Fill)
    .padding(4)
    .style(list_group_style)
    .into()
}

fn setting_link(
    label: &'static str,
    detail: &'static str,
    trailing: &'static str,
) -> AppElement<'static> {
    setting_control(
        label,
        detail,
        button(row![space().width(Length::Fill), text(trailing).size(22)])
            .on_press(Message::SectionSelected(Section::General))
            .width(Length::Fixed(64.0))
            .style(list_button_style),
    )
}

fn setting_toggle(
    label: &'static str,
    detail: &'static str,
    control: impl Into<AppElement<'static>>,
) -> AppElement<'static> {
    setting_control(label, detail, control)
}

fn setting_control(
    label: &'static str,
    detail: impl Into<String>,
    control: impl Into<AppElement<'static>>,
) -> AppElement<'static> {
    container(
        row![
            column![
                text(label).size(14),
                text(detail.into())
                    .size(11)
                    .style(|theme| text::Style { color: Some(palette(theme).text_secondary) })
            ]
            .spacing(3)
            .width(Length::Fill),
            control.into(),
        ]
        .spacing(18)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .padding([12, 14])
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
    button(text(label).size(11))
        .on_press(Message::AppearanceSelected(appearance))
        .style(move |theme, status| segmented_button_style(theme, status, appearance == selected))
        .into()
}

fn accent_picker() -> AppElement<'static> {
    row![
        accent_dot(Color::from_rgb(0.20, 0.48, 1.0)),
        accent_dot(Color::from_rgb(0.60, 0.35, 0.95)),
        accent_dot(Color::from_rgb(0.12, 0.72, 0.54)),
        accent_dot(Color::from_rgb(0.95, 0.38, 0.40)),
    ]
    .spacing(7)
    .into()
}

fn accent_dot(color: Color) -> AppElement<'static> {
    container(text(" "))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .style(move |theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border::default().rounded(7.0).width(1.0).color(palette(theme).group_border),
            ..container::Style::default()
        })
        .into()
}

fn glass_search(value: &str, scheme: UiColorScheme) -> AppElement<'_> {
    let field = text_input("Search", value)
        .on_input(Message::SearchChanged)
        .width(Length::Fill)
        .padding([5, 8])
        .style(search_style);
    glass_surface_with_padding(
        GlassId(11),
        Rect::new(0.0, 0.0, 212.0, 36.0),
        GlassShape::Superellipse { exponent: 4.5 },
        GlassRole::SearchField,
        scheme,
        2.0,
        field,
    )
}

fn glass_surface_with_padding<'a>(
    id: GlassId,
    bounds: Rect,
    shape: GlassShape,
    role: GlassRole,
    scheme: UiColorScheme,
    padding: f32,
    content: impl Into<AppElement<'a>>,
) -> AppElement<'a> {
    let mut overlay_material = GlassMaterial::clear();
    overlay_material.tint = liquid_glass::Color::transparent();
    let background = GlassContainer::new(id, bounds)
        .shape(shape)
        .material(overlay_material)
        .chrome(UiTheme::new(scheme).glass_chrome(role))
        .into_element::<Message, Theme, Renderer>();
    let foreground: AppElement<'a> = container(content)
        .width(Length::Fixed(bounds.width))
        .height(Length::Fixed(bounds.height))
        .padding(padding)
        .into();
    stack![background, foreground].into()
}

fn compositor_button(
    id: GlassId,
    label: &'static str,
    message: Message,
    width: f32,
    height: f32,
    scheme: UiColorScheme,
) -> AppElement<'static> {
    let mut overlay_material = GlassMaterial::clear();
    overlay_material.tint = liquid_glass::Color::transparent();
    GlassButton::new(id, label, Rect::new(0.0, 0.0, width, height))
        .material(overlay_material)
        .chrome(UiTheme::new(scheme).glass_chrome(GlassRole::FloatingControl))
        .into_element::<Message, Theme, Renderer>(message)
}

fn palette(theme: &Theme) -> UiPalette {
    UiTheme::from_iced(theme).palette()
}

fn sidebar_style(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        text_color: Some(palette.text_primary),
        background: None,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

fn content_style(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        text_color: Some(palette.text_primary),
        background: Some(Background::Color(palette.content_background)),
        ..container::Style::default()
    }
}

fn list_group_style(theme: &Theme) -> container::Style {
    let palette = palette(theme);
    container::Style {
        text_color: Some(palette.text_primary),
        background: Some(Background::Color(palette.group_background)),
        border: Border::default().rounded(10.0).width(1.0).color(palette.group_border),
        ..container::Style::default()
    }
}

fn progress_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette(theme).group_background)),
        ..container::Style::default()
    }
}

fn sidebar_button_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let palette = palette(theme);
    let background = match (selected, status) {
        (true, button::Status::Pressed) => palette.accent.scale_alpha(0.28),
        (true, _) => palette.selection,
        (false, button::Status::Hovered | button::Status::Pressed) => palette.hover,
        (false, _) => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.text_primary,
        border: Border::default().rounded(7.0),
        ..button::Style::default()
    }
}

fn list_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = match status {
        button::Status::Hovered => palette.hover,
        button::Status::Pressed => palette.selection,
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.accent,
        border: Border::default().rounded(8.0),
        ..button::Style::default()
    }
}

fn toolbar_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = palette(theme);
    let background = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        palette.hover
    } else {
        Color::TRANSPARENT
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.text_primary,
        border: Border::default().rounded(10.0),
        ..button::Style::default()
    }
}

fn segmented_button_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let palette = palette(theme);
    let background = if selected {
        palette.selection
    } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        palette.hover
    } else {
        Color::TRANSPARENT
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.text_primary,
        border: Border::default().rounded(7.0).width(1.0).color(Color::from_rgba(
            palette.group_border.r,
            palette.group_border.g,
            palette.group_border.b,
            if selected { 0.30 } else { palette.group_border.a },
        )),
        ..button::Style::default()
    }
}

fn search_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = palette(theme);
    let border = if matches!(status, text_input::Status::Focused { .. }) {
        palette.accent.scale_alpha(0.55)
    } else {
        palette.group_border
    };
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default().rounded(8.0).width(1.0).color(border),
        icon: palette.text_secondary,
        placeholder: palette.text_secondary,
        value: palette.text_primary,
        selection: palette.selection,
    }
}

fn rule_style(theme: &Theme) -> rule::Style {
    rule::Style {
        color: palette(theme).separator,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

fn split_rule_style(theme: &Theme) -> rule::Style {
    rule_style(theme)
}

fn main() -> iced::Result {
    iced::application::<State, Message, Theme, Renderer>(boot, update, view)
        .title("System Settings")
        .theme(app_theme)
        .subscription(subscription)
        .window_size(iced::Size::new(1320.0, 760.0))
        .run()
}
