#[path = "../iced_backend.rs"]
mod iced_backend;

use iced::{
    Background, Border, Color, Element, Length, Shadow, Task, Theme,
    widget::{
        button, column, container, progress_bar, row, rule, scrollable, slider, space, stack, text,
        text_input, toggler,
    },
};
use iced_backend::Renderer;
use liquid_glass::{GlassButton, GlassContainer, GlassId, GlassMaterial, GlassShape, Rect};

struct State {
    active_section: Section,
    appearance: Appearance,
    search: String,
    auto_updates: bool,
    notifications: bool,
    reduce_motion: bool,
    volume: f32,
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
}

type AppElement<'a> = Element<'a, Message, Theme, Renderer>;

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::SectionSelected(section) => state.active_section = section,
        Message::AppearanceSelected(appearance) => state.appearance = appearance,
        Message::SearchChanged(search) => state.search = search,
        Message::AutoUpdatesChanged(enabled) => state.auto_updates = enabled,
        Message::NotificationsChanged(enabled) => state.notifications = enabled,
        Message::ReduceMotionChanged(enabled) => state.reduce_motion = enabled,
        Message::VolumeChanged(volume) => state.volume = volume,
    }
    Task::none()
}

#[allow(clippy::too_many_lines)]
fn view(state: &State) -> AppElement<'_> {
    let sidebar: AppElement<'static> = container(column![
        text("SETTINGS").size(18),
        text("System preferences").size(12),
        space().height(Length::Fixed(16.0)),
        sidebar_button("General", Section::General, state.active_section),
        sidebar_button("Appearance", Section::Appearance, state.active_section),
        sidebar_button("Notifications", Section::Notifications, state.active_section),
        sidebar_button("Privacy & Security", Section::Privacy, state.active_section),
        space().height(Length::Fill),
        text("Liquid Glass UI").size(12),
        text("Iced compositor preview").size(11),
    ])
    .width(Length::Fixed(220.0))
    .height(Length::Fill)
    .padding(18)
    .style(sidebar_style)
    .into();

    let toolbar = glass_surface_with_padding(
        GlassId(10),
        Rect::new(0.0, 0.0, 1034.0, 72.0),
        12.0,
        row![
            compositor_button(
                GlassId(12),
                "‹",
                Message::SectionSelected(state.active_section),
                36.0,
                40.0
            ),
            compositor_button(
                GlassId(13),
                "›",
                Message::SectionSelected(state.active_section),
                36.0,
                40.0
            ),
            column![
                text(section_title(state.active_section)).size(24),
                text("A calm place to tune your system").size(12)
            ]
            .spacing(2)
            .width(Length::Fill),
            glass_search(&state.search),
            button(text("•••").size(16))
                .on_press(Message::SectionSelected(state.active_section))
                .width(Length::Fixed(46.0))
                .style(toolbar_button_style),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    );

    let content = scrollable(settings_page(state)).height(Length::Fill).width(Length::Fill);

    let main = column![toolbar, content].spacing(16).width(Length::Fill).height(Length::Fill);

    container(row![sidebar, main].spacing(18).padding(24))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn sidebar_button(label: &'static str, section: Section, active: Section) -> AppElement<'static> {
    let prefix = if section == active { "●" } else { "○" };
    button(text(format!("{prefix}  {label}")))
        .on_press(Message::SectionSelected(section))
        .width(Length::Fill)
        .style(sidebar_button_style)
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
    column![text(title).size(20), text(subtitle).size(12)]
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
            column![text(label).size(14), text(detail.into()).size(11)]
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
        .style(move |_theme, status| segmented_button_style(status, appearance == selected))
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
        .style(move |_theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border::default().rounded(7.0).width(1.0).color(Color::WHITE),
            ..container::Style::default()
        })
        .into()
}

fn glass_search(value: &str) -> AppElement<'_> {
    let field = text_input("Search", value)
        .on_input(Message::SearchChanged)
        .width(Length::Fill)
        .padding([7, 10])
        .style(search_style);
    glass_surface_with_padding(GlassId(11), Rect::new(0.0, 0.0, 250.0, 40.0), 2.0, field)
}

