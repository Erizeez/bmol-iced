//! Platform boundary for window and display concerns.

#![deny(unsafe_code)]

/// Display scale information passed into the renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayScale {
    pub scale_factor: f32,
}

impl Default for DisplayScale {
    fn default() -> Self {
        Self { scale_factor: 1.0 }
    }
}

/// Where a transparent surface obtains the pixels behind the application.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BackdropSource {
    /// The application owns an opaque background.
    #[default]
    Application,
    /// The operating-system compositor owns the pixels behind the window.
    Desktop,
    /// A caller-provided texture, commonly used for previews or tests.
    Texture,
}

/// Window configuration that remains independent from any specific windowing backend.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct WindowConfig {
    pub transparent: bool,
    pub blur: bool,
    pub backdrop: BackdropSource,
    pub decorations: bool,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            transparent: false,
            blur: false,
            backdrop: BackdropSource::Application,
            decorations: true,
            resizable: true,
        }
    }
}

impl WindowConfig {
    /// Configures a transparent window whose backdrop is supplied by the OS.
    #[must_use]
    pub const fn desktop_backdrop() -> Self {
        Self {
            transparent: true,
            blur: true,
            backdrop: BackdropSource::Desktop,
            decorations: true,
            resizable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_backdrop_requires_transparency_and_blur() {
        let config = WindowConfig::desktop_backdrop();

        assert!(config.transparent);
        assert!(config.blur);
        assert_eq!(config.backdrop, BackdropSource::Desktop);
    }
}
