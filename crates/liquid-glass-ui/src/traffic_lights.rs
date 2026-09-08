//! Reusable macOS Liquid Glass Window Controls (Traffic Lights).
//!
//! Provides pixel-accurate macOS window controls (Close, Minimize, Zoom/Expand)
//! with liquid-glass styling, spring physics scaling, extracted Apple vector glyphs,
//! group hover sensing, and inactive window state blending.

use std::time::Instant;

use iced::advanced::{self, svg as advanced_svg, text as advanced_text};
use iced::widget::{container, mouse_area, row, space, stack};
use iced::{Background, Color, Element, Length, Padding, Theme};
use liquid_glass_scene::{GlassId, GlassMaterial, GlassShape, Rect};
use spring_rs::{Spring, SpringMotion};

use crate::components;
use crate::icon::UiIcon;
use crate::theme::{GlassChrome, GlassRole, UiColorScheme, UiTheme};
use crate::window::WindowExpandBehavior;
use crate::{GlassButton, GlassForegroundRenderer};

// Standard macOS traffic light dimensions
pub const WINDOW_CONTROL_NATIVE_SIZE: f32 = 14.0;
pub const WINDOW_CONTROL_LARGE_SIZE: f32 = 64.0;
pub const WINDOW_CONTROL_GAP: f32 = 9.0;
pub const WINDOW_CONTROL_LARGE_GAP: f32 = 12.0;

pub const WINDOW_CONTROL_NATIVE_IDS: [GlassId; 3] = [GlassId(100), GlassId(101), GlassId(102)];

#[must_use]
pub fn slot_index(id: GlassId) -> Option<usize> {
    WINDOW_CONTROL_NATIVE_IDS.iter().position(|&x| x == id)
}

// Spring physics constants for bouncy clicks
pub const PRESS_SCALE_OVERSHOOT: f32 = 1.18;
pub const PRESS_SCALE_SETTLED: f32 = 1.06;
pub const PRESS_SCALE_SPRING_DURATION: f32 = 0.24;
pub const PRESS_SCALE_SPRING_EXTRA_BOUNCE: f32 = 0.40;
pub const INTERACTION_ENTER_ANIMATION_TIME_CONSTANT: f32 = 0.08;
pub const INTERACTION_EXIT_ANIMATION_TIME_CONSTANT: f32 = 0.18;

/// A window control semantic action.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ControlAction {
    Close,
    Minimize,
    Expand,
}

impl ControlAction {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::Minimize => "Minimize",
            Self::Expand => "Expand",
        }
    }
}

/// Control group variants for preview and inspection.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ControlGroup {
    Native,
    Reference,
    Active,
    Inactive,
    Disabled,
}

impl ControlGroup {
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Native => 0,
            Self::Reference => 1,
            Self::Active => 2,
            Self::Inactive => 3,
            Self::Disabled => 4,
        }
    }
}

/// Dynamic state managing the liquid glass traffic lights (springs, hover fade, tick timer).
#[derive(Debug, Clone)]
pub struct TrafficLightsState {
    pub hover_progress: f32,
    pub hover_target: f32,
    pub press_springs: [SpringMotion; 3],
    pub press_targets: [f32; 3],
    pub expand_behavior: WindowExpandBehavior,
    pub last_tick: Option<Instant>,
}

impl Default for TrafficLightsState {
    fn default() -> Self {
        Self {
            hover_progress: 0.0,
            hover_target: 0.0,
            press_springs: [
                SpringMotion::new(
                    1.0,
                    1.0,
                    Spring::bouncy_custom(
                        PRESS_SCALE_SPRING_DURATION,
                        PRESS_SCALE_SPRING_EXTRA_BOUNCE,
                    ),
                ),
                SpringMotion::new(
                    1.0,
                    1.0,
                    Spring::bouncy_custom(
                        PRESS_SCALE_SPRING_DURATION,
                        PRESS_SCALE_SPRING_EXTRA_BOUNCE,
                    ),
                ),
                SpringMotion::new(
                    1.0,
                    1.0,
                    Spring::bouncy_custom(
                        PRESS_SCALE_SPRING_DURATION,
                        PRESS_SCALE_SPRING_EXTRA_BOUNCE,
                    ),
                ),
            ],
            press_targets: [0.0; 3],
            expand_behavior: WindowExpandBehavior::Fullscreen,
            last_tick: None,
        }
    }
}

