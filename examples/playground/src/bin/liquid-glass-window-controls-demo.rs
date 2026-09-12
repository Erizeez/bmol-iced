//! Standalone Iced laboratory for custom macOS-style window controls (traffic lights).
//!
//! Rebuilt with the authentic physical optics pipeline:
//! - 2x Retina Point-to-Point Physical Pixels (1:1 native device resolution, zero bilinear blur)
//! - Quartic Bernstein-Bézier Droplet Profile (P1=1.25, P2=0.85, P3=0.20) for rich 3D bead lensing
//! - Physical Grazing Highlight (top-left specular glint with cubic drop-off)
//! - Subtractive Perimeter Crevice AO (dark hairline rim without artificial stroke outlines)
//! - Calibrated Jewel Glass Transmittance (Ruby Red, Amber Gold, Emerald Green, Platinum Gray)

#![allow(
    clippy::many_single_char_names,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::suboptimal_flops,
    clippy::similar_names
)]

#[path = "../iced_backend.rs"]
mod iced_backend;

use bmol_window_shell::{WindowChromeConfig, WindowShellController, window_metrics};
use iced::{
    Alignment, Background, Color, Element, Length, Padding, Subscription, Task, Theme,
    widget::{button, column, container, row, space, stack, svg, text},
};
use iced_backend::{
    Renderer, WINDOW_CONTROL_DISABLED_IDS, WINDOW_CONTROL_INACTIVE_IDS, WINDOW_CONTROL_LARGE_IDS,
    WINDOW_CONTROL_NATIVE_IDS, WINDOW_CONTROL_NATIVE_X, WINDOW_CONTROL_NATIVE_Y,
    WINDOW_CONTROL_REFERENCE_IDS,
};
use liquid_glass::{
    GlassId, UiColorScheme, WindowDragArea, WindowExpandBehavior,
    ui::{components, font},
};
use spring_rs::{Spring, SpringMotion};
use std::time::Duration;

pub use liquid_glass::traffic_lights as window_controls;

pub use window_controls::{
    ControlAction, ControlGroup, INTERACTION_ENTER_ANIMATION_TIME_CONSTANT,
    INTERACTION_EXIT_ANIMATION_TIME_CONSTANT, PRESS_SCALE_OVERSHOOT, PRESS_SCALE_SETTLED,
    PRESS_SCALE_SPRING_DURATION, PRESS_SCALE_SPRING_EXTRA_BOUNCE, TrafficLightsState,
    WINDOW_CONTROL_GAP, WINDOW_CONTROL_LARGE_GAP, WINDOW_CONTROL_LARGE_SIZE,
    WINDOW_CONTROL_NATIVE_SIZE, blend_color, centered, control_hover_slop,
    window_control_glyph_color, window_control_glyph_size, window_control_icon,
    window_control_status_dot,
};

type AppElement<'a> = Element<'a, Message, Theme, Renderer>;

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

/// Live press scale for one group's three controls.
fn press_scales(state: &State, group: ControlGroup) -> [f32; 3] {
    let base = group.index() * 3;
    [
        state.press_springs[base].value(),
        state.press_springs[base + 1].value(),
        state.press_springs[base + 2].value(),
    ]
}

// =========================================================================
// 1. Physical Traffic Light Optical Generator (2x Retina Point-to-Point)
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalTrafficLightTuning {
    pub highlight_intensity: f32, // 0.0 ..= 2.0 (default 0.85)
    pub dark_rim_intensity: f32,  // 0.0 ..= 3.0 (default 2.00)
    pub center_glow: f32,         // 0.0 ..= 2.0 (default 1.00)
    pub p1: f32,                  // 0.0 ..= 2.0 (default 0.00)
    pub p2: f32,                  // 0.0 ..= 2.0 (default 0.00)
    pub p3: f32,                  // 0.0 ..= 1.0 (default 0.00)
    pub core_span_factor: f32,    // 0.5 ..= 2.0 (default 1.0)
    pub rim_span_factor: f32,     // 0.5 ..= 2.0 (default 1.0)
    pub saturation_lift: f32,     // 0.0 ..= 0.50 (default 0.25)
}

impl Default for PhysicalTrafficLightTuning {
    fn default() -> Self {
        Self {
            highlight_intensity: 0.85,
            dark_rim_intensity: 2.00,
            center_glow: 1.00,
            p1: 0.00,
            p2: 0.00,
            p3: 0.00,
            core_span_factor: 1.0,
            rim_span_factor: 1.0,
            saturation_lift: 0.25,
        }
    }
}

