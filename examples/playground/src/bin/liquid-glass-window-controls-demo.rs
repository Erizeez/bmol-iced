//! Standalone Iced laboratory for custom macOS-style window controls.
//!
//! The demo deliberately hides the native window decorations and renders the
//! traffic lights through the same Liquid Glass compositor used by the
//! settings demo. It shows the measured 14 pt controls, a second 1:1 sample,
//! and enlarged active/inactive samples for inspecting hover and press light.
//! The system glyphs are revealed on hover; inactive windows use the native
//! pale gray/white treatment until hovered, then transition to the focused
//! treatment. Unavailable controls stay dot-only.

#[path = "../iced_backend.rs"]
mod iced_backend;

use std::time::Duration;

use iced::{
    Background, Color, Element, Length, Padding, Subscription, Task, Theme,
    widget::{button, column, container, mouse_area, row, scrollable, space, stack, text},
};
use iced_backend::{
    DemoSurface, INTERACTION_ENTER_ANIMATION_TIME_CONSTANT,
    INTERACTION_EXIT_ANIMATION_TIME_CONSTANT, Renderer, WINDOW_CONTROL_DISABLED_IDS,
    WINDOW_CONTROL_DISABLED_X, WINDOW_CONTROL_DISABLED_Y, WINDOW_CONTROL_GAP,
    WINDOW_CONTROL_INACTIVE_IDS, WINDOW_CONTROL_INACTIVE_X, WINDOW_CONTROL_INACTIVE_Y,
    WINDOW_CONTROL_LARGE_GAP, WINDOW_CONTROL_LARGE_IDS, WINDOW_CONTROL_LARGE_SIZE,
    WINDOW_CONTROL_LARGE_X, WINDOW_CONTROL_LARGE_Y, WINDOW_CONTROL_NATIVE_IDS,
    WINDOW_CONTROL_NATIVE_SIZE, WINDOW_CONTROL_NATIVE_X, WINDOW_CONTROL_NATIVE_Y,
    WINDOW_CONTROL_REFERENCE_IDS, WINDOW_CONTROL_REFERENCE_X, WINDOW_CONTROL_REFERENCE_Y,
    WindowControlTuning,
};
use liquid_glass::{
    GlassButton, GlassId, GlassMaterial, GlassRole, GlassShape, IcedWindowController,
    IcedWindowPolicy, Rect, UiColorScheme, UiCornerStyle, UiIcon, UiTheme, WindowCommand,
    WindowDragArea, WindowExpandBehavior,
    ui::{GlassChrome, components, font},
};
use spring_rs::{Spring, SpringMotion};

// The control deliberately has a visible, inspectable click pulse. The
// resting target remains above 1.0, while the press target gives the spring
// enough travel to show both its growth and release phases at 14 pt.
const PRESS_SCALE_OVERSHOOT: f32 = 1.18;
const PRESS_SCALE_SETTLED: f32 = 1.06;
const PRESS_SCALE_SPRING_DURATION: f32 = 0.24;
const PRESS_SCALE_SPRING_EXTRA_BOUNCE: f32 = 0.40;

type AppElement<'a> = Element<'a, Message, Theme, Renderer>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ControlAction {
    Close,
    Minimize,
    Expand,
}

