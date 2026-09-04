//! Platform boundary for window and display concerns.

#![deny(unsafe_code)]

use std::fmt;

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

/// Physical pixel dimensions of a desktop backdrop frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackdropSize {
    pub width: u32,
    pub height: u32,
}

impl BackdropSize {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// A request for the desktop pixels behind one transparent application
/// surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropRequest {
    /// The top-left physical screen coordinate of the requested region.
    pub origin: [i32; 2],
    pub size: BackdropSize,
    pub scale_factor: f32,
}

impl BackdropRequest {
    #[must_use]
    pub const fn new(origin: [i32; 2], size: BackdropSize, scale_factor: f32) -> Self {
        Self { origin, size, scale_factor }
    }
}

/// An RGBA8 desktop frame supplied to the compositor.
///
/// `stride` is measured in bytes and may be larger than `width * 4`, which
/// accommodates native capture APIs that pad each row. The renderer strips
/// row padding before uploading the frame to a filterable GPU texture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackdropFrame {
    size: BackdropSize,
    stride: u32,
    rgba8: Vec<u8>,
}

impl BackdropFrame {
    /// Creates a frame from an RGBA8 buffer.
    ///
    /// # Errors
    ///
    /// Returns [`BackdropFrameError`] when dimensions, stride, or buffer
    /// length do not describe a complete frame.
    pub fn new(
        size: BackdropSize,
        stride: u32,
        rgba8: Vec<u8>,
    ) -> Result<Self, BackdropFrameError> {
        if !size.is_valid() {
            return Err(BackdropFrameError::InvalidSize);
        }
        let minimum_stride = size.width.checked_mul(4).ok_or(BackdropFrameError::InvalidSize)?;
        if stride < minimum_stride {
            return Err(BackdropFrameError::StrideTooSmall {
                minimum: minimum_stride,
                actual: stride,
            });
        }
        let expected = usize::try_from(stride)
            .ok()
            .and_then(|stride| {
                usize::try_from(size.height).ok().and_then(|height| stride.checked_mul(height))
            })
            .ok_or(BackdropFrameError::InvalidSize)?;
        if rgba8.len() != expected {
            return Err(BackdropFrameError::BufferLength { expected, actual: rgba8.len() });
        }
        Ok(Self { size, stride, rgba8 })
    }

    /// Creates a tightly packed RGBA8 frame.
    ///
    /// # Errors
    ///
    /// Returns [`BackdropFrameError`] when the dimensions are invalid or the
    /// buffer is not exactly `width * height * 4` bytes long.
    pub fn packed(size: BackdropSize, rgba8: Vec<u8>) -> Result<Self, BackdropFrameError> {
        let stride = size.width.checked_mul(4).ok_or(BackdropFrameError::InvalidSize)?;
        Self::new(size, stride, rgba8)
    }

    #[must_use]
    pub const fn size(&self) -> BackdropSize {
        self.size
    }

    #[must_use]
    pub const fn stride(&self) -> u32 {
        self.stride
    }

    #[must_use]
    pub fn rgba8(&self) -> &[u8] {
        &self.rgba8
    }
}

/// Validation failures for a [`BackdropFrame`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackdropFrameError {
    InvalidSize,
    StrideTooSmall { minimum: u32, actual: u32 },
    BufferLength { expected: usize, actual: usize },
}

impl fmt::Display for BackdropFrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize => {
                write!(formatter, "backdrop frame dimensions overflow or are zero")
            }
            Self::StrideTooSmall { minimum, actual } => {
                write!(
                    formatter,
                    "backdrop frame stride {actual} is smaller than the minimum {minimum}"
                )
            }
            Self::BufferLength { expected, actual } => {
                write!(formatter, "backdrop frame has {actual} bytes; expected {expected}")
            }
        }
    }
}

impl std::error::Error for BackdropFrameError {}

/// Errors returned by a platform desktop backdrop provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackdropError {
    Unsupported,
    CaptureFailed(String),
    InvalidFrame(BackdropFrameError),
}

impl fmt::Display for BackdropError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(formatter, "desktop backdrop capture is unsupported"),
            Self::CaptureFailed(error) => {
                write!(formatter, "desktop backdrop capture failed: {error}")
            }
            Self::InvalidFrame(error) => {
                write!(formatter, "invalid desktop backdrop frame: {error}")
            }
        }
    }
}