pub fn render_physical_traffic_light_bead(
    size: f32,
    color: [f32; 3],
    is_dark: bool,
    hover_amount: f32,
    glow_scale: f32,
    tuning: PhysicalTrafficLightTuning,
) -> iced::widget::image::Handle {
    let scale = 2.0f32; // 2x Retina point-to-point physical pixel resolution
    let w = ((size * scale).round() as usize).max(1);
    let h = ((size * scale).round() as usize).max(1);
    let r = w as f32 * 0.5;
    let cx = r;
    let cy = r;

    // Hover brightens the ENTIRE button (lifts luminance & vibrancy by ~22%)
    let hover_lift = hover_amount.clamp(0.0, 1.0) * 0.22;
    let button_r = (color[0] * (1.0 + hover_lift)).min(1.0);
    let button_g = (color[1] * (1.0 + hover_lift)).min(1.0);
    let button_b = (color[2] * (1.0 + hover_lift)).min(1.0);

    let mut raw = vec![0u8; w * h * 4];
    for y in 0..h {
        let py = y as f32 + 0.5;
        let dy = py - cy;
        for x in 0..w {
            let px = x as f32 + 0.5;
            let dx = px - cx;
            let dist = (dx * dx + dy * dy).sqrt() - r;

            let idx = (y * w + x) * 4;
            if dist > 1.0 {
                continue;
            }

            let len = (dx * dx + dy * dy).sqrt().max(1e-5);
            let nx = dx / len;
            let ny = dy / len;
            let d = (-dist).max(0.0);

            // 1. Quartic Bernstein-Bézier Droplet Profile (P1=P2=P3=0 -> (1 - t)^4)
            let t_dist = (d / r).min(1.0);
            let u = 1.0 - t_dist;
            let falloff = u * u * u * u * 1.0
                + 4.0 * u * u * u * t_dist * tuning.p1
                + 6.0 * u * u * t_dist * t_dist * tuning.p2
                + 4.0 * u * t_dist * t_dist * t_dist * tuning.p3;

            let mode_caustic_mult = if is_dark { 0.80f32 } else { 1.10f32 };
            // 2. Vertical Axial Internal Glow (从下至上垂直轴向渐变，绝非圆弧形)
            let norm_y = dy / r;
            let vert_t = ((norm_y - (-0.15)) / (1.0 - (-0.15))).clamp(0.0, 1.0);
            let vert_glow = vert_t.powf(1.35);

            // Subtle horizontal edge roll-off
            let norm_x = dx / r;
            let horiz_mask = (1.0 - norm_x * norm_x).max(0.0).powf(0.25);
            let effective_glow = tuning.center_glow * glow_scale;
            let axial_light = vert_glow * horiz_mask * 0.42 * mode_caustic_mult * effective_glow;

            // Translucent glass core saturation lift + vertical axial glow
            let core_lift = (1.0 - falloff) * tuning.saturation_lift * effective_glow;
            let depth_lift = core_lift + axial_light;
            let base_r = button_r * (0.88 + depth_lift);
            let base_g = button_g * (0.88 + depth_lift);
            let base_b = button_b * (0.88 + depth_lift);

            // Light vs Dark Mode modulation (Mutually Exclusive / 独占生效):
            let (mode_high_mult, mode_dark_mult) = if is_dark {
                // 深色模式: 亮边独占生效 (1.0), 暗边完全不生效 (0.0)
                (1.0f32, 0.0f32)
            } else {
                // 浅色模式: 暗边独占生效 (1.0), 亮边完全不生效 (0.0)
                (0.0f32, 1.0f32)
            };

            // 2. Bright Edge: strictly TOP and BOTTOM (|ny| -> 1.0)
            let high_weight = ny.abs().powf(1.8);
            let core_span = (1.8f32 * scale).max(2.4 * scale * (size / 14.0).powf(0.5))
                * tuning.core_span_factor;
            let sharp_core = (1.0f32 - (d / core_span).min(1.0f32)).powi(3);
            let faint_halo = (1.0f32 - (d / r).min(1.0f32)).powi(2) * 0.06;
            let light_contrib = (sharp_core * 0.95 + faint_halo)
                * high_weight
                * mode_high_mult
                * tuning.highlight_intensity;

            // 3. Dark Rim: strictly LEFT and RIGHT (|nx| -> 1.0)
            let dark_weight = nx.abs().powf(2.0);
            let rim_span = (1.8f32 * scale).max(2.4 * scale * (size / 14.0).powf(0.5))
                * tuning.rim_span_factor;
            let rim_decay = (1.0f32 - (d / rim_span).min(1.0f32)).powi(2);
            let dark_drop = rim_decay
                * (52.0 / 255.0)
                * dark_weight
                * mode_dark_mult
                * tuning.dark_rim_intensity;

            // Combine
            let final_r = (base_r - dark_drop + light_contrib).clamp(0.0, 1.0);
            let final_g = (base_g - dark_drop + light_contrib).clamp(0.0, 1.0);
            let final_b = (base_b - dark_drop + light_contrib).clamp(0.0, 1.0);

            let alpha = (0.5 - dist).clamp(0.0, 1.0);
            raw[idx] = (final_r * 255.0).round() as u8;
            raw[idx + 1] = (final_g * 255.0).round() as u8;
            raw[idx + 2] = (final_b * 255.0).round() as u8;
            raw[idx + 3] = (alpha * 255.0).round() as u8;
        }
    }

    iced::widget::image::Handle::from_rgba(w as u32, h as u32, raw)
}

