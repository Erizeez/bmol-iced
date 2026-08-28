#[path = "../iced_backend.rs"]
mod iced_backend;

use iced::{
    Element, Length, Task, Theme,
    widget::{
        button, checkbox, column, container, progress_bar, row, scrollable, slider, space, stack,
        text, text_input, toggler,
    },
};
use iced_backend::Renderer;
use liquid_glass::{GlassButton, GlassContainer, GlassId, GlassMaterial, GlassShape, Rect};

struct State {
    active_section: Section,
    blur: f32,
    glow_enabled: bool,
    notifications_enabled: bool,
    search: String,
    applied: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_section: Section::Overview,
            blur: 20.0,
            glow_enabled: true,
            notifications_enabled: true,
            search: String::new(),
            applied: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum Section {
    #[default]
    Overview,
    Materials,
    Components,
}

#[derive(Debug, Clone)]
enum Message {
    SectionSelected(Section),
    BlurChanged(f32),
    GlowToggled(bool),
    NotificationsToggled(bool),
    SearchChanged(String),
    ApplyPressed,
    RefreshPressed,
}

type AppElement<'a> = Element<'a, Message, Theme, Renderer>;

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::SectionSelected(section) => state.active_section = section,
        Message::BlurChanged(blur) => state.blur = blur,
        Message::GlowToggled(enabled) => state.glow_enabled = enabled,
        Message::NotificationsToggled(enabled) => state.notifications_enabled = enabled,
        Message::SearchChanged(search) => state.search = search,
        Message::ApplyPressed => state.applied += 1,
        Message::RefreshPressed => state.applied = 0,
    }
    Task::none()
}

#[allow(clippy::too_many_lines)]
fn view(state: &State) -> AppElement<'_> {
    let sidebar = glass_surface(
        GlassId(10),
        Rect::new(0.0, 0.0, 220.0, 620.0),
        column![
            text("LIQUID GLASS").size(18),
            text("Studio dashboard").size(13),
            text(""),
            sidebar_button("Overview", Section::Overview, state.active_section),
            sidebar_button("Materials", Section::Materials, state.active_section),
            sidebar_button("Components", Section::Components, state.active_section),
            space().height(Length::Fill),
            text("Renderer").size(12),
            text("wgpu / SDF / blur").size(13),
        ]
        .spacing(12),
    );

    let toolbar = row![
        column![
            text("Overview").size(26),
            text("Compose, tune, and inspect glass surfaces").size(13)
        ]
        .spacing(4)
        .width(Length::Fill),
        text_input("Search components", &state.search)
            .on_input(Message::SearchChanged)
            .width(Length::Fixed(220.0)),
        button(text("Refresh")).on_press(Message::RefreshPressed),
    ]
    .spacing(14)
    .align_y(iced::Alignment::Center);

    let stat_cards = row![
        stat_card(GlassId(20), "Backdrop regions", "04", "merged this frame"),
        stat_card(GlassId(21), "Blur target", "1/2", "resolution pipeline"),
        stat_card(GlassId(22), "Frame budget", "2.8 ms", "GPU compositor"),
    ]
    .spacing(14);

    let controls = glass_surface(
        GlassId(30),
        Rect::new(0.0, 0.0, 470.0, 250.0),
        column![
            row![
                text("Material controls").size(18),
                space().width(Length::Fill),
                text(format!("{:.0} px", state.blur)).size(13)
            ]
            .align_y(iced::Alignment::Center),
            text("Live parameters are routed through the scene material").size(12),
            text("Blur radius").size(13),
            slider(0.0..=48.0, state.blur, Message::BlurChanged),
            progress_bar(0.0..=48.0, state.blur),
            toggler(state.glow_enabled).label("Fresnel glow").on_toggle(Message::GlowToggled),
            checkbox(state.notifications_enabled)
                .label("Enable surface notifications")
                .on_toggle(Message::NotificationsToggled),
        ]
        .spacing(12),
    );

    let activity = glass_surface(
        GlassId(31),
        Rect::new(0.0, 0.0, 470.0, 250.0),
        column![
            row![
                text("Recent activity").size(18),
                space().width(Length::Fill),
                text("LIVE").size(12)
            ]
            .align_y(iced::Alignment::Center),
            scrollable(
                column![
                    activity_row("GlassButton", "pressed state", "now"),
                    activity_row("GlassScene", "2 nodes composed", "12s"),
                    activity_row("Backdrop", "vertical blur rebuilt", "28s"),
                    activity_row("Iced layout", "bounds bridged", "41s"),
                ]
                .spacing(10),
            )
            .height(Length::Fill),
        ]
        .spacing(14),
    );

    let action_bar = glass_surface(
        GlassId(32),
        Rect::new(0.0, 0.0, 954.0, 82.0),
        row![
            column![
                text("Scene ready").size(15),
                text(format!("Applied changes: {}", state.applied)).size(12)
            ]
            .width(Length::Fill),
            GlassButton::new(GlassId(40), "Apply material", Rect::new(0.0, 0.0, 180.0, 48.0),)
                .into_element::<Message, Theme, Renderer>(Message::ApplyPressed),
            GlassButton::new(GlassId(41), "Preview merge", Rect::new(0.0, 0.0, 180.0, 48.0),)
                .into_element::<Message, Theme, Renderer>(Message::RefreshPressed),
        ]
        .spacing(14)
        .align_y(iced::Alignment::Center),
    );

    let dashboard = row![
        sidebar,
        column![toolbar, stat_cards, row![controls, activity].spacing(14), action_bar].spacing(16),
    ]
    .spacing(18);

    container(container(dashboard).padding(24)).width(Length::Fill).height(Length::Fill).into()
}

fn sidebar_button(label: &'static str, section: Section, active: Section) -> AppElement<'static> {
    let prefix = if section as u8 == active as u8 { "●" } else { "○" };
    button(text(format!("{prefix}  {label}")))
        .on_press(Message::SectionSelected(section))
        .width(Length::Fill)
        .into()
}

fn stat_card(
    id: GlassId,
    title: &'static str,
    value: &'static str,
    detail: &'static str,
) -> AppElement<'static> {
    glass_surface(
        id,
        Rect::new(0.0, 0.0, 300.0, 112.0),
        column![text(title).size(12), text(value).size(28), text(detail).size(12)].spacing(6),
    )
}

fn activity_row(
    name: &'static str,
    detail: &'static str,
    age: &'static str,
) -> AppElement<'static> {
    row![
        column![text(name).size(13), text(detail).size(11)].spacing(2).width(Length::Fill),
        text(age).size(11)
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

fn glass_surface<'a>(
    id: GlassId,
    bounds: Rect,
    content: impl Into<AppElement<'a>>,
) -> AppElement<'a> {
    let background = GlassContainer::new(id, bounds)
        .shape(GlassShape::Superellipse { exponent: 4.5 })
        .material(GlassMaterial::regular())
        .into_element::<Message, Theme, Renderer>();
    let foreground: AppElement<'a> = container(content)
        .width(Length::Fixed(bounds.width))
        .height(Length::Fixed(bounds.height))
        .padding(18)
        .into();
    stack![background, foreground].into()
}

fn main() -> iced::Result {
    iced::application::<State, Message, Theme, Renderer>(State::default, update, view)
        .theme(Theme::Dark)
        .window_size(iced::Size::new(1320.0, 760.0))
        .run()
}