impl std::error::Error for BackdropError {}

/// Platform hook for supplying the pixels behind a transparent surface.
///
/// The trait intentionally contains no OS or windowing types. Platform
/// adapters can implement it with native compositor APIs or an optional
/// screen-capture backend, while the renderer consumes the same frame model.
pub trait DesktopBackdropProvider: fmt::Debug {
    /// Captures the requested physical screen region.
    ///
    /// # Errors
    ///
    /// Returns [`BackdropError::Unsupported`] when the platform cannot
    /// provide desktop pixels, or another [`BackdropError`] when capture
    /// fails or returns an invalid frame.
    fn capture(&mut self, request: BackdropRequest) -> Result<BackdropFrame, BackdropError>;
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

/// Controls how a sidebar's frosted background extends relative to the
/// window. This is a semantic platform boundary so each backend can keep the
/// sidebar background behind window chrome while sharing the same renderer
/// contract.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SidebarBackgroundExtension {
    /// Extend from the window edges, including the area behind window chrome.
    #[default]
    WindowEdges,
    /// Keep the background inside the application's content bounds.
    ContentBounds,
}

impl SidebarBackgroundExtension {
    /// Resolves the physical-pixel region for a sidebar background.
    #[must_use]
    pub const fn region(
        self,
        window_width: u32,
        window_height: u32,
        sidebar_width: u32,
        content_start_y: u32,
    ) -> (u32, u32, u32, u32) {
        let sidebar_width = if sidebar_width < window_width { sidebar_width } else { window_width };
        match self {
            Self::WindowEdges => (0, 0, sidebar_width, window_height),
            Self::ContentBounds => (
                0,
                if content_start_y < window_height { content_start_y } else { window_height },
                sidebar_width,
                window_height.saturating_sub(content_start_y),
            ),
        }
    }
}

/// Shared style contract for a frosted sidebar background.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidebarBackgroundConfig {
    pub extension: SidebarBackgroundExtension,
    /// Blur radius in physical pixels.
    pub blur_radius: u32,
    /// Linear-light RGBA tint applied over the blurred backdrop.
    pub tint: [f32; 4],
}

impl SidebarBackgroundConfig {
    /// Creates a sidebar that extends behind the window's top chrome.
    #[must_use]
    pub const fn window_edges(blur_radius: u32, tint: [f32; 4]) -> Self {
        Self { extension: SidebarBackgroundExtension::WindowEdges, blur_radius, tint }
    }

    /// Resolves the configured physical-pixel region.
    #[must_use]
    pub const fn region(
        self,
        window_width: u32,
        window_height: u32,
        sidebar_width: u32,
        content_start_y: u32,
    ) -> (u32, u32, u32, u32) {
        self.extension.region(window_width, window_height, sidebar_width, content_start_y)
    }
}

impl Default for SidebarBackgroundConfig {
    fn default() -> Self {
        Self::window_edges(64, [0.90, 0.91, 0.92, 0.84])
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

    #[test]
    fn backdrop_frame_accepts_padded_rows() {
        let frame = BackdropFrame::new(BackdropSize::new(2, 2), 12, vec![0; 24])
            .expect("padded rows are valid");

        assert_eq!(frame.size(), BackdropSize::new(2, 2));
        assert_eq!(frame.stride(), 12);
        assert_eq!(frame.rgba8().len(), 24);
    }

    #[test]
    fn backdrop_frame_rejects_incomplete_pixels() {
        let error = BackdropFrame::packed(BackdropSize::new(2, 2), vec![0; 15])
            .expect_err("incomplete frames must be rejected");

        assert_eq!(error, BackdropFrameError::BufferLength { expected: 16, actual: 15 });
    }

    #[test]
    fn sidebar_background_defaults_to_window_edges() {
        let config = SidebarBackgroundConfig::default();

        assert_eq!(config.extension, SidebarBackgroundExtension::WindowEdges);
        assert_eq!(config.blur_radius, 64);
        assert_eq!(config.region(1_000, 700, 232, 48), (0, 0, 232, 700));
    }

    #[test]
    fn sidebar_background_region_is_clamped_to_the_window() {
        let config = SidebarBackgroundConfig {
            extension: SidebarBackgroundExtension::ContentBounds,
            blur_radius: 32,
            tint: [1.0; 4],
        };

        assert_eq!(config.region(100, 80, 140, 12), (0, 12, 100, 68));
    }
}