// Colors:
const COLOR_RUBY_RED: [f32; 3] = [1.00, 0.36, 0.34];
const COLOR_AMBER_YELLOW: [f32; 3] = [1.00, 0.74, 0.18];
const COLOR_EMERALD_GREEN: [f32; 3] = [0.16, 0.80, 0.28];
const COLOR_PLATINUM_GRAY: [f32; 3] = [0.82, 0.84, 0.88];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuningParameter {
    HighlightIntensity,
    DarkRimIntensity,
    CenterGlow,
    P1Convexity,
    P2Belly,
    P3Landing,
    CoreSpanFactor,
    RimSpanFactor,
    SaturationLift,
}

#[derive(Debug, Clone)]
enum Message {
    ControlPressStarted { id: GlassId },
    ControlReleased { id: GlassId, action: ControlAction, execute: bool },
    ControlPressVisualCancelled { id: GlassId },
    ControlGroupHover { group: ControlGroup, hovered: bool },
    WindowReady(Option<iced::window::Id>),
    WindowEvent((iced::window::Id, iced::window::Event)),
    BeginWindowDrag,
    AnimationTick,
    TuningChanged { parameter: TuningParameter, value: f32 },
    CopyConfiguration,
    ToggleScheme,
    SystemThemeChanged(iced::theme::Mode),
}

struct State {
    pub controller: WindowShellController,
    scheme: UiColorScheme,
    last_action: String,
    hover_targets: [f32; 5],
    hover_progress: [f32; 5],
    pressed_control: Option<GlassId>,
    press_targets: [f32; 15],
    press_progress: [f32; 15],
    press_springs: [SpringMotion; 15],
    tuning: PhysicalTrafficLightTuning,
    beads_14_norm: [iced::widget::image::Handle; 3],
    beads_14_hov: [iced::widget::image::Handle; 3],
    beads_14_pressed: [iced::widget::image::Handle; 3],
    beads_64_norm: [iced::widget::image::Handle; 3],
    beads_64_hov: [iced::widget::image::Handle; 3],
    beads_64_pressed: [iced::widget::image::Handle; 3],
    beads_64_inactive: [iced::widget::image::Handle; 3],
    beads_64_disabled: [iced::widget::image::Handle; 3],
}

impl State {
    fn regenerate_beads(&mut self) {
        let t = self.tuning;
        let is_dark = self.scheme == UiColorScheme::Dark;
        self.beads_14_norm = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
        self.beads_14_hov = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                0.0,
                1.00, // Hover lifts red light center glow to 100%!
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
        self.beads_14_pressed = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                1.0,
                1.00,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                1.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                1.0,
                0.60,
                t,
            ),
        ];
        self.beads_64_norm = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
        self.beads_64_hov = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                0.0,
                1.00, // Hover lifts red light center glow to 100%!
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
        self.beads_64_pressed = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                1.0,
                1.00,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                1.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                1.0,
                0.60,
                t,
            ),
        ];
        self.beads_64_inactive = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_PLATINUM_GRAY,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_PLATINUM_GRAY,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_PLATINUM_GRAY,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
        self.beads_64_disabled = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
    }
}

impl Default for State {
    fn default() -> Self {
        let config = WindowChromeConfig::unified_header(window_metrics::FUSED_HEADER_HEIGHT);
        let controller = WindowShellController::new(config, true);

        let t = PhysicalTrafficLightTuning::default();
        let is_dark = true;
        let beads_14_norm = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
        let beads_14_hov = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                0.0,
                1.00,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
        let beads_14_pressed = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                1.0,
                1.00,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                1.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_NATIVE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                1.0,
                0.60,
                t,
            ),
        ];
        let beads_64_norm = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
        let beads_64_hov = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                0.0,
                1.00,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
        let beads_64_pressed = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                1.0,
                1.00,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                1.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                1.0,
                0.60,
                t,
            ),
        ];
        let beads_64_inactive = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_PLATINUM_GRAY,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_PLATINUM_GRAY,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_PLATINUM_GRAY,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];
        let beads_64_disabled = [
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_RUBY_RED,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_AMBER_YELLOW,
                is_dark,
                0.0,
                0.60,
                t,
            ),
            render_physical_traffic_light_bead(
                WINDOW_CONTROL_LARGE_SIZE,
                COLOR_EMERALD_GREEN,
                is_dark,
                0.0,
                0.60,
                t,
            ),
        ];

        Self {
            controller,
            scheme: UiColorScheme::Dark,
            last_action: "Hover over a control to reveal its vector system glyph".into(),
            hover_targets: [0.0; 5],
            hover_progress: [0.0; 5],
            pressed_control: None,
            press_targets: [0.0; 15],
            press_progress: [0.0; 15],
            press_springs: std::array::from_fn(|_| {
                SpringMotion::new(
                    1.0,
                    1.0,
                    // zeta = 0.70: a slight, visible overshoot rather than the
                    // wide bounce the previous `bouncy_custom` produced.
                    Spring::perceptual(0.20, 0.30),
                )
            }),
            tuning: t,
            beads_14_norm,
            beads_14_hov,
            beads_14_pressed,
            beads_64_norm,
            beads_64_hov,
            beads_64_pressed,
            beads_64_inactive,
            beads_64_disabled,
        }
    }
}