impl TrafficLightsState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_group_hover(&mut self, hovered: bool) {
        self.hover_target = if hovered { 1.0 } else { 0.0 };
        if self.last_tick.is_none() {
            self.last_tick = Some(Instant::now());
        }
    }

    pub fn on_press_start(&mut self, index: usize) {
        if index < 3 {
            self.press_targets[index] = 1.0;
            self.press_springs[index].retarget(PRESS_SCALE_OVERSHOOT);
            if self.last_tick.is_none() {
                self.last_tick = Some(Instant::now());
            }
        }
    }

    pub fn on_press_end(&mut self, index: usize) {
        if index < 3 {
            self.press_targets[index] = 0.0;
            self.press_springs[index].retarget(PRESS_SCALE_SETTLED);
            if self.last_tick.is_none() {
                self.last_tick = Some(Instant::now());
            }
        }
    }

    pub fn on_press_cancel(&mut self, index: usize) {
        if index < 3 {
            self.press_targets[index] = 0.0;
            self.press_springs[index].retarget(1.0);
            if self.last_tick.is_none() {
                self.last_tick = Some(Instant::now());
            }
        }
    }

    pub fn step(&mut self, now: Instant) {
        let dt = if let Some(last) = self.last_tick {
            now.saturating_duration_since(last).as_secs_f32().min(0.1)
        } else {
            0.016
        };
        self.last_tick = Some(now);

        let hover_tc = if self.hover_target >= self.hover_progress {
            INTERACTION_ENTER_ANIMATION_TIME_CONSTANT
        } else {
            INTERACTION_EXIT_ANIMATION_TIME_CONSTANT
        };
        let hover_step = 1.0 - (-dt / hover_tc).exp();
        self.hover_progress += (self.hover_target - self.hover_progress) * hover_step;
        if (self.hover_target - self.hover_progress).abs() < 0.001 {
            self.hover_progress = self.hover_target;
        }

        for (spring, target) in self.press_springs.iter_mut().zip(self.press_targets.iter()) {
            spring.step(dt);
            if spring.is_settled(0.001, 0.01) && (*target - spring.value()).abs() < 0.001 {
                spring.retarget(*target);
            }
        }

        if !self.is_animating() {
            self.last_tick = None;
        }
    }

    #[must_use]
    pub fn is_animating(&self) -> bool {
        (self.hover_target - self.hover_progress).abs() > 0.001
            || self.press_springs.iter().any(|s| !s.is_settled(0.001, 0.01))
    }
}

/// Measured glyph size inside a control circle of `size`.
#[must_use]
pub fn window_control_glyph_size(action: ControlAction, size: f32) -> f32 {
    let factor = if size <= WINDOW_CONTROL_NATIVE_SIZE {
        match action {
            ControlAction::Close => 7.0 / WINDOW_CONTROL_NATIVE_SIZE,
            ControlAction::Minimize => 8.0 / WINDOW_CONTROL_NATIVE_SIZE,
            ControlAction::Expand => 0.42,
        }
    } else {
        0.42
    };
    (size * factor).max(4.0)
}