impl ControlAction {
    fn name(self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::Minimize => "Minimize",
            Self::Expand => "Expand",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ControlGroup {
    Native,
    Reference,
    Active,
    Inactive,
    Disabled,
}

impl ControlGroup {
    const fn index(self) -> usize {
        match self {
            Self::Native => 0,
            Self::Reference => 1,
            Self::Active => 2,
            Self::Inactive => 3,
            Self::Disabled => 4,
        }
    }
}

const ALL_WINDOW_CONTROL_IDS: [GlassId; 15] = [
    WINDOW_CONTROL_NATIVE_IDS[0],
    WINDOW_CONTROL_NATIVE_IDS[1],
    WINDOW_CONTROL_NATIVE_IDS[2],
    WINDOW_CONTROL_REFERENCE_IDS[0],
    WINDOW_CONTROL_REFERENCE_IDS[1],
    WINDOW_CONTROL_REFERENCE_IDS[2],
    WINDOW_CONTROL_LARGE_IDS[0],
    WINDOW_CONTROL_LARGE_IDS[1],
    WINDOW_CONTROL_LARGE_IDS[2],
    WINDOW_CONTROL_INACTIVE_IDS[0],
    WINDOW_CONTROL_INACTIVE_IDS[1],
    WINDOW_CONTROL_INACTIVE_IDS[2],
    WINDOW_CONTROL_DISABLED_IDS[0],
    WINDOW_CONTROL_DISABLED_IDS[1],
    WINDOW_CONTROL_DISABLED_IDS[2],
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum TuningParameter {
    BlurRadius,
    InternalScattering,
    SideEdgeDarkness,
    SideEdgeWidth,
    Opacity,
    SubstrateCoverage,
    LowerSubstrateCoverage,
    LowerTintCoverage,
    AngularLight,
    LightAngle,
    LightSoftness,
    BodyThickness,
    EdgeSideBias,
    EdgeSideAngle,
    RefractionStrength,
    FresnelStrength,
}

#[derive(Debug, Clone)]
enum Message {
    ControlPressed { id: GlassId, action: ControlAction, execute: bool },
    ControlPressStarted { id: GlassId },
    ControlPressVisualCancelled { id: GlassId },
    ControlPressEnded { id: GlassId },
    ControlGroupHover { group: ControlGroup, hovered: bool },
    WindowReady(Option<iced::window::Id>),
    WindowEvent((iced::window::Id, iced::window::Event)),
    BeginWindowDrag,
    ResizeWindow(iced::window::Direction),
    AnimationTick,
    TuningChanged { parameter: TuningParameter, value: f32 },
    CopyConfiguration,
    ToggleScheme,
    SystemThemeChanged(iced::theme::Mode),
}

struct State {
    window: IcedWindowController,
    window_policy: IcedWindowPolicy,
    scheme: UiColorScheme,
    last_action: String,
    hover_targets: [f32; 5],
    hover_progress: [f32; 5],
    press_targets: [f32; 15],
    press_progress: [f32; 15],
    press_springs: [SpringMotion; 15],
    tuning: WindowControlTuning,
}

impl Default for State {
    fn default() -> Self {
        Self {
            window: IcedWindowController::new(),
            window_policy: demo_window_policy(),
            scheme: UiColorScheme::Light,
            last_action: "Move over a control to reveal its system glyph".into(),
            hover_targets: [0.0; 5],
            hover_progress: [0.0; 5],
            press_targets: [0.0; 15],
            press_progress: [0.0; 15],
            press_springs: std::array::from_fn(|_| {
                SpringMotion::new(
                    1.0,
                    1.0,
                    Spring::bouncy_custom(
                        PRESS_SCALE_SPRING_DURATION,
                        PRESS_SCALE_SPRING_EXTRA_BOUNCE,
                    ),
                )
            }),
            tuning: WindowControlTuning::default(),
        }
    }
}

fn boot() -> (State, Task<Message>) {
    let state = State::default();
    iced_backend::set_surface(DemoSurface::WindowControls);
    iced_backend::set_color_scheme(state.scheme);
    iced_backend::set_accessibility(liquid_glass::GlassAccessibility::none());
    iced_backend::set_window_control_tuning(state.tuning);
    for index in 0..state.hover_progress.len() {
        iced_backend::set_window_control_group_progress(index, 0.0);
    }
    for id in ALL_WINDOW_CONTROL_IDS {
        iced_backend::set_window_control_press_progress(id, 0.0);
        iced_backend::set_window_control_scale(id, 1.0);
    }
    (
        state,
        Task::batch([
            iced::system::theme().map(Message::SystemThemeChanged),
            IcedWindowController::latest().map(Message::WindowReady),
        ]),
    )
}

const fn demo_window_policy() -> IcedWindowPolicy {
    IcedWindowPolicy::liquid_glass().manual_close()
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::ControlPressed { id, action, execute } => {
            if execute {
                finish_control_press(state, id);
            }
            state.last_action = if execute {
                format!("{} control pressed — sent to the Iced window", action.name())
            } else {
                format!("{} control pressed — inspection sample", action.name())
            };
            if execute {
                let command = match action {
                    ControlAction::Close => WindowCommand::Close,
                    ControlAction::Minimize => WindowCommand::Minimize,
                    ControlAction::Expand => state.window_policy.expand_command(),
                };
                return state.window.task(command);
            }
        }
        Message::ControlPressStarted { id } => {
            if let Some(index) = window_control_slot_index(id) {
                state.press_targets[index] = 1.0;
                state.press_springs[index].retarget(PRESS_SCALE_OVERSHOOT);
            }
        }
        Message::ControlPressVisualCancelled { id } => {
            finish_control_press_visual(state, id);
        }
        Message::ControlPressEnded { id } => {
            finish_control_press(state, id);
        }
        Message::WindowReady(id) => {
            if let Some(id) = id {
                state.window.attach(id);
                let options = state.window_policy.native_options();
                return iced::window::run(id, move |w| {
                    if let Ok(handle) = w.window_handle() {
                        let _ = liquid_glass::setup_native_window(handle.as_raw(), options);
                    }
                })
                .discard();
            }
        }
        Message::ResizeWindow(direction) => {
            if let Some(id) = state.window.id() {
                return iced::window::drag_resize(id, direction);
            }
        }
        Message::WindowEvent((id, event)) => {
            let close_requested = matches!(event, iced::window::Event::CloseRequested);
            state.window.observe(id, &event);
            if close_requested {
                // This demo explicitly uses `manual_close` so a real app can
                // insert confirmation or save logic here. It has nothing to
                // save, so it closes immediately.
                return state.window.task(WindowCommand::Close);
            }
        }
        Message::BeginWindowDrag => {
            return state.window.task(WindowCommand::BeginDrag);
        }
        Message::ControlGroupHover { group, hovered } => {
            iced_backend::set_window_control_group_hover(group.index(), hovered);
            state.hover_targets[group.index()] = if hovered { 1.0 } else { 0.0 };
        }
        Message::AnimationTick => {
            // Keep the Iced glyph layer and the compositor's material layer
            for (progress, target) in state.hover_progress.iter_mut().zip(state.hover_targets) {
                let time_constant = if target >= *progress {
                    iced_backend::INTERACTION_ENTER_ANIMATION_TIME_CONSTANT
                } else {
                    iced_backend::INTERACTION_EXIT_ANIMATION_TIME_CONSTANT
                };
                let step = 1.0 - (-0.016_f32 / time_constant).exp();
                let value = *progress + (target - *progress) * step;
                *progress = if (value - target).abs() < 0.001 { target } else { value };
            }
            for (progress, target) in state.press_progress.iter_mut().zip(state.press_targets) {
                let time_constant = if target >= *progress {
                    INTERACTION_ENTER_ANIMATION_TIME_CONSTANT
                } else {
                    INTERACTION_EXIT_ANIMATION_TIME_CONSTANT
                };
                let step = 1.0 - (-0.016_f32 / time_constant).exp();
                let value = *progress + (target - *progress) * step;
                *progress = if (value - target).abs() < 0.001 { target } else { value };
            }
            for (index, progress) in state.hover_progress.iter().copied().enumerate() {
                iced_backend::set_window_control_group_progress(index, progress);
            }
            for (id, progress) in
                ALL_WINDOW_CONTROL_IDS.into_iter().zip(state.press_progress.iter().copied())
            {
                iced_backend::set_window_control_press_progress(id, progress);
            }
            for (id, spring) in
                ALL_WINDOW_CONTROL_IDS.into_iter().zip(state.press_springs.iter_mut())
            {
                spring.step(1.0 / 60.0);
                iced_backend::set_window_control_scale(id, spring.value());
            }
        }
        Message::TuningChanged { parameter, value } => {
            match parameter {
                TuningParameter::BlurRadius => state.tuning.blur_radius = value,
                TuningParameter::InternalScattering => {
                    state.tuning.internal_scattering = value;
                }
                TuningParameter::SideEdgeDarkness => state.tuning.side_edge_darkness = value,
                TuningParameter::SideEdgeWidth => state.tuning.side_edge_width = value,
                TuningParameter::Opacity => state.tuning.opacity = value,
                TuningParameter::SubstrateCoverage => {
                    state.tuning.substrate_coverage = value;
                }
                TuningParameter::LowerSubstrateCoverage => {
                    state.tuning.lower_substrate_coverage = value;
                }
                TuningParameter::LowerTintCoverage => {
                    state.tuning.lower_tint_coverage = value;
                }
                TuningParameter::AngularLight => state.tuning.angular_light = value,
                TuningParameter::LightAngle => state.tuning.light_angle = value,
                TuningParameter::LightSoftness => state.tuning.light_softness = value,
                TuningParameter::BodyThickness => state.tuning.body_thickness = value,
                TuningParameter::EdgeSideBias => state.tuning.edge_side_bias = value,
                TuningParameter::EdgeSideAngle => state.tuning.edge_side_angle = value,
                TuningParameter::RefractionStrength => {
                    state.tuning.refraction_strength = value;
                }
                TuningParameter::FresnelStrength => state.tuning.fresnel_strength = value,
            }
            iced_backend::set_window_control_tuning(state.tuning);
        }
        Message::CopyConfiguration => {
            state.last_action = "Configuration copied to clipboard".into();
            return iced::clipboard::write(configuration_text(state));
        }
        Message::ToggleScheme => {
            state.scheme = match state.scheme {
                UiColorScheme::Light => UiColorScheme::Dark,
                UiColorScheme::Dark => UiColorScheme::Light,
            };
            state.tuning = WindowControlTuning::for_scheme(state.scheme);
            iced_backend::set_color_scheme(state.scheme);
            iced_backend::set_window_control_tuning(state.tuning);
        }
        Message::SystemThemeChanged(mode) => {
            state.scheme = UiColorScheme::from_mode(mode);
            state.tuning = WindowControlTuning::for_scheme(state.scheme);
            iced_backend::set_color_scheme(state.scheme);
            iced_backend::set_window_control_tuning(state.tuning);
        }
    }
    Task::none()
}

fn subscription(state: &State) -> Subscription<Message> {
    let theme_changes = iced::system::theme_changes().map(Message::SystemThemeChanged);
    let window_events = IcedWindowController::events().map(Message::WindowEvent);
    let animation_active = state
        .hover_targets
        .iter()
        .zip(state.hover_progress.iter())
        .any(|(target, progress)| (target - progress).abs() > 0.001)
        || state
            .press_targets
            .iter()
            .zip(state.press_progress.iter())
            .any(|(target, progress)| (target - progress).abs() > 0.001);
    let spring_active = state.press_springs.iter().any(|spring| !spring.is_settled(0.001, 0.01));
    if animation_active || spring_active {
        Subscription::batch([
            theme_changes,
            window_events,
            iced::time::every(Duration::from_millis(16)).map(|_| Message::AnimationTick),
        ])
    } else {
        Subscription::batch([theme_changes, window_events])
    }
}

fn app_theme(state: &State) -> Theme {
    UiTheme::new(state.scheme).iced_theme()
}

fn view(state: &State) -> AppElement<'_> {
    let info = container(
        column![
            row![
                text("Custom Window Controls")
                    .size(28.0)
                    .font(font::ui_font(iced::font::Weight::Semibold)),
                space().width(Length::Fill),
                button(text("Copy configuration"))
                    .on_press(Message::CopyConfiguration)
                    .padding([7, 12]),
                button(text(match state.scheme {
                    UiColorScheme::Light => "Switch to dark mode",
                    UiColorScheme::Dark => "Switch to light mode",
                }))
                .on_press(Message::ToggleScheme)
                .padding([7, 12]),
            ]
            .align_y(iced::Alignment::Center),
            text("Live SDF glass · macOS-extracted vector glyphs · hover and press")
                .size(font::size::BODY)
                .style(components::secondary_text),
            text(&state.last_action).size(font::size::CAPTION).style(components::tertiary_text),
        ]
        .spacing(8),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(Padding { top: 86.0, right: 48.0, bottom: 0.0, left: 48.0 })
    .style(transparent_surface);

    let native_controls = positioned_control_group(
        control_group(
            WINDOW_CONTROL_NATIVE_IDS,
            WINDOW_CONTROL_NATIVE_SIZE,
            WINDOW_CONTROL_GAP,
            state.scheme,
            true,
            false,
            true,
            ControlGroup::Native,
            state.hover_progress[ControlGroup::Native.index()],
            state.window_policy.expand_behavior,
            control_group_scales(&state.press_springs, ControlGroup::Native),
        ),
        WINDOW_CONTROL_NATIVE_X,
        WINDOW_CONTROL_NATIVE_Y,
        WINDOW_CONTROL_NATIVE_SIZE,
    );
    let tuning_panel = positioned(tuning_panel(state.tuning), 620.0, 82.0);
    let reference_label = positioned(
        sample_label("1:1 reference · 14 pt visual diameter"),
        WINDOW_CONTROL_REFERENCE_X,
        WINDOW_CONTROL_REFERENCE_Y - 30.0,
    );
    let reference_controls = positioned_control_group(
        control_group(
            WINDOW_CONTROL_REFERENCE_IDS,
            WINDOW_CONTROL_NATIVE_SIZE,
            WINDOW_CONTROL_GAP,
            state.scheme,
            true,
            false,
            false,
            ControlGroup::Reference,
            state.hover_progress[ControlGroup::Reference.index()],
            WindowExpandBehavior::Fullscreen,
            control_group_scales(&state.press_springs, ControlGroup::Reference),
        ),
        WINDOW_CONTROL_REFERENCE_X,
        WINDOW_CONTROL_REFERENCE_Y,
        WINDOW_CONTROL_NATIVE_SIZE,
    );
    let large_label = positioned(
        sample_label("Maximize behavior · plus glyph · 64 pt"),
        WINDOW_CONTROL_LARGE_X,
        WINDOW_CONTROL_LARGE_Y - 30.0,
    );
    let large_controls = positioned_control_group(
        control_group(
            WINDOW_CONTROL_LARGE_IDS,
            WINDOW_CONTROL_LARGE_SIZE,
            WINDOW_CONTROL_LARGE_GAP,
            state.scheme,
            true,
            false,
            false,
            ControlGroup::Active,
            state.hover_progress[ControlGroup::Active.index()],
            WindowExpandBehavior::Maximize,
            control_group_scales(&state.press_springs, ControlGroup::Active),
        ),
        WINDOW_CONTROL_LARGE_X,
        WINDOW_CONTROL_LARGE_Y,
        WINDOW_CONTROL_LARGE_SIZE,
    );
    let inactive_label = positioned(
        sample_label("Inactive window · all controls pale gray"),
        WINDOW_CONTROL_INACTIVE_X,
        WINDOW_CONTROL_INACTIVE_Y - 30.0,
    );
    let inactive_controls = positioned_control_group(
        control_group(
            WINDOW_CONTROL_INACTIVE_IDS,
            WINDOW_CONTROL_LARGE_SIZE,
            WINDOW_CONTROL_LARGE_GAP,
            state.scheme,
            true,
            false,
            false,
            ControlGroup::Inactive,
            state.hover_progress[ControlGroup::Inactive.index()],
            WindowExpandBehavior::Fullscreen,
            control_group_scales(&state.press_springs, ControlGroup::Inactive),
        ),
        WINDOW_CONTROL_INACTIVE_X,
        WINDOW_CONTROL_INACTIVE_Y,
        WINDOW_CONTROL_LARGE_SIZE,
    );
    let disabled_label = positioned(
        sample_label("Running · close status dot"),
        WINDOW_CONTROL_DISABLED_X,
        WINDOW_CONTROL_DISABLED_Y - 30.0,
    );
    let disabled_controls = positioned_control_group(
        control_group(
            WINDOW_CONTROL_DISABLED_IDS,
            WINDOW_CONTROL_LARGE_SIZE,
            WINDOW_CONTROL_LARGE_GAP,
            state.scheme,
            true,
            true,
            false,
            ControlGroup::Disabled,
            state.hover_progress[ControlGroup::Disabled.index()],
            WindowExpandBehavior::Maximize,
            control_group_scales(&state.press_springs, ControlGroup::Disabled),
        ),
        WINDOW_CONTROL_DISABLED_X,
        WINDOW_CONTROL_DISABLED_Y,
        WINDOW_CONTROL_LARGE_SIZE,
    );

    let root = container(stack![
        // The selected scheme belongs to the window surface. Traffic-light
        // nodes opt into their own light reference sample in the GPU material;
        // do not turn the entire stage white just to keep their pigment
        // calibration stable.
        stage_background(state.scheme),
        window_drag_region(),
        info,
        tuning_panel,
        native_controls,
        reference_label,
        reference_controls,
        large_label,
        large_controls,
        inactive_label,
        inactive_controls,
        disabled_label,
        disabled_controls,
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(transparent_surface);

    let is_dark = state.scheme == UiColorScheme::Dark;
    let rimmed = liquid_glass::wrap_window_rim(
        root,
        liquid_glass::WindowRimConfig::new(is_dark)
            .with_corner_radius(state.window_policy.corner_radius() as f32),
    );

    liquid_glass::wrap_border_resizer(rimmed, false, Message::ResizeWindow)
}

fn configuration_text(state: &State) -> String {
    let scheme = match state.scheme {
        UiColorScheme::Light => "light",
        UiColorScheme::Dark => "dark",
    };
    let tuning = state.tuning;

    format!(
        "liquid-glass-window-controls-demo\n\
scheme = {scheme}\n\
blur_radius = {blur_radius:.2}\n\
internal_scattering = {internal_scattering:.4}\n\
side_edge_darkness = {side_edge_darkness:.4}\n\
side_edge_width = {side_edge_width:.4}\n\
opacity = {opacity:.4}\n\
substrate_coverage = {substrate_coverage:.4}\n\
lower_substrate_coverage = {lower_substrate_coverage:.4}\n\
lower_tint_coverage = {lower_tint_coverage:.4}\n\
angular_light = {angular_light:.4}\n\
light_angle = {light_angle:.4}\n\
light_softness = {light_softness:.4}\n\
body_thickness = {body_thickness:.4}\n\
edge_side_bias = {edge_side_bias:.4}\n\
edge_side_angle = {edge_side_angle:.2}\n\
refraction_strength = {refraction_strength:.4}\n\
fresnel_strength = {fresnel_strength:.4}\n",
        blur_radius = tuning.blur_radius,
        internal_scattering = tuning.internal_scattering,
        side_edge_darkness = tuning.side_edge_darkness,
        side_edge_width = tuning.side_edge_width,
        opacity = tuning.opacity,
        substrate_coverage = tuning.substrate_coverage,
        lower_substrate_coverage = tuning.lower_substrate_coverage,
        lower_tint_coverage = tuning.lower_tint_coverage,
        angular_light = tuning.angular_light,
        light_angle = tuning.light_angle,
        light_softness = tuning.light_softness,
        body_thickness = tuning.body_thickness,
        edge_side_bias = tuning.edge_side_bias,
        edge_side_angle = tuning.edge_side_angle,
        refraction_strength = tuning.refraction_strength,
        fresnel_strength = tuning.fresnel_strength,
    )
}

fn tuning_panel(tuning: WindowControlTuning) -> AppElement<'static> {
    let rows = components::settings_group(vec![
        components::setting_slider_with_step(
            "Blur radius",
            format!("{:.0} px", tuning.blur_radius),
            tuning.blur_radius,
            0.0..=80.0,
            1.0,
            |value| Message::TuningChanged { parameter: TuningParameter::BlurRadius, value },
        ),
        components::setting_slider_with_step(
            "Internal scattering",
            format!("{:.0}%", tuning.internal_scattering * 100.0),
            tuning.internal_scattering,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged {
                parameter: TuningParameter::InternalScattering,
                value,
            },
        ),
        components::setting_slider_with_step(
            "Edge darkness",
            format!("{:.2}", tuning.side_edge_darkness),
            tuning.side_edge_darkness,
            0.0..=4.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::SideEdgeDarkness, value },
        ),
        components::setting_slider_with_step(
            "Edge width",
            format!("{:.2}×", tuning.side_edge_width),
            tuning.side_edge_width,
            0.5..=4.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::SideEdgeWidth, value },
        ),
        components::setting_slider_with_step(
            "Opacity",
            format!("{:.0}%", tuning.opacity * 100.0),
            tuning.opacity,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::Opacity, value },
        ),
        components::setting_slider_with_step(
            "Upper substrate",
            format!("{:.0}%", tuning.substrate_coverage * 100.0),
            tuning.substrate_coverage,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::SubstrateCoverage, value },
        ),
        components::setting_slider_with_step(
            "Lower substrate",
            format!("{:.0}%", tuning.lower_substrate_coverage * 100.0),
            tuning.lower_substrate_coverage,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged {
                parameter: TuningParameter::LowerSubstrateCoverage,
                value,
            },
        ),
        components::setting_slider_with_step(
            "Lower tint coverage",
            format!("{:.0}%", tuning.lower_tint_coverage * 100.0),
            tuning.lower_tint_coverage,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::LowerTintCoverage, value },
        ),
        components::setting_slider_with_step(
            "Angular light",
            format!("{:.1}%", tuning.angular_light * 100.0),
            tuning.angular_light,
            0.0..=0.15,
            0.001,
            |value| Message::TuningChanged { parameter: TuningParameter::AngularLight, value },
        ),
        components::setting_slider_with_step(
            "Light angle",
            format!("{:.0}°", tuning.light_angle * 90.0),
            tuning.light_angle,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::LightAngle, value },
        ),
        components::setting_slider_with_step(
            "Light softness",
            format!("{:.0}%", tuning.light_softness * 100.0),
            tuning.light_softness,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::LightSoftness, value },
        ),
        components::setting_slider_with_step(
            "Body thickness",
            format!("{:.2}×", tuning.body_thickness),
            tuning.body_thickness,
            0.4..=2.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::BodyThickness, value },
        ),
        components::setting_slider_with_step(
            "Edge side bias",
            format!("{:.0}%", tuning.edge_side_bias * 100.0),
            tuning.edge_side_bias,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::EdgeSideBias, value },
        ),
        components::setting_slider_with_step(
            "Side thickness sigma",
            format!("{:.0}° σ", tuning.edge_side_angle),
            tuning.edge_side_angle,
            10.0..=80.0,
            1.0,
            |value| Message::TuningChanged { parameter: TuningParameter::EdgeSideAngle, value },
        ),
        components::setting_slider_with_step(
            "Refraction",
            format!("{:.2}", tuning.refraction_strength),
            tuning.refraction_strength,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged {
                parameter: TuningParameter::RefractionStrength,
                value,
            },
        ),
        components::setting_slider_with_step(
            "Fresnel edge",
            format!("{:.2}", tuning.fresnel_strength),
            tuning.fresnel_strength,
            0.0..=1.0,
            0.01,
            |value| Message::TuningChanged { parameter: TuningParameter::FresnelStrength, value },
        ),
    ]);

    container(
        column![
            text("Physical material tuning")
                .size(font::size::TITLE)
                .font(font::ui_font(iced::font::Weight::Semibold)),
            text("Applies to every traffic-light sample")
                .size(font::size::CAPTION)
                .style(components::secondary_text),
            scrollable(rows).height(Length::Fixed(500.0)),
        ]
        .spacing(10),
    )
    .width(Length::Fixed(440.0))
    .padding(16)
    .style(|theme| container::Style {
        background: Some(Background::Color(UiTheme::from_iced(theme).palette().group_background)),
        border: iced::Border::default().rounded(UiCornerStyle::GROUP.with_radius(16.0).radius()),
        ..container::Style::default()
    })
    .into()
}