fn boot() -> (State, Task<Message>) {
    let state = State::default();
    (state, Task::batch([iced::system::theme().map(Message::SystemThemeChanged)]))
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::ControlPressStarted { id } => {
            state.pressed_control = Some(id);
            if let Some(index) = window_control_slot_index(id) {
                state.press_targets[index] = 1.0;
                state.press_springs[index].retarget(PRESS_SCALE_OVERSHOOT);
            }
            state.last_action = format!("Pressed control #{id:?} (hold down to preview)");
            Task::none()
        }
        Message::ControlReleased { id, action, execute } => {
            let was_pressed = state.pressed_control == Some(id);
            state.pressed_control = None;
            finish_control_press(state, id);
            if was_pressed {
                state.last_action = format!("Triggered {action:?} on control #{id:?}");
                if execute {
                    match action {
                        ControlAction::Close => {
                            if let Some(wid) = state.controller.window_id {
                                return iced::window::close(wid);
                            }
                        }
                        ControlAction::Minimize => {
                            if let Some(wid) = state.controller.window_id {
                                return iced::window::minimize(wid, true);
                            }
                        }
                        ControlAction::Zoom | ControlAction::Expand => {
                            if let Some(wid) = state.controller.window_id {
                                return iced::window::toggle_maximize(wid);
                            }
                        }
                    }
                }
            }
            Task::none()
        }
        Message::ControlPressVisualCancelled { id } => {
            if state.pressed_control == Some(id) {
                state.pressed_control = None;
                state.last_action = format!("Cancelled press on control #{id:?}");
            }
            finish_control_press(state, id);
            Task::none()
        }
        Message::WindowReady(Some(id)) => {
            state.controller.set_window_id(id);
            let controller = state.controller.clone();
            iced::window::run(id, move |w| {
                if let Ok(handle) = w.window_handle() {
                    let _ = controller.setup_window(handle.as_raw());
                }
            })
            .discard()
        }
        Message::WindowReady(None) => Task::none(),
        Message::WindowEvent((id, event)) => {
            if let iced::window::Event::Opened { .. } = event {
                if state.controller.window_id.is_none() {
                    return update(state, Message::WindowReady(Some(id)));
                }
            }
            if let Some(shell_event) = state.controller.handle_window_event(&event) {
                match shell_event {
                    bmol_window_shell::ShellEvent::CloseRequested => {
                        if let Some(wid) = state.controller.window_id {
                            return iced::window::close(wid);
                        }
                    }
                    _ => {}
                }
            }
            Task::none()
        }
        Message::BeginWindowDrag => {
            if let Some(wid) = state.controller.window_id {
                iced::window::drag(wid)
            } else {
                Task::none()
            }
        }
        Message::ControlGroupHover { group, hovered } => {
            state.hover_targets[group.index()] = if hovered { 1.0 } else { 0.0 };
            Task::none()
        }
        Message::AnimationTick => {
            for (progress, target) in state.hover_progress.iter_mut().zip(state.hover_targets) {
                let time_constant = if target >= *progress {
                    INTERACTION_ENTER_ANIMATION_TIME_CONSTANT
                } else {
                    INTERACTION_EXIT_ANIMATION_TIME_CONSTANT
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
            for (id, spring) in
                ALL_WINDOW_CONTROL_IDS.into_iter().zip(state.press_springs.iter_mut())
            {
                spring.step(1.0 / 60.0);
                let _ = id;
            }
            Task::none()
        }
        Message::TuningChanged { parameter, value } => {
            match parameter {
                TuningParameter::HighlightIntensity => state.tuning.highlight_intensity = value,
                TuningParameter::DarkRimIntensity => state.tuning.dark_rim_intensity = value,
                TuningParameter::CenterGlow => state.tuning.center_glow = value,
                TuningParameter::P1Convexity => state.tuning.p1 = value,
                TuningParameter::P2Belly => state.tuning.p2 = value,
                TuningParameter::P3Landing => state.tuning.p3 = value,
                TuningParameter::CoreSpanFactor => state.tuning.core_span_factor = value,
                TuningParameter::RimSpanFactor => state.tuning.rim_span_factor = value,
                TuningParameter::SaturationLift => state.tuning.saturation_lift = value,
            }
            state.regenerate_beads();
            state.last_action = format!("Tuned {parameter:?} -> {value:.2}");
            Task::none()
        }
        Message::CopyConfiguration => {
            state.last_action = "Configuration copied to clipboard".into();
            iced::clipboard::write(configuration_text(state))
        }
        Message::ToggleScheme => {
            state.scheme = match state.scheme {
                UiColorScheme::Light => UiColorScheme::Dark,
                UiColorScheme::Dark => UiColorScheme::Light,
            };
            state.regenerate_beads();
            state.last_action = format!("Switched scheme to {:?}", state.scheme);
            Task::none()
        }
        Message::SystemThemeChanged(mode) => {
            state.scheme = match mode {
                iced::theme::Mode::Dark => UiColorScheme::Dark,
                _ => UiColorScheme::Light,
            };
            state.regenerate_beads();
            Task::none()
        }
    }
}

fn subscription(_state: &State) -> Subscription<Message> {
    Subscription::batch([
        iced::time::every(Duration::from_millis(16)).map(|_| Message::AnimationTick),
        iced::event::listen_with(|event, _status, id| match event {
            iced::Event::Window(event) => Some(Message::WindowEvent((id, event))),
            _ => None,
        }),
    ])
}

// =========================================================================
// 3. View Layout & Interactive Buttons
// =========================================================================

fn view(state: &State) -> AppElement<'_> {
    let is_dark = state.scheme == UiColorScheme::Dark;
    let title_row = row![
        text("Custom Window Controls")
            .size(28)
            .font(font::ui_font(iced::font::Weight::Bold))
            .color(if is_dark { Color::WHITE } else { Color::from_rgb(0.08, 0.08, 0.10) }),
        space().width(Length::Fill),
        button(text(if is_dark { "☀️ Light mode" } else { "🌙 Dark mode" }).size(12))
            .padding(Padding { top: 4.0, right: 10.0, bottom: 4.0, left: 10.0 })
            .on_press(Message::ToggleScheme),
    ]
    .align_y(Alignment::Center);

    let subtitle =
        text("Physical Water Droplet Lensing · 2x Retina Point-to-Point · Vector Glyphs on Hover")
            .size(13)
            .font(font::ui_font(iced::font::Weight::Medium))
            .color(if is_dark {
                Color::from_rgb(0.7, 0.72, 0.76)
            } else {
                Color::from_rgb(0.4, 0.42, 0.46)
            });

    let status_text = text(&state.last_action)
        .size(12)
        .font(font::ui_font(iced::font::Weight::Normal))
        .color(if is_dark {
            Color::from_rgb(0.5, 0.52, 0.56)
        } else {
            Color::from_rgb(0.55, 0.58, 0.62)
        });

    let info_header = column![title_row, subtitle, status_text].spacing(4);

    // 1. Native 14pt titlebar controls
    let native_group = physical_control_group(
        WINDOW_CONTROL_NATIVE_IDS,
        &state.beads_14_norm,
        &state.beads_14_hov,
        &state.beads_14_pressed,
        None,
        WINDOW_CONTROL_NATIVE_SIZE,
        WINDOW_CONTROL_GAP,
        is_dark,
        true,
        false,
        true,
        ControlGroup::Native,
        state.hover_progress[ControlGroup::Native.index()],
        WindowExpandBehavior::Fullscreen,
        state.pressed_control,
        press_scales(state, ControlGroup::Native),
    );

    // 2. 1:1 reference sample (14pt)
    let ref_label = sample_label("1:1 reference · 14 pt visual diameter", is_dark);
    let ref_group = physical_control_group(
        WINDOW_CONTROL_REFERENCE_IDS,
        &state.beads_14_norm,
        &state.beads_14_hov,
        &state.beads_14_pressed,
        None,
        WINDOW_CONTROL_NATIVE_SIZE,
        WINDOW_CONTROL_GAP,
        is_dark,
        true,
        false,
        false,
        ControlGroup::Reference,
        state.hover_progress[ControlGroup::Reference.index()],
        WindowExpandBehavior::Fullscreen,
        state.pressed_control,
        press_scales(state, ControlGroup::Reference),
    );

    // 3. 64pt Maximize sample (Active)
    let large_label = sample_label("Maximize behavior · plus glyph · 64 pt", is_dark);
    let large_group = physical_control_group(
        WINDOW_CONTROL_LARGE_IDS,
        &state.beads_64_norm,
        &state.beads_64_hov,
        &state.beads_64_pressed,
        None,
        WINDOW_CONTROL_LARGE_SIZE,
        WINDOW_CONTROL_LARGE_GAP,
        is_dark,
        true,
        false,
        true,
        ControlGroup::Active,
        state.hover_progress[ControlGroup::Active.index()],
        WindowExpandBehavior::Maximize,
        state.pressed_control,
        press_scales(state, ControlGroup::Active),
    );

    // 4. 64pt Inactive sample
    // In macOS: hovering over an inactive window reveals its 3 focused colored beads!
    let inactive_label = sample_label("Inactive window · all controls pale gray", is_dark);
    let inactive_group = physical_control_group(
        WINDOW_CONTROL_INACTIVE_IDS,
        &state.beads_64_norm,
        &state.beads_64_hov,
        &state.beads_64_pressed,
        Some(&state.beads_64_inactive),
        WINDOW_CONTROL_LARGE_SIZE,
        WINDOW_CONTROL_LARGE_GAP,
        is_dark,
        true,
        false,
        false,
        ControlGroup::Inactive,
        state.hover_progress[ControlGroup::Inactive.index()],
        WindowExpandBehavior::Fullscreen,
        state.pressed_control,
        press_scales(state, ControlGroup::Inactive),
    );

    // 5. 64pt Disabled / Running dot sample
    let disabled_label = sample_label("Running · close status dot", is_dark);
    let disabled_group = physical_control_group(
        WINDOW_CONTROL_DISABLED_IDS,
        &state.beads_64_disabled,
        &state.beads_64_hov,
        &state.beads_64_pressed,
        None,
        WINDOW_CONTROL_LARGE_SIZE,
        WINDOW_CONTROL_LARGE_GAP,
        is_dark,
        true,
        true,
        false,
        ControlGroup::Disabled,
        state.hover_progress[ControlGroup::Disabled.index()],
        WindowExpandBehavior::Maximize,
        state.pressed_control,
        press_scales(state, ControlGroup::Disabled),
    );

    let left_column = column![
        info_header,
        space().height(16),
        ref_label,
        ref_group,
        space().height(16),
        large_label,
        large_group,
        space().height(16),
        inactive_label,
        inactive_group,
        space().height(16),
        disabled_label,
        disabled_group,
    ]
    .spacing(6)
    .width(Length::FillPortion(3));

    let tuning_card = physical_tuning_panel(state.tuning, is_dark);
    let right_column = column![tuning_card].width(Length::FillPortion(2)).padding(Padding {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 16.0,
    });

    let content_layout = row![left_column, right_column].spacing(16).padding(Padding {
        top: 56.0,
        right: 32.0,
        bottom: 32.0,
        left: 32.0,
    });

    // Top titlebar native controls
    let top_bar = row![
        space().width(Length::Fixed(WINDOW_CONTROL_NATIVE_X)),
        native_group,
        space().width(Length::Fill),
    ]
    .padding(Padding { top: WINDOW_CONTROL_NATIVE_Y, ..Padding::ZERO });

    let stage_bg =
        container(space().width(Length::Fill).height(Length::Fill)).style(move |_theme| {
            container::Style {
                background: Some(Background::Color(if is_dark {
                    Color::from_rgb(0.12, 0.13, 0.16)
                } else {
                    Color::from_rgb(0.94, 0.95, 0.97)
                })),
                ..Default::default()
            }
        });

    container(stack![stage_bg, window_drag_region(), content_layout, top_bar,])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn sample_label(text_content: &'static str, is_dark: bool) -> AppElement<'static> {
    container(text(text_content).size(11).font(font::ui_font(iced::font::Weight::Semibold)).color(
        if is_dark { Color::from_rgb(0.85, 0.86, 0.90) } else { Color::from_rgb(0.25, 0.26, 0.30) },
    ))
    .padding(Padding { top: 2.0, right: 8.0, bottom: 2.0, left: 8.0 })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.08)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.05)
        })),
        border: iced::Border::default().rounded(6.0),
        ..Default::default()
    })
    .into()
}

