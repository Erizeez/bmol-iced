//! Semantic colors and glass materials shared by Iced integrations.

use iced::{Color as IcedColor, Theme};
use liquid_glass_scene::{Color as GlassColor, GlassMaterial};

/// The resolved light or dark tone used to render an application window.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiColorScheme {
    Light,
    #[default]
    Dark,
}

impl UiColorScheme {
    /// Resolves an Iced system theme mode to a concrete color scheme.
    #[must_use]
    pub const fn from_mode(mode: iced::theme::Mode) -> Self {
        match mode {
            iced::theme::Mode::Light => Self::Light,
            iced::theme::Mode::Dark | iced::theme::Mode::None => Self::Dark,
        }
    }
}

/// A semantic role whose material can vary with the active color scheme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlassRole {
    Toolbar,
    SearchField,
    FloatingControl,
}

/// Iced-side chrome drawn above a compositor-provided glass surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassChrome {
    pub border: GlassColor,
    pub hover_border: GlassColor,
    pub shadow: GlassColor,
    pub text: GlassColor,
    pub hover_overlay: GlassColor,
    pub pressed_overlay: GlassColor,
    pub shadow_offset_y: f32,
    pub shadow_blur: f32,
}

/// Semantic colors for regular, non-glass application UI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiPalette {
    pub window_background: IcedColor,
    pub sidebar_background: IcedColor,
    pub content_background: IcedColor,
    pub group_background: IcedColor,
    pub group_border: IcedColor,
    pub separator: IcedColor,
    pub text_primary: IcedColor,
    pub text_secondary: IcedColor,
    pub accent: IcedColor,
    pub selection: IcedColor,
    pub hover: IcedColor,
    pub shadow: IcedColor,
}

/// Theme object used by the UI library and compositor adapters.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiTheme {
    scheme: UiColorScheme,
}

impl UiTheme {
    #[must_use]
    pub const fn new(scheme: UiColorScheme) -> Self {
        Self { scheme }
    }

    #[must_use]
    pub const fn light() -> Self {
        Self::new(UiColorScheme::Light)
    }

    #[must_use]
    pub const fn dark() -> Self {
        Self::new(UiColorScheme::Dark)
    }

    #[must_use]
    pub const fn scheme(self) -> UiColorScheme {
        self.scheme
    }

    /// Derives the semantic theme from the Iced theme currently used to draw.
    #[must_use]
    pub fn from_iced(theme: &Theme) -> Self {
        if theme.extended_palette().is_dark { Self::dark() } else { Self::light() }
    }

    /// Returns the matching built-in Iced theme for standard controls.
    #[must_use]
    pub const fn iced_theme(self) -> Theme {
        match self.scheme {
            UiColorScheme::Light => Theme::Light,
            UiColorScheme::Dark => Theme::Dark,
        }
    }

    /// Returns semantic colors modelled after macOS split-view settings windows.
    #[must_use]
    pub fn palette(self) -> UiPalette {
        match self.scheme {
            UiColorScheme::Light => UiPalette {
                window_background: rgba(0.950, 0.950, 0.965, 1.0),
                sidebar_background: rgba(0.900, 0.900, 0.920, 0.72),
                content_background: rgba(0.955, 0.955, 0.970, 0.98),
                group_background: rgba(1.0, 1.0, 1.0, 0.94),
                group_border: rgba(0.0, 0.0, 0.0, 0.10),
                separator: rgba(0.0, 0.0, 0.0, 0.10),
                text_primary: rgba(0.08, 0.08, 0.09, 1.0),
                text_secondary: rgba(0.32, 0.32, 0.35, 1.0),
                accent: rgba(0.04, 0.42, 0.95, 1.0),
                selection: rgba(0.12, 0.46, 0.95, 0.18),
                hover: rgba(0.0, 0.0, 0.0, 0.055),
                shadow: rgba(0.0, 0.0, 0.0, 0.16),
            },
            UiColorScheme::Dark => UiPalette {
                window_background: rgba(0.105, 0.105, 0.115, 1.0),
                sidebar_background: rgba(0.145, 0.145, 0.155, 0.72),
                content_background: rgba(0.105, 0.105, 0.115, 0.98),
                group_background: rgba(0.175, 0.175, 0.190, 0.96),
                group_border: rgba(1.0, 1.0, 1.0, 0.085),
                separator: rgba(1.0, 1.0, 1.0, 0.085),
                text_primary: rgba(0.94, 0.94, 0.96, 1.0),
                text_secondary: rgba(0.66, 0.66, 0.69, 1.0),
                accent: rgba(0.24, 0.55, 1.0, 1.0),
                selection: rgba(0.20, 0.48, 0.95, 0.30),
                hover: rgba(1.0, 1.0, 1.0, 0.065),
                shadow: rgba(0.0, 0.0, 0.0, 0.34),
            },
        }
    }