fn stage_background(scheme: UiColorScheme) -> AppElement<'static> {
    // Use the same neutral substrate as the rest of the macOS window instead
    // of the old four-colour calibration board. The controls remain easy to
    // inspect, but their transmission is now judged against a titlebar-like
    // surface rather than an artificial saturated backdrop.
    color_panel(UiTheme::new(scheme).palette().window_background)
}

fn color_panel(color: Color) -> AppElement<'static> {
    container(space())
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(color)),
            ..container::Style::default()
        })
        .into()
}

fn transparent_surface(_theme: &Theme) -> container::Style {
    container::Style::default()
}

fn sample_label(label: &'static str) -> AppElement<'static> {
    container(text(label).size(font::size::BODY).font(font::ui_font(iced::font::Weight::Semibold)))
        .padding([4, 8])
        .style(|theme| container::Style {
            background: Some(Background::Color(
                UiTheme::from_iced(theme).palette().group_background,
            )),
            border: iced::Border::default().rounded(UiCornerStyle::MENU.radius()),
            ..container::Style::default()
        })
        .into()
}

fn positioned(content: AppElement<'static>, x: f32, y: f32) -> AppElement<'static> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding { top: y, right: 0.0, bottom: 0.0, left: x })
        .into()
}

fn window_drag_region() -> AppElement<'static> {
    let content = container(space())
        .width(Length::Fill)
        .height(Length::Fixed(iced_backend::FUSED_TOP_BAR_HEIGHT));
    WindowDragArea::new(content, Message::BeginWindowDrag).into_element()
}