fn window_drag_region() -> AppElement<'static> {
    let content = container(space().width(Length::Fill).height(Length::Fixed(44.0)))
        .width(Length::Fill)
        .height(Length::Fixed(44.0));
    WindowDragArea::new(content, Message::BeginWindowDrag).into_element()
}

fn window_control_slot_index(id: GlassId) -> Option<usize> {
    ALL_WINDOW_CONTROL_IDS.iter().position(|candidate| *candidate == id)
}

fn finish_control_press(state: &mut State, id: GlassId) {
    if let Some(index) = window_control_slot_index(id) {
        state.press_targets[index] = 0.0;
        // Rest is 1.0: the control returns to its natural size on release.
        state.press_springs[index].retarget(1.0);
    }
}

#[allow(clippy::fn_params_excessive_bools)]
fn physical_control_group<'a>(
    ids: [GlassId; 3],
    norm_handles: &[iced::widget::image::Handle; 3],
    hov_handles: &[iced::widget::image::Handle; 3],
    pressed_handles: &[iced::widget::image::Handle; 3],
    inactive_handles: Option<&[iced::widget::image::Handle; 3]>,
    size: f32,
    gap: f32,
    is_dark: bool,
    show_glyphs: bool,
    close_disabled: bool,
    interactive: bool,
    group: ControlGroup,
    hover_amount: f32,
    expand_behavior: WindowExpandBehavior,
    pressed_control: Option<GlassId>,
    control_scales: [f32; 3],
) -> AppElement<'a> {
    let hover = if show_glyphs { hover_amount } else { 0.0 };
    let is_fullscreen = expand_behavior == WindowExpandBehavior::Fullscreen;

    // Hover policy:
    // When hovered, use `hov_handles` (Red lifts to 100% center glow, Yellow/Green stay at 60%)!
    // On inactive window, unhovered uses `inactive_handles` (pale gray), hovered awakens to `hov_handles`!
    let active_color_handles = if let Some(inact) = inactive_handles {
        if hover_amount > 0.05 { hov_handles } else { inact }
    } else if hover_amount > 0.05 {
        hov_handles
    } else {
        norm_handles
    };

    // Click policy: ONLY the single clicked button brightens while pressed!
    let btn_close = view_physical_button(
        ControlAction::Close,
        if pressed_control == Some(ids[0]) {
            pressed_handles[0].clone()
        } else {
            active_color_handles[0].clone()
        },
        size,
        control_scales[0],
        hover,
        is_dark,
        is_fullscreen,
        close_disabled,
        Message::ControlPressStarted { id: ids[0] },
        Message::ControlReleased { id: ids[0], action: ControlAction::Close, execute: interactive },
        Message::ControlPressVisualCancelled { id: ids[0] },
    );

    let btn_min = view_physical_button(
        ControlAction::Minimize,
        if pressed_control == Some(ids[1]) {
            pressed_handles[1].clone()
        } else {
            active_color_handles[1].clone()
        },
        size,
        control_scales[1],
        hover,
        is_dark,
        is_fullscreen,
        false,
        Message::ControlPressStarted { id: ids[1] },
        Message::ControlReleased {
            id: ids[1],
            action: ControlAction::Minimize,
            execute: interactive,
        },
        Message::ControlPressVisualCancelled { id: ids[1] },
    );

    let btn_zoom = view_physical_button(
        ControlAction::Expand,
        if pressed_control == Some(ids[2]) {
            pressed_handles[2].clone()
        } else {
            active_color_handles[2].clone()
        },
        size,
        control_scales[2],
        hover,
        is_dark,
        is_fullscreen,
        false,
        Message::ControlPressStarted { id: ids[2] },
        Message::ControlReleased {
            id: ids[2],
            action: ControlAction::Expand,
            execute: interactive,
        },
        Message::ControlPressVisualCancelled { id: ids[2] },
    );

    let slop = control_hover_slop(size);
    let content = row![btn_close, space().width(gap), btn_min, space().width(gap), btn_zoom]
        .align_y(Alignment::Center);

    let mouse_area = iced::widget::mouse_area(content)
        .on_enter(Message::ControlGroupHover { group, hovered: true })
        .on_exit(Message::ControlGroupHover { group, hovered: false });

    container(mouse_area)
        .padding(Padding {
            top: slop * 0.5,
            right: slop * 0.5,
            bottom: slop * 0.5,
            left: slop * 0.5,
        })
        .into()
}