    /// Builds a scheme-aware material for a selective glass surface.
    #[must_use]
    pub fn glass_material(self, role: GlassRole) -> GlassMaterial {
        let (blur_radius, tint) = match (self.scheme, role) {
            (UiColorScheme::Light, GlassRole::Toolbar) => {
                (16.0, GlassColor::rgba(1.0, 1.0, 1.0, 0.08))
            }
            (UiColorScheme::Light, GlassRole::SearchField) => {
                (12.0, GlassColor::rgba(1.0, 1.0, 1.0, 0.12))
            }
            (UiColorScheme::Light, GlassRole::FloatingControl) => {
                (9.0, GlassColor::rgba(1.0, 1.0, 1.0, 0.14))
            }
            (UiColorScheme::Dark, GlassRole::Toolbar) => {
                (16.0, GlassColor::rgba(0.10, 0.13, 0.20, 0.10))
            }
            (UiColorScheme::Dark, GlassRole::SearchField) => {
                (12.0, GlassColor::rgba(0.14, 0.18, 0.27, 0.12))
            }
            (UiColorScheme::Dark, GlassRole::FloatingControl) => {
                (9.0, GlassColor::rgba(0.17, 0.22, 0.34, 0.13))
            }
        };

        let mut material = GlassMaterial::clear();
        material.blur.radius = blur_radius;
        material.tint = tint;
        material.refraction.thickness = 0.20;
        material.refraction.index = 1.40;
        material.dispersion.strength = 0.07;
        material.fresnel.range = 0.75;
        material.fresnel.hardness = 0.20;
        material.fresnel.strength = 0.20;
        material.opacity = 1.0;
        material
    }

    /// Builds the Iced-side border, text, state overlay, and shadow colors.
    #[must_use]
    pub fn glass_chrome(self, role: GlassRole) -> GlassChrome {
        let (border, hover_border, shadow, text, hover_overlay, pressed_overlay) = match self.scheme
        {
            UiColorScheme::Light => (
                GlassColor::rgba(0.0, 0.0, 0.0, 0.10),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.20),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.14),
                GlassColor::rgba(0.08, 0.08, 0.09, 1.0),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.04),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.09),
            ),
            UiColorScheme::Dark => (
                GlassColor::rgba(1.0, 1.0, 1.0, 0.16),
                GlassColor::rgba(1.0, 1.0, 1.0, 0.30),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.30),
                GlassColor::rgba(0.95, 0.95, 0.97, 1.0),
                GlassColor::rgba(1.0, 1.0, 1.0, 0.055),
                GlassColor::rgba(1.0, 1.0, 1.0, 0.11),
            ),
        };
        let (shadow_offset_y, shadow_blur) = match role {
            GlassRole::Toolbar => (2.0, 10.0),
            GlassRole::SearchField => (3.0, 9.0),
            GlassRole::FloatingControl => (4.0, 10.0),
        };
        GlassChrome {
            border,
            hover_border,
            shadow,
            text,
            hover_overlay,
            pressed_overlay,
            shadow_offset_y,
            shadow_blur,
        }
    }

    /// Returns chrome for an Iced widget layered over a real shader surface.
    /// The shader owns the edge highlight and shadow, so the overlay only
    /// keeps text and interaction-state fills.
    #[must_use]
    pub fn compositor_chrome(self, role: GlassRole) -> GlassChrome {
        let mut chrome = self.glass_chrome(role);
        chrome.border = GlassColor::transparent();
        chrome.hover_border = GlassColor::transparent();
        chrome.shadow = GlassColor::transparent();
        if role == GlassRole::Toolbar {
            chrome.hover_overlay = GlassColor::transparent();
            chrome.pressed_overlay = GlassColor::transparent();
        }
        chrome
    }
}

impl Default for GlassChrome {
    fn default() -> Self {
        UiTheme::dark().glass_chrome(GlassRole::FloatingControl)
    }
}

fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> IcedColor {
    IcedColor::from_rgba(red, green, blue, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_palettes_have_opposite_tones() {
        let light = UiTheme::light().palette();
        let dark = UiTheme::dark().palette();

        assert!(light.content_background.r > dark.content_background.r);
        assert!(light.text_primary.r < dark.text_primary.r);
    }

    #[test]
    fn glass_roles_use_different_blur_radii() {
        let theme = UiTheme::dark();

        assert!(
            theme.glass_material(GlassRole::Toolbar).blur.radius
                > theme.glass_material(GlassRole::FloatingControl).blur.radius
        );
    }

    #[test]
    fn compositor_chrome_leaves_edges_to_the_shader() {
        let chrome = UiTheme::light().compositor_chrome(GlassRole::SearchField);

        assert!(chrome.border.a.abs() < f32::EPSILON);
        assert!(chrome.hover_border.a.abs() < f32::EPSILON);
        assert!(chrome.shadow.a.abs() < f32::EPSILON);
    }
}