fn positioned_control_group(
    content: AppElement<'static>,
    x: f32,
    y: f32,
    size: f32,
) -> AppElement<'static> {
    let slop = control_hover_slop(size);
    positioned(content, x - slop, y - slop)
}

fn control_hover_slop(size: f32) -> f32 {
    (size * 0.20).clamp(6.0, 12.0)
}

fn window_control_slot_index(id: GlassId) -> Option<usize> {
    ALL_WINDOW_CONTROL_IDS.iter().position(|candidate| *candidate == id)
}

fn finish_control_press(state: &mut State, id: GlassId) {
    finish_control_press_visual(state, id);
    if let Some(index) = window_control_slot_index(id) {
        // Retargeting preserves the current spring velocity, so the release
        // phase visibly shrinks with the same bouncy dynamics as the growth
        // phase, while settling at a still-slightly-larger resting size.
        state.press_springs[index].retarget(PRESS_SCALE_SETTLED);
    }
}

fn finish_control_press_visual(state: &mut State, id: GlassId) {
    if let Some(index) = window_control_slot_index(id) {
        state.press_targets[index] = 0.0;
    }
}

fn control_group_scales(springs: &[SpringMotion; 15], group: ControlGroup) -> [f32; 3] {
    let start = group.index() * 3;
    [springs[start].value(), springs[start + 1].value(), springs[start + 2].value()]
}