fn view_physical_button<'a>(
    action: ControlAction,
    texture: iced::widget::image::Handle,
    visual_size: f32,
    scale: f32,
    hover_amount: f32,
    is_dark: bool,
    is_fullscreen: bool,
    show_status_dot: bool,
    on_press_start: Message,
    on_release: Message,
    on_cancel: Message,
) -> AppElement<'a> {
    // The pressed control grows about its centre: draw every layer at
    // `visual_size * scale`, but keep a fixed layout box so the neighbouring
    // controls and the row spacing never move. `visual_size` is shadowed so the
    // glyphs, status dot and hit area scale with the bead.
    let box_size = visual_size;
    let visual_size = visual_size * scale;
    let bead_img = iced::widget::image(texture)
        .width(Length::Fixed(visual_size))
        .height(Length::Fixed(visual_size));

    let glyph_elem: Element<'a, Message, Theme, Renderer> = if show_status_dot {
        let dot_color = window_control_glyph_color(is_dark, action, false, 1.0);
        container(window_control_status_dot(dot_color, visual_size))
            .width(Length::Fixed(visual_size))
            .height(Length::Fixed(visual_size))
            .center_x(Length::Fixed(visual_size))
            .center_y(Length::Fixed(visual_size))
            .into()
    } else if hover_amount > 0.001 {
        let glyph_svg = match action {
            ControlAction::Close => window_controls::SVG_CLOSE,
            ControlAction::Minimize => window_controls::SVG_MINIMIZE,
            ControlAction::Zoom | ControlAction::Expand => {
                if is_fullscreen {
                    window_controls::SVG_ZOOM
                } else {
                    window_controls::SVG_MAXIMIZE
                }
            }
        };
        let glyph_color = window_control_glyph_color(is_dark, action, false, hover_amount);
        let glyph_size = window_control_glyph_size(action, visual_size);
        container(
            svg(svg::Handle::from_memory(glyph_svg.as_bytes()))
                .width(Length::Fixed(glyph_size))
                .height(Length::Fixed(glyph_size))
                .style(move |_theme, _status| svg::Style { color: Some(glyph_color) }),
        )
        .width(Length::Fixed(visual_size))
        .height(Length::Fixed(visual_size))
        .center_x(Length::Fixed(visual_size))
        .center_y(Length::Fixed(visual_size))
        .into()
    } else {
        container(space().width(Length::Fixed(visual_size)).height(Length::Fixed(visual_size)))
            .width(Length::Fixed(visual_size))
            .height(Length::Fixed(visual_size))
            .into()
    };

    let layer = stack![bead_img, glyph_elem];
    let mouse_area = iced::widget::mouse_area(layer)
        .on_press(on_press_start)
        .on_release(on_release)
        .on_exit(on_cancel)
        .interaction(iced::mouse::Interaction::Pointer);

    centered(
        container(mouse_area)
            .width(Length::Fixed(visual_size))
            .height(Length::Fixed(visual_size))
            .into(),
        box_size,
    )
}

