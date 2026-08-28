use iced::{
    Element, Task, Theme,
    widget::{column, text},
};
use liquid_glass::{GlassButton, GlassContainer, GlassId, GlassMaterial, GlassShape, Rect};

#[derive(Default)]
struct State {
    presses: u32,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    ButtonPressed,
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::ButtonPressed => state.presses += 1,
    }
    Task::none()
}

fn view(state: &State) -> Element<'_, Message> {
    let panel = GlassContainer::new(GlassId(1), Rect::new(0.0, 0.0, 560.0, 360.0))
        .shape(GlassShape::Superellipse { exponent: 4.5 })
        .material(GlassMaterial::regular())
        .padding(24.0)
        .into_element::<Message, Theme, iced::Renderer>();
    let button = GlassButton::new(GlassId(2), "Connect", Rect::new(0.0, 0.0, 220.0, 56.0))
        .into_element::<Message, Theme, iced::Renderer>(Message::ButtonPressed);

    column![
        text("Liquid Glass / Iced").size(28),
        text("GlassContainer is an iced::advanced::Widget").size(16),
        panel,
        button,
        text(format!("Connect pressed {} times", state.presses)),
    ]
    .spacing(16)
    .padding(32)
    .into()
}

fn main() -> iced::Result {
    iced::application(State::default, update, view).theme(Theme::Dark).run()
}