fn control_group(
    ids: [GlassId; 3],
    size: f32,
    gap: f32,
    scheme: UiColorScheme,
    show_glyphs: bool,
    close_disabled: bool,
    interactive: bool,
    group: ControlGroup,
    hover_amount: f32,
    expand_behavior: WindowExpandBehavior,
    press_scales: [f32; 3],
) -> AppElement<'static> {
    let inactive = group == ControlGroup::Inactive;
    let controls = row![
        window_control(
            ids[0],
            size,
            scheme,
            ControlAction::Close,
            show_glyphs,
            close_disabled,
            interactive,
            inactive,
            hover_amount,
            expand_behavior,
            press_scales[0],
        ),
        window_control(
            ids[1],
            size,
            scheme,
            ControlAction::Minimize,
            show_glyphs,
            false,
            interactive,
            inactive,
            hover_amount,
            expand_behavior,
            press_scales[1],
        ),
        window_control(
            ids[2],
            size,
            scheme,
            ControlAction::Expand,
            show_glyphs,
            false,
            interactive,
            inactive,
            hover_amount,
            expand_behavior,
            press_scales[2],
        ),
    ]
    .spacing(gap);
    let slop = control_hover_slop(size);
    let group_width = size * 3.0 + gap * 2.0;
    let tracking_area = container(controls)
        .width(Length::Fixed(group_width + slop * 2.0))
        .height(Length::Fixed(size + slop * 2.0))
        .padding(Padding { top: slop, right: slop, bottom: slop, left: slop });
    mouse_area(tracking_area)
        .on_enter(Message::ControlGroupHover { group, hovered: true })
        .on_exit(Message::ControlGroupHover { group, hovered: false })
        .into()
}