// =========================================================================
// 4. Physical Material Tuning Panel
// =========================================================================

fn configuration_text(state: &State) -> String {
    let t = state.tuning;
    format!(
        "// Calibrated Physical Traffic Light Constants:\n\
         let mut m = ContentGlassMaterial::traffic_light(TrafficLightKind::Close)\n\
             .with_bezier_profile(0.00, 0.00, 0.00)\n\
             .with_highlight_intensity({:.2})\n\
             .with_dark_rim({:.2})\n\
             .with_internal_glow({:.2});\n",
        t.highlight_intensity, t.dark_rim_intensity, t.center_glow
    )
}

fn physical_tuning_panel(tuning: PhysicalTrafficLightTuning, is_dark: bool) -> AppElement<'static> {
    let rows = components::settings_group(vec![
        components::setting_slider_with_step(
            "打光强度 (Highlight)",
            format!("{:.0}%", tuning.highlight_intensity * 100.0),
            tuning.highlight_intensity,
            0.0..=2.0,
            0.05,
            |value| Message::TuningChanged {
                parameter: TuningParameter::HighlightIntensity,
                value,
            },
        ),
        components::setting_slider_with_step(
            "微缝暗边 (Dark Rim)",
            format!("{:.2}", tuning.dark_rim_intensity),
            tuning.dark_rim_intensity,
            0.0..=3.0,
            0.05,
            |value| Message::TuningChanged { parameter: TuningParameter::DarkRimIntensity, value },
        ),
        components::setting_slider_with_step(
            "中间光感 (Center Glow)",
            format!("{:.0}%", tuning.center_glow * 100.0),
            tuning.center_glow,
            0.0..=2.0,
            0.05,
            |value| Message::TuningChanged { parameter: TuningParameter::CenterGlow, value },
        ),
        components::setting_slider_with_step(
            "肩部凸度 P1 (Shoulder)",
            format!("{:.2}", tuning.p1),
            tuning.p1,
            0.0..=2.0,
            0.02,
            |value| Message::TuningChanged { parameter: TuningParameter::P1Convexity, value },
        ),
        components::setting_slider_with_step(
            "腰身弧度 P2 (Belly)",
            format!("{:.2}", tuning.p2),
            tuning.p2,
            0.0..=2.0,
            0.02,
            |value| Message::TuningChanged { parameter: TuningParameter::P2Belly, value },
        ),
        components::setting_slider_with_step(
            "落底平滑 P3 (Floor Landing)",
            format!("{:.2}", tuning.p3),
            tuning.p3,
            0.0..=1.0,
            0.02,
            |value| Message::TuningChanged { parameter: TuningParameter::P3Landing, value },
        ),
        components::setting_slider_with_step(
            "高光微核范围 (Core Span)",
            format!("{:.2}x", tuning.core_span_factor),
            tuning.core_span_factor,
            0.5..=2.0,
            0.05,
            |value| Message::TuningChanged { parameter: TuningParameter::CoreSpanFactor, value },
        ),
        components::setting_slider_with_step(
            "暗边微缝范围 (Rim Span)",
            format!("{:.2}x", tuning.rim_span_factor),
            tuning.rim_span_factor,
            0.5..=2.0,
            0.05,
            |value| Message::TuningChanged { parameter: TuningParameter::RimSpanFactor, value },
        ),
        components::setting_slider_with_step(
            "核心饱和增益 (Core Saturation)",
            format!("{:.2}", tuning.saturation_lift),
            tuning.saturation_lift,
            0.0..=0.50,
            0.02,
            |value| Message::TuningChanged { parameter: TuningParameter::SaturationLift, value },
        ),
    ]);

    container(
        column![
            row![
                text("Physical Material Tuning")
                    .size(16)
                    .font(font::ui_font(iced::font::Weight::Bold))
                    .color(if is_dark { Color::WHITE } else { Color::BLACK }),
                space().width(Length::Fill),
                button(text("Copy Config").size(12))
                    .padding(Padding { top: 4.0, right: 10.0, bottom: 4.0, left: 10.0 })
                    .on_press(Message::CopyConfiguration),
            ],
            text("Live physical GPU shader parameters for 3D liquid glass beads").size(12).color(
                if is_dark {
                    Color::from_rgb(0.6, 0.62, 0.66)
                } else {
                    Color::from_rgb(0.4, 0.42, 0.46)
                }
            ),
            space().height(12),
            rows,
        ]
        .spacing(8),
    )
    .padding(16)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.05)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.03)
        })),
        border: iced::Border::default().rounded(12.0),
        ..Default::default()
    })
    .into()
}

fn main() -> iced::Result {
    let fonts = font::ui_fonts();
    let mut app = iced::application::<State, Message, Theme, Renderer>(boot, update, view)
        .title("Liquid Glass Window Controls")
        .subscription(subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1120.0, 720.0),
            ..Default::default()
        });
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
        let active_light = window_control_glyph_color(false, ControlAction::Close, false, 1.0);
        let inactive_light = window_control_glyph_color(false, ControlAction::Close, true, 0.0);
        let active_dark = window_control_glyph_color(true, ControlAction::Close, false, 1.0);
        let inactive_dark = window_control_glyph_color(true, ControlAction::Close, true, 0.0);

        assert!(inactive_light.r > active_light.r);
        assert!(inactive_light.g > active_light.g);
        assert!(inactive_dark.r > active_dark.r);
        assert!(inactive_dark.g > active_dark.g);
    }
}
