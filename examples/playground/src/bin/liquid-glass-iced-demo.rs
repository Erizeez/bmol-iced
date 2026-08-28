use iced::{
    Element, Task, Theme,
    widget::{column, text},
};
use liquid_glass::{GlassContainer, GlassId, GlassMaterial, GlassShape, Rect};

#[derive(Default)]
struct State;

#[derive(Debug, Clone, Copy)]
enum Message {}

fn update(_state: &mut State, _message: Message) -> Task<Message> {
    Task::none()
}

fn view(_state: &State) -> Element<'_, Message> {
    let panel = GlassContainer::new(GlassId(1), Rect::new(0.0, 0.0, 560.0, 360.0))
        .shape(GlassShape::Superellipse { exponent: 4.5 })
        .material(GlassMaterial::regular())
        .padding(24.0)
        .into_element::<Message, Theme, iced::Renderer>();

    column![
        text("Liquid Glass / Iced").size(28),
        text("GlassContainer is an iced::advanced::Widget").size(16),
        panel,
    ]
    .spacing(16)
    .padding(32)
    .into()
}

fn main() -> iced::Result {
    iced::application(State::default, update, view).theme(Theme::Dark).run()
}