fn window_control(
    id: GlassId,
    size: f32,
    scheme: UiColorScheme,
    action: ControlAction,
    show_glyph: bool,
    disabled: bool,
    interactive: bool,
    inactive: bool,
    hover_amount: f32,
    expand_behavior: WindowExpandBehavior,
    scale: f32,
) -> AppElement<'static> {
    let visual_size = size * scale;
    let mut chrome: GlassChrome =
        UiTheme::new(scheme).compositor_chrome(GlassRole::FloatingControl);
    chrome.text = match scheme {
        UiColorScheme::Light | UiColorScheme::Dark => {
            liquid_glass::Color::rgba(0.22, 0.23, 0.25, 0.92)
        }
    };
    let button = GlassButton::new(id, "", Rect::new(0.0, 0.0, visual_size, visual_size))
        .shape(GlassShape::Circle)
        .material(GlassMaterial::interactive())
        .chrome({
            chrome.pressed_overlay = liquid_glass::Color::transparent();
            chrome
        });
    let button = if interactive {
        button.into_element_with_press_callbacks::<Message, Theme, Renderer>(
            Message::ControlPressed { id, action, execute: true },
            Some(Message::ControlPressStarted { id }),
            Some(Message::ControlPressVisualCancelled { id }),
            Some(Message::ControlPressEnded { id }),
        )
    } else {
        button.into_element::<Message, Theme, Renderer>(Message::ControlPressed {
            id,
            action,
            execute: false,
        })
    };
    let status_dot = action == ControlAction::Close && disabled;
    let button = centered(button, size);
    let glyph = if status_dot {
        let glyph_color = window_control_glyph_color(scheme, action, inactive, 1.0);
        window_control_status_dot(glyph_color, visual_size)
    } else {
        let focus_amount = if show_glyph { hover_amount } else { 0.0 };
        let glyph_color = window_control_glyph_color(scheme, action, inactive, focus_amount);
        components::icon_tinted_with_opacity(
            window_control_icon(action, expand_behavior),
            window_control_glyph_size(action, size) * scale,
            glyph_color,
        )
    };

    // Keep the button and its glyph in a stable tree shape. Changing from a
    // single child to a button+glyph stack during hover would recreate the
    // button's capture node exactly while a press is in flight.
    components::glass_overlay(stack![button, centered(glyph, size)])
}