fn glass_surface_with_padding<'a>(
    id: GlassId,
    bounds: Rect,
    padding: f32,
    content: impl Into<AppElement<'a>>,
) -> AppElement<'a> {
    let mut overlay_material = GlassMaterial::clear();
    overlay_material.tint = liquid_glass::Color::transparent();
    let background = GlassContainer::new(id, bounds)
        .shape(GlassShape::Superellipse { exponent: 4.5 })
        .material(overlay_material)
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
) -> AppElement<'static> {
    let mut overlay_material = GlassMaterial::clear();
    overlay_material.tint = liquid_glass::Color::transparent();
    GlassButton::new(id, label, Rect::new(0.0, 0.0, width, height))
        .material(overlay_material)
        .into_element::<Message, Theme, Renderer>(message)
}

fn sidebar_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(Color::from_rgb(0.92, 0.94, 0.98)),
        background: Some(Background::Color(Color::from_rgba(0.08, 0.10, 0.15, 0.94))),
        border: Border::default()
            .rounded(16.0)
            .width(1.0)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.08)),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.22),
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 22.0,
        },
        snap: true,
    }
}

fn list_group_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(Color::from_rgb(0.92, 0.94, 0.98)),
        background: Some(Background::Color(Color::from_rgba(0.10, 0.12, 0.18, 0.96))),
        border: Border::default()
            .rounded(14.0)
            .width(1.0)
            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.08)),
        ..container::Style::default()
    }
}

fn progress_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.10, 0.12, 0.18, 0.70))),
        ..container::Style::default()
    }
}

fn sidebar_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Color::from_rgba(0.24, 0.35, 0.58, 0.34),
        button::Status::Pressed => Color::from_rgba(0.30, 0.45, 0.75, 0.48),
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::from_rgb(0.88, 0.91, 0.97),
        border: Border::default().rounded(8.0),
        ..button::Style::default()
    }
}

fn list_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.07),
        button::Status::Pressed => Color::from_rgba(0.30, 0.45, 0.75, 0.24),
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::from_rgb(0.58, 0.72, 1.0),
        border: Border::default().rounded(8.0),
        ..button::Style::default()
    }
}

fn toolbar_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Color::from_rgba(1.0, 1.0, 1.0, 0.10)
    } else {
        Color::TRANSPARENT
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border::default().rounded(10.0),
        ..button::Style::default()
    }
}

fn segmented_button_style(status: button::Status, selected: bool) -> button::Style {
    let alpha = if selected {
        0.30
    } else if matches!(status, button::Status::Hovered) {
        0.12
    } else {
        0.04
    };
    button::Style {
        background: Some(Background::Color(Color::from_rgba(0.30, 0.50, 1.0, alpha))),
        text_color: Color::WHITE,
        border: Border::default().rounded(7.0).width(1.0).color(Color::from_rgba(
            1.0,
            1.0,
            1.0,
            if selected { 0.20 } else { 0.06 },
        )),
        ..button::Style::default()
    }
}

fn search_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let border_alpha =
        if matches!(status, text_input::Status::Focused { .. }) { 0.40 } else { 0.14 };
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default().rounded(9.0).width(1.0).color(Color::from_rgba(
            1.0,
            1.0,
            1.0,
            border_alpha,
        )),
        icon: Color::from_rgb(0.65, 0.70, 0.80),
        placeholder: Color::from_rgb(0.60, 0.65, 0.75),
        value: Color::WHITE,
        selection: Color::from_rgb(0.28, 0.52, 1.0),
    }
}

fn rule_style(_theme: &Theme) -> rule::Style {
    rule::Style {
        color: Color::from_rgba(1.0, 1.0, 1.0, 0.07),
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

fn main() -> iced::Result {
    iced::application::<State, Message, Theme, Renderer>(State::default, update, view)
        .theme(Theme::Dark)
        .window_size(iced::Size::new(1320.0, 760.0))
        .run()
}