/// Color for the control glyph based on action, color scheme, inactive state, and focus amount.
#[must_use]
pub fn window_control_glyph_color(
    scheme: UiColorScheme,
    action: ControlAction,
    inactive: bool,
    focus_amount: f32,
) -> Color {
    let focus_amount = focus_amount.clamp(0.0, 1.0);
    let active = match action {
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

/// Returns the matching vector icon for `action`.
#[must_use]
pub fn window_control_icon(action: ControlAction, expand_behavior: WindowExpandBehavior) -> UiIcon {
    match action {
        ControlAction::Close => UiIcon::WindowClose,
        ControlAction::Minimize => UiIcon::WindowMinimize,
        ControlAction::Expand => expand_behavior.icon(),
    }
}

/// Expanded hover detection buffer around window controls to prevent flicker.
#[must_use]
pub fn control_hover_slop(size: f32) -> f32 {
    (size * 0.20).clamp(6.0, 12.0)
}

/// Blends two colors linearly by `amount`.
#[must_use]
pub fn blend_color(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color::from_rgba(
        from.r + (to.r - from.r) * amount,
        from.g + (to.g - from.g) * amount,
        from.b + (to.b - from.b) * amount,
        from.a + (to.a - from.a) * amount,
    )
}

/// Status dot for disabled or unsaved close control.
#[must_use]
pub fn window_control_status_dot<'a, Message: 'a, Renderer>(
    color: Color,
    size: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer + 'a,
{
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

/// Centered container helper for optical alignment.
#[must_use]
pub fn centered<'a, Message: 'a, Renderer>(
    content: Element<'a, Message, Theme, Renderer>,
    size: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer + 'a,
{
    container(content)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .into()
}

/// Authentic liquid-glass colors and borders for traffic light buttons (fallback).
#[allow(dead_code, clippy::fn_params_excessive_bools, clippy::if_not_else)]
#[must_use]
pub fn traffic_light_appearance(
    action: ControlAction,
    is_active: bool,
    is_dark: bool,
    is_hover: bool,
    is_press: bool,
) -> (Color, Color) {
    if !is_active {
        if is_dark {
            (
                Color::from_rgba(0.32, 0.32, 0.35, 0.50),
                Color::from_rgba(0.24, 0.24, 0.26, 0.40),
            )
        } else {
            (
                Color::from_rgba(0.85, 0.85, 0.87, 0.95),
                Color::from_rgba(0.72, 0.72, 0.75, 0.70),
            )
        }
    } else {
        match action {
            ControlAction::Close => {
                if is_press {
                    (Color::from_rgb8(0xD2, 0x3C, 0x34), Color::from_rgb8(0xBF, 0x34, 0x2D))
                } else if is_hover {
                    (Color::from_rgb8(0xFF, 0x6E, 0x67), Color::from_rgb8(0xE2, 0x4B, 0x46))
                } else {
                    (Color::from_rgb8(0xFF, 0x5F, 0x56), Color::from_rgb8(0xE0, 0x44, 0x3E))
                }
            }
            ControlAction::Minimize => {
                if is_press {
                    (Color::from_rgb8(0xD7, 0x96, 0x1E), Color::from_rgb8(0xC2, 0x82, 0x16))
                } else if is_hover {
                    (Color::from_rgb8(0xFF, 0xC8, 0x47), Color::from_rgb8(0xDF, 0xA9, 0x32))
                } else {
                    (Color::from_rgb8(0xFF, 0xBD, 0x2E), Color::from_rgb8(0xDE, 0xA1, 0x23))
                }
            }
            ControlAction::Expand => {
                if is_press {
                    (Color::from_rgb8(0x19, 0xA0, 0x23), Color::from_rgb8(0x14, 0x8C, 0x1C))
                } else if is_hover {
                    (Color::from_rgb8(0x32, 0xD8, 0x4D), Color::from_rgb8(0x23, 0xB6, 0x33))
                } else {
                    (Color::from_rgb8(0x27, 0xC9, 0x3F), Color::from_rgb8(0x1A, 0xAB, 0x29))
                }
            }
        }
    }
}

/// Builds an individual Liquid Glass traffic light button.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub fn window_control<'a, Message: Clone + 'static, Renderer>(
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
    on_action: Message,
    on_press_start: Option<Message>,
    on_press_cancel: Option<Message>,
    on_press_end: Option<Message>,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer
        + advanced_text::Renderer
        + advanced_svg::Renderer
        + GlassForegroundRenderer
        + 'static,
{
    let visual_size = size * scale;
    let mut chrome: GlassChrome =
        UiTheme::new(scheme).compositor_chrome(GlassRole::FloatingControl);
    chrome.text = match scheme {
        UiColorScheme::Light | UiColorScheme::Dark => {
            liquid_glass_scene::Color::rgba(0.22, 0.23, 0.25, 0.92)
        }
    };
    let button = GlassButton::new(id, "", Rect::new(0.0, 0.0, visual_size, visual_size))
        .shape(GlassShape::Circle)
        .material(GlassMaterial::interactive())
        .chrome({
            chrome.pressed_overlay = liquid_glass_scene::Color::transparent();
            chrome
        });
    let button = if interactive {
        button.into_element_with_press_callbacks::<Message, Theme, Renderer>(
            on_action,
            on_press_start,
            on_press_cancel,
            on_press_end,
        )
    } else {
        button.into_element::<Message, Theme, Renderer>(on_action)
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

    components::glass_overlay(stack![button, centered(glyph, size)])
}

/// Renders a group of three traffic light controls (Close, Minimize, Expand).
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub fn control_group<'a, Message: Clone + 'static, Renderer>(
    ids: [GlassId; 3],
    size: f32,
    gap: f32,
    scheme: UiColorScheme,
    show_glyphs: bool,
    close_disabled: bool,
    interactive: bool,
    inactive: bool,
    hover_amount: f32,
    expand_behavior: WindowExpandBehavior,
    press_scales: [f32; 3],
    on_action: impl Fn(GlassId, ControlAction) -> Message + 'a,
    on_press_start: impl Fn(GlassId) -> Message + 'a,
    on_press_cancel: impl Fn(GlassId) -> Message + 'a,
    on_press_end: impl Fn(GlassId) -> Message + 'a,
    on_group_hover: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer
        + advanced_text::Renderer
        + advanced_svg::Renderer
        + GlassForegroundRenderer
        + 'static,
{
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
            on_action(ids[0], ControlAction::Close),
            Some(on_press_start(ids[0])),
            Some(on_press_cancel(ids[0])),
            Some(on_press_end(ids[0])),
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
            on_action(ids[1], ControlAction::Minimize),
            Some(on_press_start(ids[1])),
            Some(on_press_cancel(ids[1])),
            Some(on_press_end(ids[1])),
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
            on_action(ids[2], ControlAction::Expand),
            Some(on_press_start(ids[2])),
            Some(on_press_cancel(ids[2])),
            Some(on_press_end(ids[2])),
        ),
    ]
    .spacing(gap);
    let slop = control_hover_slop(size);
    let group_width = size * 3.0 + gap * 2.0;
    let tracking_area = container(controls)
        .width(Length::Fixed(group_width + slop * 2.0))
        .height(Length::Fixed(size + slop * 2.0))
        .padding(Padding {
            top: slop,
            right: slop,
            bottom: slop,
            left: slop,
        });
    mouse_area(tracking_area)
        .on_enter(on_group_hover(true))
        .on_exit(on_group_hover(false))
        .into()
}

/// Helper to position a control group with optical slop compensation.
#[must_use]
pub fn positioned_control_group<'a, Message: 'a, Renderer: advanced::Renderer + 'a>(
    content: Element<'a, Message, Theme, Renderer>,
    x: f32,
    y: f32,
    size: f32,
) -> Element<'a, Message, Theme, Renderer> {
    let slop = control_hover_slop(size);
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top: y - slop,
            right: 0.0,
            bottom: 0.0,
            left: x - slop,
        })
        .into()
}

/// Renders the complete, authentic Apple Liquid Glass traffic lights row.
#[allow(clippy::too_many_arguments)]
pub fn view_traffic_lights<'a, Message: Clone + 'static, Renderer>(
    state: &'a TrafficLightsState,
    scheme: UiColorScheme,
    inactive: bool,
    on_action: impl Fn(ControlAction) -> Message + 'a,
    on_press_start: impl Fn(usize) -> Message + 'a,
    on_press_cancel: impl Fn(usize) -> Message + 'a,
    on_press_end: impl Fn(usize) -> Message + 'a,
    on_group_hover: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer
        + advanced_text::Renderer
        + advanced_svg::Renderer
        + GlassForegroundRenderer
        + 'static,
{
    let press_scales = [
        state.press_springs[0].value(),
        state.press_springs[1].value(),
        state.press_springs[2].value(),
    ];
    control_group(
        WINDOW_CONTROL_NATIVE_IDS,
        WINDOW_CONTROL_NATIVE_SIZE,
        WINDOW_CONTROL_GAP,
        scheme,
        true,
        false,
        true,
        inactive,
        state.hover_progress,
        state.expand_behavior,
        press_scales,
        move |_id, action| on_action(action),
        move |id| on_press_start(slot_index(id).unwrap_or(0)),
        move |id| on_press_cancel(slot_index(id).unwrap_or(0)),
        move |id| on_press_end(slot_index(id).unwrap_or(0)),
        on_group_hover,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sizes_and_gaps() {
        assert_eq!(WINDOW_CONTROL_NATIVE_SIZE, 14.0);
        assert_eq!(WINDOW_CONTROL_GAP, 9.0);
        assert_eq!(WINDOW_CONTROL_LARGE_SIZE, 64.0);
    }

    #[test]
    fn test_glyph_sizes() {
        let close = window_control_glyph_size(ControlAction::Close, WINDOW_CONTROL_NATIVE_SIZE);
        let minimize = window_control_glyph_size(ControlAction::Minimize, WINDOW_CONTROL_NATIVE_SIZE);
        let expand = window_control_glyph_size(ControlAction::Expand, WINDOW_CONTROL_NATIVE_SIZE);

        assert_eq!(close, 7.0);
        assert_eq!(minimize, 8.0);
        assert!(minimize > close);
        assert!(close > expand);
    }

    #[test]
    fn test_glyph_colors() {
        let active_light =
            window_control_glyph_color(UiColorScheme::Light, ControlAction::Close, false, 1.0);
        let inactive_light =
            window_control_glyph_color(UiColorScheme::Light, ControlAction::Close, true, 0.0);

        assert!(inactive_light.r > active_light.r);
        assert_eq!(inactive_light.a, 0.0);
    }

    #[test]
    fn test_spring_dynamics() {
        let mut state = TrafficLightsState::new();
        assert!(!state.is_animating());

        state.on_press_start(0);
        assert!(state.is_animating());
        assert_eq!(state.press_targets[0], 1.0);
    }
}