fn window_control_status_dot(color: Color, size: f32) -> AppElement<'static> {
    let dot_size = (size * 0.24).max(3.0);
    container(space())
        .width(Length::Fixed(dot_size))
        .height(Length::Fixed(dot_size))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(color)),
            border: iced::Border::default().rounded(dot_size * 0.5),
            ..container::Style::default()
        })
        .into()
}

fn window_control_glyph_size(action: ControlAction, size: f32) -> f32 {
    // Measured from a 2x AppKit screenshot: the close glyph spans 14 px
    // (7 pt) and the minimize glyph spans 16 px (8 pt) inside a 14 pt
    // (28 px) traffic-light circle. The SVG viewBoxes are normalized to
    // those footprints, so these ratios remain valid on every scale.
    let factor = if size <= WINDOW_CONTROL_NATIVE_SIZE {
        match action {
            ControlAction::Close => 7.0 / 14.0,
            ControlAction::Minimize => 8.0 / 14.0,
            ControlAction::Expand => 0.42,
        }
    } else {
        0.42
    };
    (size * factor).max(4.0)
}

fn window_control_glyph_color(
    scheme: UiColorScheme,
    action: ControlAction,
    inactive: bool,
    focus_amount: f32,
) -> Color {
    let focus_amount = focus_amount.clamp(0.0, 1.0);
    let active = match action {
        // Keep each glyph in the button's hue family while darkening it
        // enough to remain legible over the saturated glass body.
        ControlAction::Close => Color::from_rgba(0.38, 0.10, 0.08, 0.92),
        ControlAction::Minimize => Color::from_rgba(0.42, 0.28, 0.03, 0.92),
        ControlAction::Expand => Color::from_rgba(0.09, 0.32, 0.07, 0.92),
    };
    let inactive_color = match scheme {
        UiColorScheme::Light => Color::from_rgba(0.52, 0.53, 0.56, 0.78),
        UiColorScheme::Dark => Color::from_rgba(0.82, 0.83, 0.86, 0.76),
    };
    let color = if inactive { blend_color(inactive_color, active, focus_amount) } else { active };
    Color::from_rgba(color.r, color.g, color.b, color.a * focus_amount)
}

