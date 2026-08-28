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

/// Window configuration that remains independent from any specific windowing backend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowConfig {
    pub transparent: bool,
    pub decorations: bool,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { transparent: false, decorations: true, resizable: true }
    }
}
