//! Semantic colors and glass materials shared by Iced integrations.

use iced::{Color as IcedColor, Theme};
use liquid_glass_scene::{Color as GlassColor, GlassMaterial, GlassShape, ShadowStyle};

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
    /// A full-height split-view surface that reveals and heavily blurs the
    /// desktop backdrop.
    Sidebar,
    Toolbar,
    InputField,
    /// Compatibility role for search-specific input fields.
    SearchField,
    FloatingControl,
}

/// Iced-side chrome drawn above a compositor-provided glass surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassChrome {
    pub border: GlassColor,
    pub hover_border: GlassColor,
    pub divider: GlassColor,
    pub shadow: GlassColor,
    pub text: GlassColor,
    pub disabled_text: GlassColor,
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
    /// De-emphasized text such as disclosure chevrons and placeholders.
    pub text_tertiary: IcedColor,
    pub accent: IcedColor,
    /// Accent-tinted selection used inside lists and text fields.
    pub selection: IcedColor,
    /// Neutral pill drawn behind the selected sidebar item.
    pub sidebar_selection: IcedColor,
    pub hover: IcedColor,
    /// The track of an off toggle switch.
    pub control_track_off: IcedColor,
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
                content_background: rgba(0.955, 0.955, 0.970, 1.0),
                group_background: rgba(1.0, 1.0, 1.0, 0.94),
                group_border: rgba(0.0, 0.0, 0.0, 0.10),
                separator: rgba(0.0, 0.0, 0.0, 0.10),
                text_primary: rgba(0.08, 0.08, 0.09, 1.0),
                text_secondary: rgba(0.32, 0.32, 0.35, 1.0),
                text_tertiary: rgba(0.0, 0.0, 0.0, 0.26),
                accent: rgba(0.04, 0.42, 0.95, 1.0),
                selection: rgba(0.12, 0.46, 0.95, 0.18),
                sidebar_selection: rgba(0.0, 0.0, 0.0, 0.085),
                hover: rgba(0.0, 0.0, 0.0, 0.055),
                control_track_off: rgba(0.0, 0.0, 0.0, 0.10),
                shadow: rgba(0.0, 0.0, 0.0, 0.16),
            },
            UiColorScheme::Dark => UiPalette {
                window_background: rgba(0.105, 0.105, 0.115, 1.0),
                sidebar_background: rgba(0.145, 0.145, 0.155, 0.72),
                content_background: rgba(0.105, 0.105, 0.115, 1.0),
                group_background: rgba(0.175, 0.175, 0.190, 0.96),
                group_border: rgba(1.0, 1.0, 1.0, 0.085),
                separator: rgba(1.0, 1.0, 1.0, 0.085),
                text_primary: rgba(0.94, 0.94, 0.96, 1.0),
                text_secondary: rgba(0.66, 0.66, 0.69, 1.0),
                text_tertiary: rgba(1.0, 1.0, 1.0, 0.25),
                accent: rgba(0.24, 0.55, 1.0, 1.0),
                selection: rgba(0.20, 0.48, 0.95, 0.30),
                sidebar_selection: rgba(1.0, 1.0, 1.0, 0.12),
                hover: rgba(1.0, 1.0, 1.0, 0.065),
                control_track_off: rgba(1.0, 1.0, 1.0, 0.17),
                shadow: rgba(0.0, 0.0, 0.0, 0.34),
            },
        }
    }

    /// Builds a scheme-aware material for a selective glass surface.
    #[must_use]
    pub fn glass_material(self, role: GlassRole) -> GlassMaterial {
        let (blur_radius, tint, whiteness) = match (self.scheme, role) {
            (UiColorScheme::Light, GlassRole::Sidebar) => {
                (32.0, GlassColor::rgba(1.0, 1.0, 1.0, 0.18), 0.55)
            }
            (UiColorScheme::Light, GlassRole::Toolbar) => {
                (8.0, GlassColor::rgba(1.0, 1.0, 1.0, 0.04), 0.04)
            }
            (UiColorScheme::Light, GlassRole::InputField | GlassRole::SearchField) => {
                (7.0, GlassColor::rgba(1.0, 1.0, 1.0, 0.06), 0.14)
            }
            (UiColorScheme::Light, GlassRole::FloatingControl) => {
                // Navigation content is drawn inside this capsule. Keep its
                // interior nearly sharp while retaining the refractive edge.
                (2.0, GlassColor::rgba(1.0, 1.0, 1.0, 0.06), 0.06)
            }
            (UiColorScheme::Dark, GlassRole::Sidebar) => {
                (32.0, GlassColor::rgba(0.12, 0.16, 0.25, 0.14), 0.36)
            }
            (UiColorScheme::Dark, GlassRole::Toolbar) => {
                (8.0, GlassColor::rgba(0.10, 0.13, 0.20, 0.08), 0.025)
            }
            (UiColorScheme::Dark, GlassRole::InputField | GlassRole::SearchField) => {
                (7.0, GlassColor::rgba(0.14, 0.18, 0.27, 0.09), 0.10)
            }
            (UiColorScheme::Dark, GlassRole::FloatingControl) => {
                (2.0, GlassColor::rgba(0.17, 0.22, 0.34, 0.10), 0.05)
            }
        };

        let mut material = GlassMaterial::clear();
        material.blur.radius = blur_radius;
        material.tint = tint;
        material.whiteness = whiteness;
        material.refraction.thickness = 0.20;
        material.refraction.index = 1.40;
        material.refraction.strength = 0.70;
        material.dispersion.strength = 0.07;
        material.fresnel.range = 0.75;
        material.fresnel.hardness = 0.20;
        material.fresnel.strength = 0.20;
        if role == GlassRole::Sidebar {
            // The sidebar is a broad system surface, not a floating optical
            // object. Keep only the Gaussian backdrop blur and neutral wash;
            // the reference shader uses zero refraction as its flat-blur
            // mode, which also suppresses Fresnel and glare in the fragment
            // path.
            material.refraction.strength = 0.0;
            material.dispersion.strength = 0.0;
            material.fresnel.strength = 0.0;
        }
        material.opacity = match role {
            GlassRole::Sidebar => match self.scheme {
                UiColorScheme::Light => 0.84,
                UiColorScheme::Dark => 0.76,
            },
            // Keep the neutral layer present, but leave enough of the
            // compositor backdrop visible to read as glass on a transparent
            // desktop surface. The input remains the whitest control; the
            // navigation capsule is lighter and more transparent.
            GlassRole::Toolbar => 0.54,
            GlassRole::InputField | GlassRole::SearchField => 0.64,
            GlassRole::FloatingControl => 0.72,
        };
        material.shadow = match role {
            GlassRole::FloatingControl => ShadowStyle::elevated(),
            GlassRole::Sidebar
            | GlassRole::Toolbar
            | GlassRole::InputField
            | GlassRole::SearchField => ShadowStyle::subtle(),
        };
        material
    }

    /// Returns the default geometry associated with a semantic glass role.
    #[must_use]
    pub const fn glass_shape(self, role: GlassRole) -> GlassShape {
        match role {
            GlassRole::Sidebar | GlassRole::Toolbar => GlassShape::RoundedRect { radius: 0.0 },
            GlassRole::InputField | GlassRole::SearchField | GlassRole::FloatingControl => {
                GlassShape::Capsule
            }
        }
    }

    /// Builds the Iced-side border, text, state overlay, and shadow colors.
    #[must_use]
    pub fn glass_chrome(self, role: GlassRole) -> GlassChrome {
        let (
            border,
            hover_border,
            divider,
            shadow,
            text,
            disabled_text,
            hover_overlay,
            pressed_overlay,
        ) = match self.scheme {
            UiColorScheme::Light => (
                GlassColor::rgba(0.0, 0.0, 0.0, 0.10),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.20),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.14),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.14),
                GlassColor::rgba(0.08, 0.08, 0.09, 1.0),
                GlassColor::rgba(0.08, 0.08, 0.09, 0.34),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.04),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.09),
            ),
            UiColorScheme::Dark => (
                GlassColor::rgba(1.0, 1.0, 1.0, 0.16),
                GlassColor::rgba(1.0, 1.0, 1.0, 0.30),
                GlassColor::rgba(1.0, 1.0, 1.0, 0.18),
                GlassColor::rgba(0.0, 0.0, 0.0, 0.30),
                GlassColor::rgba(0.95, 0.95, 0.97, 1.0),
                GlassColor::rgba(0.95, 0.95, 0.97, 0.34),
                GlassColor::rgba(1.0, 1.0, 1.0, 0.055),
                GlassColor::rgba(1.0, 1.0, 1.0, 0.11),
            ),
        };
        let (shadow_offset_y, shadow_blur) = match role {
            GlassRole::Sidebar => (1.0, 14.0),
            GlassRole::Toolbar => (2.0, 10.0),
            GlassRole::InputField | GlassRole::SearchField => (3.0, 9.0),
            GlassRole::FloatingControl => (4.0, 10.0),
        };
        GlassChrome {
            border,
            hover_border,
            divider,
            shadow,
            text,
            disabled_text,
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
        let input = theme.glass_material(GlassRole::InputField);
        let button = theme.glass_material(GlassRole::FloatingControl);

        assert!(theme.glass_material(GlassRole::Toolbar).blur.radius > button.blur.radius);
        assert!(input.blur.radius > button.blur.radius);
        assert!(input.whiteness > button.whiteness);
    }

    #[test]
    fn sidebar_is_whiter_and_more_blurred_than_toolbar() {
        let theme = UiTheme::light();
        let sidebar = theme.glass_material(GlassRole::Sidebar);
        let toolbar = theme.glass_material(GlassRole::Toolbar);

        assert!(sidebar.blur.radius > toolbar.blur.radius);
        assert!(sidebar.whiteness > toolbar.whiteness);
        assert!(sidebar.opacity > toolbar.opacity);
        assert!(sidebar.shadow.factor > 0.0);
        assert_eq!(sidebar.refraction.strength, 0.0);
        assert_eq!(sidebar.dispersion.strength, 0.0);
        assert_eq!(sidebar.fresnel.strength, 0.0);
    }

    #[test]
    fn input_fields_default_to_continuous_capsules() {
        let theme = UiTheme::light();

        assert_eq!(theme.glass_shape(GlassRole::InputField), GlassShape::Capsule);
    }

    #[test]
    fn compositor_chrome_leaves_edges_to_the_shader() {
        let chrome = UiTheme::light().compositor_chrome(GlassRole::SearchField);

        assert!(chrome.border.a.abs() < f32::EPSILON);
        assert!(chrome.hover_border.a.abs() < f32::EPSILON);
        assert!(chrome.shadow.a.abs() < f32::EPSILON);
    }

    #[test]
    fn disabled_glass_content_is_visually_muted() {
        let light = UiTheme::light().glass_chrome(GlassRole::FloatingControl);
        let dark = UiTheme::dark().glass_chrome(GlassRole::FloatingControl);

        assert!(light.disabled_text.a < light.text.a);
        assert!(dark.disabled_text.a < dark.text.a);
    }
}