fn blend_color(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color::from_rgba(
        from.r + (to.r - from.r) * amount,
        from.g + (to.g - from.g) * amount,
        from.b + (to.b - from.b) * amount,
        from.a + (to.a - from.a) * amount,
    )
}

fn window_control_icon(action: ControlAction, expand_behavior: WindowExpandBehavior) -> UiIcon {
    match action {
        ControlAction::Close => UiIcon::WindowClose,
        ControlAction::Minimize => UiIcon::WindowMinimize,
        ControlAction::Expand => expand_behavior.icon(),
    }
}

fn centered(content: AppElement<'static>, size: f32) -> AppElement<'static> {
    container(content)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .into()
}

fn main() -> iced::Result {
    let fonts = font::ui_fonts();
    let window_settings = demo_window_policy().apply(iced::window::Settings::default());

    let mut app = iced::application::<State, Message, Theme, Renderer>(boot, update, view)
        .title("Liquid Glass Window Controls")
        .theme(app_theme)
        .subscription(subscription)
        .window(window_settings)
        .window_size(iced::Size::new(1120.0, 720.0));
    for bytes in &fonts.bytes {
        app = app.font(bytes.clone());
    }
    if let Some(ui_font) = fonts.font() {
        app = app.default_font(ui_font);
    }
    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measured_and_large_samples_use_distinct_sizes() {
        assert_eq!(WINDOW_CONTROL_NATIVE_SIZE, 14.0);
        assert_eq!(WINDOW_CONTROL_LARGE_SIZE, 64.0);
        assert!(WINDOW_CONTROL_LARGE_SIZE > WINDOW_CONTROL_NATIVE_SIZE);
        assert_eq!(iced_backend::FUSED_TOP_BAR_HEIGHT, 52.0);
    }

    #[test]
    fn native_close_and_minimize_glyphs_get_extra_optical_weight() {
        let close = window_control_glyph_size(ControlAction::Close, WINDOW_CONTROL_NATIVE_SIZE);
        let minimize =
            window_control_glyph_size(ControlAction::Minimize, WINDOW_CONTROL_NATIVE_SIZE);
        let expand = window_control_glyph_size(ControlAction::Expand, WINDOW_CONTROL_NATIVE_SIZE);

        assert!(minimize > close);
        assert!(close > expand);
        assert_eq!(close, 7.0);
        assert_eq!(minimize, 8.0);
        assert_eq!(
            window_control_glyph_size(ControlAction::Close, WINDOW_CONTROL_LARGE_SIZE),
            WINDOW_CONTROL_LARGE_SIZE * 0.42
        );
    }

    #[test]
    fn inactive_window_glyphs_use_a_separate_pale_tone() {
        let active_light =
            window_control_glyph_color(UiColorScheme::Light, ControlAction::Close, false, 1.0);
        let inactive_light =
            window_control_glyph_color(UiColorScheme::Light, ControlAction::Close, true, 0.0);
        let active_dark =
            window_control_glyph_color(UiColorScheme::Dark, ControlAction::Close, false, 1.0);
        let inactive_dark =
            window_control_glyph_color(UiColorScheme::Dark, ControlAction::Close, true, 0.0);

        assert!(inactive_light.r > active_light.r);
        assert!(inactive_light.g > active_light.g);
        assert!(inactive_dark.r > active_dark.r);
        assert!(inactive_dark.g > active_dark.g);
    }
}
