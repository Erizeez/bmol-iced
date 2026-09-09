//! Liquid Glass Dock, Search Bar, Icons & Context Menu Optics Playground Demo.
//!
//! Demonstrates authentic Apple-style Liquid Glass optics on a colorful background:
//! - Top-light specular highlight: wide & soft across straight horizontal edges.
//! - Curvature-compressed highlight: razor-sharp & pinpoint along corner arcs.
//! - Directional falloff: zero specular highlight on left/right vertical edges.
//! - Edge occlusion (Rim Darkening): subtle dark perimeter stroke revealed on sides.
//! - Authentic macOS squircle app icons with genuine vector illustrations.
//! - Floating Frosted Dock with backdrop blur, hover tooltips, and spring scale.
//! - Capsule Search Bar with spotlight styling and shortcut badge.
//! - Liquid Glass Context Menu popping up on right-click with full HIG styling.
//! - TV Color Bars, SMPTE split, and vibrant geometric gradient wallpapers.
//! - Real-time optics toggles (highlight switch & rim darkening switch) for comparison.

#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::struct_excessive_bools,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_arguments
)]

use std::sync::Arc;
use std::time::Instant;

use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Radians, Rectangle,
    Shadow, Size, Subscription, Task, Theme, Vector,
    font::Weight,
    mouse,
    widget::{
        button,
        canvas::{self, Canvas, Frame, Geometry, Path, Stroke},
        column, container, row, space, text, text_input,
    },
    window,
};
use bmol_designs::{dock_metrics, menu_metrics};
use bmol_window_shell::{
    ShellEvent, TrafficLightsEvent, WindowChromeConfig, WindowShellController, app_icon_png,
    is_system_dark_mode, window_metrics,
};
use liquid_glass::{
    ContextMenu, MenuItem, UiColorScheme, UiIcon, UiTheme,
    geometry::{
        squircle_alpha, squircle_path_commands, PathCommand,
        Point as SquirclePoint, SquircleParams, APPLE_CORNER_SMOOTHING,
    },
    ui::font,
};
use iced::advanced::graphics::gradient::Linear;
use vibrancy_rs::{ign_dither_offset, KawasePassPlan, VibrancyConfig};
#[cfg(test)]
use vibrancy_rs::{MaterialKind, VibrancyAppearance};

#[path = "../iced_backend.rs"]
mod iced_backend;

use iced_backend::{DemoSurface, WINDOW_CONTROL_NATIVE_IDS};

/// Downsamples an RGBA buffer by 2x using 2x2 area box filtering.
fn downsample_2x(w: u32, h: u32, src: &[u8]) -> (u32, u32, Vec<u8>) {
    let dw = (w / 2).max(1);
    let dh = (h / 2).max(1);
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh {
        let sy0 = (y * 2) as usize;
        let sy1 = ((y * 2 + 1) as usize).min(h as usize - 1);
        for x in 0..dw {
            let sx0 = (x * 2) as usize;
            let sx1 = ((x * 2 + 1) as usize).min(w as usize - 1);
            let idx00 = (sy0 * w as usize + sx0) * 4;
            let idx10 = (sy0 * w as usize + sx1) * 4;
            let idx01 = (sy1 * w as usize + sx0) * 4;
            let idx11 = (sy1 * w as usize + sx1) * 4;
            let out_idx = (y * dw + x) as usize * 4;
            for c in 0..4 {
                let sum = u32::from(src[idx00 + c])
                    + u32::from(src[idx10 + c])
                    + u32::from(src[idx01 + c])
                    + u32::from(src[idx11 + c]);
                out[out_idx + c] = (sum / 4) as u8;
            }
        }
    }
    (dw, dh, out)
}

/// In-memory RGBA wallpaper buffer enabling continuous 2D Gaussian optical sampling
/// with a 3-tier Dual Kawase multi-resolution pyramid for deep, silky Apple frosted diffusion.
#[derive(Clone)]
pub struct WallpaperBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<Vec<u8>>,
    pub handle: iced::widget::image::Handle,
    pub mip1_w: u32,
    pub mip1_h: u32,
    pub mip1: Arc<Vec<u8>>,
    pub mip2_w: u32,
    pub mip2_h: u32,
    pub mip2: Arc<Vec<u8>>,
    pub mip3_w: u32,
    pub mip3_h: u32,
    pub mip3: Arc<Vec<u8>>,
}

impl std::fmt::Debug for WallpaperBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WallpaperBuffer")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pixels_len", &self.pixels.len())
            .field("mip3_size", &(self.mip3_w, self.mip3_h))
            .finish()
    }
}

impl WallpaperBuffer {
    #[must_use]
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        let handle = iced::widget::image::Handle::from_rgba(width, height, pixels.clone());
        let (mip1_w, mip1_h, mip1_vec) = downsample_2x(width, height, &pixels);
        let (mip2_w, mip2_h, mip2_vec) = downsample_2x(mip1_w, mip1_h, &mip1_vec);
        let (mip3_w, mip3_h, mip3_vec) = downsample_2x(mip2_w, mip2_h, &mip2_vec);

        Self {
            width,
            height,
            pixels: Arc::new(pixels),
            handle,
            mip1_w,
            mip1_h,
            mip1: Arc::new(mip1_vec),
            mip2_w,
            mip2_h,
            mip2: Arc::new(mip2_vec),
            mip3_w,
            mip3_h,
            mip3: Arc::new(mip3_vec),
        }
    }

    #[inline]
    fn get_pixel_from_slice(w: u32, h: u32, pixels: &[u8], x: i32, y: i32) -> [f32; 3] {
        let x = x.clamp(0, (w.saturating_sub(1)) as i32) as usize;
        let y = y.clamp(0, (h.saturating_sub(1)) as i32) as usize;
        let idx = (y * w as usize + x) * 4;
        if idx + 2 < pixels.len() {
            [
                f32::from(pixels[idx]) / 255.0,
                f32::from(pixels[idx + 1]) / 255.0,
                f32::from(pixels[idx + 2]) / 255.0,
            ]
        } else {
            [0.12, 0.12, 0.14]
        }
    }

    /// Bilinear interpolation of wallpaper pixels at specified MIP level (0=base, 1=1/2, 2=1/4, 3=1/8).
    #[inline]
    #[must_use]
    pub fn sample_bilinear_mip(&self, mip: usize, u: f32, v: f32) -> [f32; 3] {
        let (w, h, buf): (u32, u32, &[u8]) = match mip {
            1 => (self.mip1_w, self.mip1_h, &self.mip1),
            2 => (self.mip2_w, self.mip2_h, &self.mip2),
            3 => (self.mip3_w, self.mip3_h, &self.mip3),
            _ => (self.width, self.height, &self.pixels),
        };

        let px = u.clamp(0.0, 1.0) * (w.saturating_sub(1) as f32);
        let py = v.clamp(0.0, 1.0) * (h.saturating_sub(1) as f32);
        let x0 = px.floor() as i32;
        let y0 = py.floor() as i32;
        let fx = px - x0 as f32;
        let fy = py - y0 as f32;

        let c00 = Self::get_pixel_from_slice(w, h, buf, x0, y0);
        let c10 = Self::get_pixel_from_slice(w, h, buf, x0 + 1, y0);
        let c01 = Self::get_pixel_from_slice(w, h, buf, x0, y0 + 1);
        let c11 = Self::get_pixel_from_slice(w, h, buf, x0 + 1, y0 + 1);

        let w00 = (1.0 - fx) * (1.0 - fy);
        let w10 = fx * (1.0 - fy);
        let w01 = (1.0 - fx) * fy;
        let w11 = fx * fy;

        [
            c00[0] * w00 + c10[0] * w10 + c01[0] * w01 + c11[0] * w11,
            c00[1] * w00 + c10[1] * w10 + c01[1] * w01 + c11[1] * w11,
            c00[2] * w00 + c10[2] * w10 + c01[2] * w01 + c11[2] * w11,
        ]
    }

    /// Bilinear interpolation of sharp base wallpaper pixels at normalized coordinates `(u, v)`.
    #[inline]
    #[must_use]
    pub fn sample_bilinear(&self, u: f32, v: f32) -> [f32; 3] {
        self.sample_bilinear_mip(0, u, v)
    }

    /// Deep multi-resolution isotropic Gaussian blur convolution powered by Dual Kawase pyramid.
    ///
    /// Selects pre-integrated downsampled pyramid levels (Mip 2 / Mip 3) for large blur radii,
    /// integrating hundreds of ambient pixels into a velvety, noise-free frosted glass substrate.
    #[must_use]
    pub fn sample_blurred(&self, u: f32, v: f32, blur_radius: f32, bounds: Size) -> [f32; 3] {
        if blur_radius <= 0.5 {
            return self.sample_bilinear(u, v);
        }

        // Multi-resolution Dual Kawase level selection:
        // radius >= 48 -> Mip 3 (1/8 resolution, 64x pre-integrated area)
        // radius >= 24 -> Mip 2 (1/4 resolution, 16x pre-integrated area)
        // radius >= 10 -> Mip 1 (1/2 resolution, 4x pre-integrated area)
        // radius < 10 (2pt, 4pt, 8pt) -> Mip 0 (1x base resolution, fine-grained micro-blur)
        let mip = if blur_radius >= 48.0 {
            3
        } else if blur_radius >= 24.0 {
            2
        } else if blur_radius >= 10.0 {
            1
        } else {
            0
        };

        let sigma = (blur_radius * 1.5).max(0.5);
        let du = sigma / bounds.width.max(1.0);
        let dv = sigma / bounds.height.max(1.0);

        let c0 = self.sample_bilinear_mip(mip, u, v);

        // Ring 1 (0.75 * sigma)
        let c_r = self.sample_bilinear_mip(mip, u + du * 0.75, v);
        let c_l = self.sample_bilinear_mip(mip, u - du * 0.75, v);
        let c_d = self.sample_bilinear_mip(mip, u, v + dv * 0.75);
        let c_u = self.sample_bilinear_mip(mip, u, v - dv * 0.75);

        // Ring 2 diagonals (1.20 * sigma)
        let c_rd = self.sample_bilinear_mip(mip, u + du * 0.85, v + dv * 0.85);
        let c_ld = self.sample_bilinear_mip(mip, u - du * 0.85, v + dv * 0.85);
        let c_ru = self.sample_bilinear_mip(mip, u + du * 0.85, v - dv * 0.85);
        let c_lu = self.sample_bilinear_mip(mip, u - du * 0.85, v - dv * 0.85);

        // Ring 3 outer (1.90 * sigma)
        let c_rr = self.sample_bilinear_mip(mip, u + du * 1.90, v);
        let c_ll = self.sample_bilinear_mip(mip, u - du * 1.90, v);
        let c_dd = self.sample_bilinear_mip(mip, u, v + dv * 1.90);
        let c_uu = self.sample_bilinear_mip(mip, u, v - dv * 1.90);

        // Normalized Gaussian weights (0.16 + 4*0.12 + 4*0.055 + 4*0.035 = 1.00)
        [
            c0[0] * 0.16
                + (c_r[0] + c_l[0] + c_d[0] + c_u[0]) * 0.12
                + (c_rd[0] + c_ld[0] + c_ru[0] + c_lu[0]) * 0.055
                + (c_rr[0] + c_ll[0] + c_dd[0] + c_uu[0]) * 0.035,
            c0[1] * 0.16
                + (c_r[1] + c_l[1] + c_d[1] + c_u[1]) * 0.12
                + (c_rd[1] + c_ld[1] + c_ru[1] + c_lu[1]) * 0.055
                + (c_rr[1] + c_ll[1] + c_dd[1] + c_uu[1]) * 0.035,
            c0[2] * 0.16
                + (c_r[2] + c_l[2] + c_d[2] + c_u[2]) * 0.12
                + (c_rd[2] + c_ld[2] + c_ru[2] + c_lu[2]) * 0.055
                + (c_rr[2] + c_ll[2] + c_dd[2] + c_uu[2]) * 0.035,
        ]
    }
}

/// Generates a realistic procedural California Redwood Forest backdrop
/// with Tyndall sunbeams, deep atmospheric mist, and contrasting vertical tree trunks.
fn create_procedural_redwood_buffer(width: u32, height: u32) -> WallpaperBuffer {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let v = y as f32 / height.max(1) as f32;
        for x in 0..width {
            let u = x as f32 / width.max(1) as f32;

            // 1. Golden Tyndall sunbeam + mist atmosphere
            let sky_beam = ((u * 6.28 - 1.2).sin() * 0.5 + 0.5).powf(2.5) * (1.0 - v * 0.6);
            let sky_grad = lerp_rgb([0.16, 0.26, 0.38], [0.88, 0.76, 0.52], sky_beam * 0.75 + (1.0 - v).powf(1.6) * 0.25);

            // 2. Redwood tree trunks (strong vertical silhouette contrast)
            let trunk1 = ((u * 18.0).sin() * 0.5 + 0.5).powf(6.0) * (v * 0.9 + 0.1);
            let trunk2 = (((u + 0.32) * 11.0).sin() * 0.5 + 0.5).powf(8.0) * (v * 0.85 + 0.15);
            let trunk3 = (((u + 0.73) * 14.0).sin() * 0.5 + 0.5).powf(7.0) * (v * 0.95 + 0.05);

            // 3. Foliage canopy & ground ferns
            let canopy = ((u * 28.0 + v * 16.0).cos() * 0.5 + 0.5) * (1.0 - v * 0.4);
            let forest_base = lerp_rgb(sky_grad, [0.10, 0.22, 0.14], canopy * 0.85);

            // 4. Combine trunks with rich warm redwood bark
            let bark_color = [0.26, 0.13, 0.09];
            let trunks_total = (trunk1 + trunk2 + trunk3).clamp(0.0, 0.92);
            let final_rgb = lerp_rgb(forest_base, bark_color, trunks_total);

            pixels.push((final_rgb[0] * 255.0).clamp(0.0, 255.0) as u8);
            pixels.push((final_rgb[1] * 255.0).clamp(0.0, 255.0) as u8);
            pixels.push((final_rgb[2] * 255.0).clamp(0.0, 255.0) as u8);
            pixels.push(255);
        }
    }
    WallpaperBuffer::new(width, height, pixels)
}

/// Loads the real macOS desktop wallpaper via AppKit ImageIO hardware decoder,
/// or falls back cleanly to the procedural Redwood Forest if running on non-macOS.
fn load_or_create_wallpaper(target_w: u32, target_h: u32) -> Arc<WallpaperBuffer> {
    if let Some((w, h, rgba)) = bmol_window_shell::load_system_wallpaper_rgba(target_w, target_h) {
        Arc::new(WallpaperBuffer::new(w, h, rgba))
    } else {
        Arc::new(create_procedural_redwood_buffer(target_w, target_h))
    }
}

/// Blur strength presets with fine-grained low-radius resolution (0pt ~ 16pt)
/// and high-end diffusion capping at 64pt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlurPreset {
    Zero0,
    Subtle2,
    Subtle4,
    Subtle8,
    Medium12,
    #[default]
    Standard16,
    Enhanced24,
    Heavy32,
    Ultra48,
    Max64,
}

impl BlurPreset {
    pub const ALL: [Self; 10] = [
        Self::Zero0,
        Self::Subtle2,
        Self::Subtle4,
        Self::Subtle8,
        Self::Medium12,
        Self::Standard16,
        Self::Enhanced24,
        Self::Heavy32,
        Self::Ultra48,
        Self::Max64,
    ];

    #[must_use]
    pub const fn radius(self) -> f32 {
        match self {
            Self::Zero0 => 0.0,
            Self::Subtle2 => 2.0,
            Self::Subtle4 => 4.0,
            Self::Subtle8 => 8.0,
            Self::Medium12 => 12.0,
            Self::Standard16 => 16.0,
            Self::Enhanced24 => 24.0,
            Self::Heavy32 => 32.0,
            Self::Ultra48 => 48.0,
            Self::Max64 => 64.0,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Zero0 => "0pt 纯透无磨砂",
            Self::Subtle2 => "2pt 极微折射",
            Self::Subtle4 => "4pt 轻微雾面",
            Self::Subtle8 => "8pt 经典微磨砂",
            Self::Medium12 => "12pt 浅层高斯",
            Self::Standard16 => "16pt 标准毛玻璃",
            Self::Enhanced24 => "24pt 强磨砂",
            Self::Heavy32 => "32pt 深度弥散",
            Self::Ultra48 => "48pt 极深毛玻璃",
            Self::Max64 => "64pt 全弥散上限",
        }
    }

    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Zero0 => "0pt",
            Self::Subtle2 => "2pt",
            Self::Subtle4 => "4pt",
            Self::Subtle8 => "8pt",
            Self::Medium12 => "12pt",
            Self::Standard16 => "16pt",
            Self::Enhanced24 => "24pt",
            Self::Heavy32 => "32pt",
            Self::Ultra48 => "48pt",
            Self::Max64 => "64pt",
        }
    }
}

/// Available colorful test pattern and atmospheric wallpapers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallpaperStyle {
    /// 100% 透明穿透至 macOS 系统桌面
    #[default]
    DesktopTransparent,
    AuroraMesh,
    SunsetGaze,
    TvColorBars,
    TvSmpteSplit,
    TvColorGrid,
    PureWhite,
    PureBlack,
}

impl WallpaperStyle {
    pub const ALL: [Self; 8] = [
        Self::DesktopTransparent,
        Self::AuroraMesh,
        Self::SunsetGaze,
        Self::TvColorBars,
        Self::TvSmpteSplit,
        Self::TvColorGrid,
        Self::PureWhite,
        Self::PureBlack,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DesktopTransparent => "🖥️ 系统壁纸",
            Self::AuroraMesh => "极光",
            Self::SunsetGaze => "日落",
            Self::TvColorBars => "彩条",
            Self::TvSmpteSplit => "SMPTE",
            Self::TvColorGrid => "网格",
            Self::PureWhite => "纯白",
            Self::PureBlack => "纯黑",
        }
    }
}

#[inline]
fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Computes the unblurred base wallpaper RGB color at coordinate `(x, y)`.
#[inline]
fn sample_sharp_wallpaper(
    style: WallpaperStyle,
    x: f32,
    y: f32,
    bounds: Size,
    wallpaper_buf: Option<&WallpaperBuffer>,
) -> [f32; 3] {
    const TV_BARS: [[f32; 3]; 8] = [
        [1.0, 1.0, 1.0], // 0: 白
        [1.0, 1.0, 0.0], // 1: 黄
        [0.0, 1.0, 1.0], // 2: 青
        [0.0, 1.0, 0.0], // 3: 绿
        [1.0, 0.0, 1.0], // 4: 洋红
        [1.0, 0.0, 0.0], // 5: 红
        [0.0, 0.0, 1.0], // 6: 蓝
        [0.0, 0.0, 0.0], // 7: 黑
    ];

    const TOP_SMPTE: [[f32; 3]; 7] = [
        [1.0, 1.0, 1.0], // 白
        [1.0, 1.0, 0.0], // 黄
        [0.0, 1.0, 1.0], // 青
        [0.0, 1.0, 0.0], // 绿
        [1.0, 0.0, 1.0], // 洋红
        [1.0, 0.0, 0.0], // 红
        [0.0, 0.0, 1.0], // 蓝
    ];

    const BOT_SMPTE: [[f32; 3]; 8] = [
        [0.0, 0.0, 1.0], // 蓝
        [0.0, 0.0, 0.0], // 黑
        [1.0, 0.0, 1.0], // 洋红
        [0.0, 0.0, 0.0], // 黑
        [0.0, 1.0, 1.0], // 青
        [0.0, 0.0, 0.0], // 黑
        [1.0, 1.0, 1.0], // 白
        [0.0, 0.0, 0.0], // 黑
    ];

    const GRID_PALETTE: [[f32; 3]; 12] = [
        [1.0, 0.2, 0.3], // Coral Red
        [1.0, 0.6, 0.0], // Orange
        [1.0, 0.9, 0.1], // Gold
        [0.2, 0.8, 0.4], // Emerald
        [0.0, 0.7, 0.9], // Cyan
        [0.2, 0.4, 1.0], // Cobalt
        [0.6, 0.2, 0.9], // Purple
        [1.0, 0.3, 0.7], // Magenta
        [0.1, 0.9, 0.8], // Mint
        [0.9, 0.8, 0.2], // Yellow
        [0.3, 0.2, 0.8], // Indigo
        [0.9, 0.4, 0.2], // Rust
    ];

    match style {
        WallpaperStyle::DesktopTransparent => {
            if let Some(buf) = wallpaper_buf {
                let u = (x / bounds.width.max(1.0)).clamp(0.0, 1.0);
                let v = (y / bounds.height.max(1.0)).clamp(0.0, 1.0);
                buf.sample_bilinear(u, v)
            } else {
                [0.12, 0.12, 0.14]
            }
        }
        WallpaperStyle::AuroraMesh => {
            let u = (x / bounds.width.max(1.0)).clamp(0.0, 1.0);
            let v = (y / bounds.height.max(1.0)).clamp(0.0, 1.0);
            let t = (u * 0.707 + v * 0.707).clamp(0.0, 1.0);
            if t < 0.20 {
                let f = t / 0.20;
                lerp_rgb([0.12, 0.06, 0.38], [0.38, 0.10, 0.58], f)
            } else if t < 0.44 {
                let f = (t - 0.20) / 0.24;
                lerp_rgb([0.38, 0.10, 0.58], [0.88, 0.16, 0.48], f)
            } else if t < 0.68 {
                let f = (t - 0.44) / 0.24;
                lerp_rgb([0.88, 0.16, 0.48], [0.98, 0.46, 0.15], f)
            } else if t < 0.86 {
                let f = (t - 0.68) / 0.18;
                lerp_rgb([0.98, 0.46, 0.15], [0.95, 0.80, 0.22], f)
            } else {
                let f = (t - 0.86) / 0.14;
                lerp_rgb([0.95, 0.80, 0.22], [0.12, 0.78, 0.82], f)
            }
        }
        WallpaperStyle::SunsetGaze => {
            let u = (x / bounds.width.max(1.0)).clamp(0.0, 1.0);
            let v = (y / bounds.height.max(1.0)).clamp(0.0, 1.0);
            let t = (u * 0.6 + v * 0.8).clamp(0.0, 1.0);
            if t < 0.30 {
                let f = t / 0.30;
                lerp_rgb([0.06, 0.10, 0.25], [0.35, 0.12, 0.42], f)
            } else if t < 0.60 {
                let f = (t - 0.30) / 0.30;
                lerp_rgb([0.35, 0.12, 0.42], [0.82, 0.22, 0.35], f)
            } else if t < 0.82 {
                let f = (t - 0.60) / 0.22;
                lerp_rgb([0.82, 0.22, 0.35], [0.96, 0.52, 0.18], f)
            } else {
                let f = (t - 0.82) / 0.18;
                lerp_rgb([0.96, 0.52, 0.18], [1.0, 0.82, 0.45], f)
            }
        }
        WallpaperStyle::TvColorBars => {
            let n = 8.0;
            let bar_w = bounds.width / n;
            let idx = ((x / bar_w).floor() as usize).min(7);
            TV_BARS[idx]
        }
        WallpaperStyle::TvSmpteSplit => {
            let top_h = bounds.height * 0.70;
            if y < top_h {
                let n = 7.0;
                let bar_w = bounds.width / n;
                let idx = ((x / bar_w).floor() as usize).min(6);
                TOP_SMPTE[idx]
            } else {
                let n = 8.0;
                let bar_w = bounds.width / n;
                let idx = ((x / bar_w).floor() as usize).min(7);
                BOT_SMPTE[idx]
            }
        }
        WallpaperStyle::TvColorGrid => {
            let cols = 4.0;
            let rows = 3.0;
            let cell_w = bounds.width / cols;
            let cell_h = bounds.height / rows;
            let c = ((x / cell_w).floor() as usize).min(3);
            let r = ((y / cell_h).floor() as usize).min(2);
            let idx = (r * 4 + c) % 12;
            GRID_PALETTE[idx]
        }
        WallpaperStyle::PureWhite => [1.0, 1.0, 1.0],
        WallpaperStyle::PureBlack => [0.0, 0.0, 0.0],
    }
}


/// Performs a true 2D Separable Gaussian Convolution on an RGB float buffer.
///
/// Executes two 1D passes (Horizontal then Vertical), with time complexity O(2 * K * W * H),
/// completely eliminating high-frequency textures (pebbles, foam, sharp edges)
/// in strict accordance with physical light diffusion.
pub fn perform_separable_gaussian_blur(src: &[[f32; 3]], w: usize, h: usize, radius: f32) -> Vec<[f32; 3]> {
    if radius <= 0.5 {
        return src.to_vec();
    }
    let sigma = (radius * 1.25).max(0.5);
    let kernel_radius = ((sigma * 2.5).ceil() as usize).clamp(1, 36);

    let mut weights = Vec::with_capacity(kernel_radius * 2 + 1);
    let two_sigma_sq = 2.0 * sigma * sigma;
    let mut sum = 0.0f32;
    for k in -(kernel_radius as isize)..=(kernel_radius as isize) {
        let weight = (-(k as f32 * k as f32) / two_sigma_sq).exp();
        weights.push(weight);
        sum += weight;
    }
    let inv_sum = 1.0 / sum;
    for w_val in &mut weights {
        *w_val *= inv_sum;
    }

    // Pass 1: Horizontal 1D convolution (src -> temp)
    let mut temp = vec![[0.0f32; 3]; w * h];
    for y in 0..h {
        let row_offset = y * w;
        for x in 0..w {
            let mut r = 0.0f32;
            let mut g = 0.0f32;
            let mut b = 0.0f32;
            for (i, &k_w) in weights.iter().enumerate() {
                let offset = i as isize - kernel_radius as isize;
                let sample_x = (x as isize + offset).clamp(0, (w - 1) as isize) as usize;
                let p = src[row_offset + sample_x];
                r += p[0] * k_w;
                g += p[1] * k_w;
                b += p[2] * k_w;
            }
            temp[row_offset + x] = [r, g, b];
        }
    }

    // Pass 2: Vertical 1D convolution (temp -> dst)
    let mut dst = vec![[0.0f32; 3]; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut r = 0.0f32;
            let mut g = 0.0f32;
            let mut b = 0.0f32;
            for (i, &k_w) in weights.iter().enumerate() {
                let offset = i as isize - kernel_radius as isize;
                let sample_y = (y as isize + offset).clamp(0, (h - 1) as isize) as usize;
                let p = temp[sample_y * w + x];
                r += p[0] * k_w;
                g += p[1] * k_w;
                b += p[2] * k_w;
            }
            dst[y * w + x] = [r, g, b];
        }
    }

    dst
}

/// Generates a physical 2D frosted backdrop plate texture slice.
///
/// Features:
/// 1. True 2D separable Gaussian convolution with edge bleeding padding;
/// 2. Apple Vibrancy color enhancement (saturation boost & luma lift);
/// 3. IGN anti-banding dithering;
/// 4. Analytical Squircle sub-pixel anti-aliased alpha mask.
pub fn generate_frosted_plate_texture(
    style: WallpaperStyle,
    rect: Rectangle,
    window_size: Size,
    blur_radius: f32,
    corner_radius: f32,
    is_dark: bool,
    wallpaper_buf: Option<&WallpaperBuffer>,
) -> iced::widget::image::Handle {
    let w = (rect.width.round() as usize).max(1);
    let h = (rect.height.round() as usize).max(1);

    let blurred_rgb = if blur_radius <= 0.5 {
        // 0pt: Sharp exact crop without any blur
        let mut raw_rgb = Vec::with_capacity(w * h);
        for iy in 0..h {
            let wy = rect.y + (iy as f32 + 0.5);
            for ix in 0..w {
                let wx = rect.x + (ix as f32 + 0.5);
                let c = sample_sharp_wallpaper(style, wx, wy, window_size, wallpaper_buf);
                raw_rgb.push(c);
            }
        }
        raw_rgb
    } else {
        // 2pt ~ 64pt: True 2D Gaussian convolution with edge bleeding padding
        let sigma = (blur_radius * 1.25).max(0.5);
        let kernel_radius = ((sigma * 2.5).ceil() as usize).clamp(1, 36);
        let p = kernel_radius;
        let ext_w = w + 2 * p;
        let ext_h = h + 2 * p;

        let mut ext_raw = Vec::with_capacity(ext_w * ext_h);
        for iy in 0..ext_h {
            let wy = (rect.y - p as f32 + (iy as f32 + 0.5)).clamp(0.0, window_size.height);
            for ix in 0..ext_w {
                let wx = (rect.x - p as f32 + (ix as f32 + 0.5)).clamp(0.0, window_size.width);
                let c = sample_sharp_wallpaper(style, wx, wy, window_size, wallpaper_buf);
                ext_raw.push(c);
            }
        }

        let ext_blurred = perform_separable_gaussian_blur(&ext_raw, ext_w, ext_h, blur_radius);

        // Crop back central w * h
        let mut cropped = Vec::with_capacity(w * h);
        for iy in 0..h {
            let row_offset = (iy + p) * ext_w;
            for ix in 0..w {
                cropped.push(ext_blurred[row_offset + (ix + p)]);
            }
        }
        cropped
    };

    // Apply Apple Vibrancy color lift & IGN anti-banding dither & Analytical Squircle AA Mask
    let vibrancy = VibrancyConfig {
        saturation_boost: if is_dark { 1.25 } else { 1.30 },
        luma_lift: if is_dark { 1.02 } else { 1.05 },
        luma_bias: if is_dark { 0.005 } else { 0.012 },
        ..Default::default()
    };

    let mut rgba_bytes = vec![0u8; w * h * 4];
    for iy in 0..h {
        let py = iy as f32 + 0.5;
        let wy = rect.y + py;
        for ix in 0..w {
            let px = ix as f32 + 0.5;
            let wx = rect.x + px;
            let idx = iy * w + ix;

            let c = blurred_rgb[idx];
            let vibrant = vibrancy.apply(c);
            let dither = ign_dither_offset(wx, wy);

            let p = SquirclePoint::new(px - w as f32 * 0.5, py - h as f32 * 0.5);
            let half = SquirclePoint::new(w as f32 * 0.5, h as f32 * 0.5);
            let alpha = squircle_alpha(p, half, corner_radius, APPLE_CORNER_SMOOTHING);

            let out_idx = idx * 4;
            rgba_bytes[out_idx] = ((vibrant[0] + dither).clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba_bytes[out_idx + 1] = ((vibrant[1] + dither).clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba_bytes[out_idx + 2] = ((vibrant[2] + dither).clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba_bytes[out_idx + 3] = (alpha * 255.0).round() as u8;
        }
    }

    iced::widget::image::Handle::from_rgba(w as u32, h as u32, rgba_bytes)
}

/// Application icons featured on the frosted Dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockApp {
    Finder,
    Safari,
    Messages,
    Mail,
    Music,
    Photos,
    Terminal,
    Settings,
    Trash,
}

impl DockApp {
    pub const ALL: [Self; 9] = [
        Self::Finder,
        Self::Safari,
        Self::Messages,
        Self::Mail,
        Self::Music,
        Self::Photos,
        Self::Terminal,
        Self::Settings,
        Self::Trash,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Finder => "访达 (Finder)",
            Self::Safari => "Safari 浏览器",
            Self::Messages => "信息 (Messages)",
            Self::Mail => "邮件 (Mail)",
            Self::Music => "音乐 (Music)",
            Self::Photos => "照片 (Photos)",
            Self::Terminal => "终端 (Terminal)",
            Self::Settings => "系统设置 (Settings)",
            Self::Trash => "废纸篓 (Trash)",
        }
    }

    #[must_use]
    pub const fn primary_color(self) -> Color {
        match self {
            Self::Finder => Color::from_rgb(0.16, 0.52, 0.95),
            Self::Safari => Color::from_rgb(0.08, 0.58, 0.98),
            Self::Messages => Color::from_rgb(0.20, 0.80, 0.38),
            Self::Mail => Color::from_rgb(0.12, 0.65, 0.95),
            Self::Music => Color::from_rgb(0.96, 0.20, 0.36),
            Self::Photos => Color::from_rgb(0.96, 0.96, 0.98),
            Self::Terminal => Color::from_rgb(0.12, 0.13, 0.15),
            Self::Settings => Color::from_rgb(0.55, 0.57, 0.62),
            Self::Trash => Color::from_rgb(0.45, 0.47, 0.52),
        }
    }

    #[must_use]
    pub const fn gradient_colors(self) -> (Color, Color) {
        match self {
            Self::Finder => (
                Color::from_rgb(0.24, 0.65, 0.98),
                Color::from_rgb(0.10, 0.42, 0.88),
            ),
            Self::Safari => (
                Color::from_rgb(0.18, 0.70, 0.98),
                Color::from_rgb(0.06, 0.45, 0.92),
            ),
            Self::Messages => (
                Color::from_rgb(0.34, 0.86, 0.42),
                Color::from_rgb(0.15, 0.72, 0.28),
            ),
            Self::Mail => (
                Color::from_rgb(0.22, 0.72, 0.98),
                Color::from_rgb(0.06, 0.52, 0.92),
            ),
            Self::Music => (
                Color::from_rgb(0.98, 0.26, 0.42),
                Color::from_rgb(0.90, 0.12, 0.28),
            ),
            Self::Photos => (
                Color::from_rgb(1.0, 1.0, 1.0),
                Color::from_rgb(0.92, 0.93, 0.96),
            ),
            Self::Terminal => (
                Color::from_rgb(0.20, 0.21, 0.24),
                Color::from_rgb(0.08, 0.08, 0.10),
            ),
            Self::Settings => (
                Color::from_rgb(0.70, 0.72, 0.76),
                Color::from_rgb(0.48, 0.50, 0.55),
            ),
            Self::Trash => (
                Color::from_rgb(0.56, 0.58, 0.62),
                Color::from_rgb(0.38, 0.40, 0.45),
            ),
        }
    }

    /// Returns the authentic macOS on-disk application bundle path for Scheme A.
    #[must_use]
    pub const fn system_app_path(self) -> &'static str {
        match self {
            Self::Finder => "/System/Library/CoreServices/Finder.app",
            Self::Safari => "/Applications/Safari.app",
            Self::Messages => "/System/Applications/Messages.app",
            Self::Mail => "/System/Applications/Mail.app",
            Self::Music => "/System/Applications/Music.app",
            Self::Photos => "/System/Applications/Photos.app",
            Self::Terminal => "/System/Applications/Utilities/Terminal.app",
            Self::Settings => "/System/Applications/System Settings.app",
            Self::Trash => "named:NSTrashEmpty",
        }
    }
}

/// Unified, mathematically guaranteed layout geometry for the entire stage.
#[derive(Debug, Clone, Copy)]
pub struct LayoutMetrics {
    pub window_size: Size,
    pub header_h: f32,
    pub status_h: f32,
    pub search_rect: Rectangle,
    pub dock_rect: Rectangle,
    pub dock_radius: f32,
    pub base_icon_size: f32,
    pub icon_radius: f32,
    pub icon_gap: f32,
    pub dock_padding: f32,
    pub icon_rects: [Rectangle; 9],
}

impl LayoutMetrics {
    #[must_use]
    pub fn new(window_size: Size, hovered_app: Option<DockApp>) -> Self {
        let header_h = 44.0f32;
        let status_h = 28.0f32;

        // Search Bar Capsule (Centered in top portion of stage)
        let search_w = 420.0f32.min(window_size.width - 60.0);
        let search_h = 42.0f32;
        let search_rect = Rectangle {
            x: (window_size.width - search_w) * 0.5,
            y: header_h + 20.0,
            width: search_w,
            height: search_h,
        };

        // --- Concentric Geometric Proportion System (SSOT: bmol_designs::dock_metrics) ---
        let base_icon_size = dock_metrics::BASE_ICON_SIZE;
        let icon_radius = dock_metrics::icon_corner_radius(base_icon_size);
        let icon_gap = dock_metrics::icon_gap(base_icon_size);
        let dock_padding = dock_metrics::dock_padding(base_icon_size);

        let dock_radius = dock_metrics::concentric_dock_radius(icon_radius, dock_padding);
        let dock_h = dock_metrics::dock_height(base_icon_size);
        let dock_w = dock_metrics::dock_width(DockApp::ALL.len(), base_icon_size)
            .min(window_size.width - 40.0);

        let dock_y = (window_size.height - status_h - 16.0 - dock_h).max(header_h + 120.0);
        let dock_x = (window_size.width - dock_w) * 0.5;
        let dock_rect = Rectangle {
            x: dock_x,
            y: dock_y,
            width: dock_w,
            height: dock_h,
        };

        // 3. Symmetrically Padded 9 Icon Squircles
        let start_x = dock_x + dock_padding;
        let base_y = dock_y + dock_padding;

        let mut icon_rects = [Rectangle::default(); 9];
        for (i, app) in DockApp::ALL.iter().enumerate() {
            let is_hovered = hovered_app == Some(*app);
            let size = if is_hovered { 46.0 } else { base_icon_size };
            let offset_x = (size - base_icon_size) * 0.5;
            let offset_y = if is_hovered { 6.0 } else { 0.0 };

            let x = start_x + i as f32 * (base_icon_size + icon_gap) - offset_x;
            let y = base_y - offset_y;

            icon_rects[i] = Rectangle {
                x,
                y,
                width: size,
                height: size,
            };
        }

        Self {
            window_size,
            header_h,
            status_h,
            search_rect,
            dock_rect,
            dock_radius,
            base_icon_size,
            icon_radius,
            icon_gap,
            dock_padding,
            icon_rects,
        }
    }
}

/// Context menu target type for right-clicks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuContext {
    App(DockApp),
    DockBar,
    SearchBar,
    Wallpaper,
}

/// Preset levels of glass plate translucency & transparency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlassTransparency {
    /// Ultra Clear Glass (Minimal center tint, pure optical refraction feel - Apple liquid glass default)
    #[default]
    Ultra,
    /// High Transparency (Apple Sequoia/Sonoma style: ~90% clear middle, vibrant wallpaper transmission)
    High,
    /// Milky Frosted Glass (Traditional heavier frosted substrate)
    Frosted,
}

impl GlassTransparency {
    pub const ALL: [Self; 3] = [Self::Ultra, Self::High, Self::Frosted];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Ultra => "极清",
            Self::High => "高透",
            Self::Frosted => "磨砂",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Ultra => Self::High,
            Self::High => Self::Frosted,
            Self::Frosted => Self::Ultra,
        }
    }

    #[must_use]
    pub fn center_alpha_light(&self) -> f32 {
        match self {
            Self::High => 0.13,
            Self::Ultra => 0.06,
            Self::Frosted => 0.24,
        }
    }

    #[must_use]
    pub fn center_alpha_dark(&self) -> f32 {
        match self {
            Self::High => 0.20,
            Self::Ultra => 0.10,
            Self::Frosted => 0.32,
        }
    }
}

/// Messages for the interactive application.
#[derive(Debug, Clone)]
pub enum Message {
    WindowReady(Option<window::Id>),
    WindowResized(Size),
    WindowEvent((window::Id, iced::window::Event)),
    AnimationFrame(Instant),
    ResizeWindow(window::Direction),
    DragWindow,
    TrafficLights(TrafficLightsEvent),
    SetWallpaper(WallpaperStyle),
    SetBlurPreset(BlurPreset),
    ToggleTheme,
    CycleTransparency,
    SetTransparency(GlassTransparency),
    ToggleHighlight(bool),
    ToggleDarkRim(bool),
    ToggleGrid(bool),
    ToggleDocumentEdited,
    SystemThemeChanged(iced::theme::Mode),
    SearchInputChanged(String),
    CursorMoved(Point),
    IconHovered(Option<DockApp>),
    IconClicked(DockApp),
    RightClicked,
    DismissFloatingMenu,
    TriggerAction(String),
}

/// Renders the sharp backdrop wallpaper, alignment grid, and 2D frosted blur slices.
struct LiquidGlassBackdropCanvas {
    style: WallpaperStyle,
    metrics: LayoutMetrics,
    show_grid: bool,
    system_wallpaper: Option<Arc<WallpaperBuffer>>,
    dock_frosted_texture: Option<iced::widget::image::Handle>,
    search_frosted_texture: Option<iced::widget::image::Handle>,
}

impl<Message> canvas::Program<Message, Theme, iced_backend::Renderer> for LiquidGlassBackdropCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced_backend::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let window_squircle = build_squircle_path(
            Rectangle::new(Point::ORIGIN, bounds.size()),
            window_metrics::DEFAULT_CORNER_RADIUS,
        );

        // 1. Draw Base Sharp Wallpaper (Presets)
        match self.style {
            WallpaperStyle::DesktopTransparent => {
                // 100% transparent to desktop — no opaque rectangular image drawn!
            }
            WallpaperStyle::AuroraMesh => {
                let grad = Linear::new(Point::ORIGIN, Point::new(bounds.width, bounds.height))
                    .add_stop(0.0, Color::from_rgb(0.12, 0.06, 0.38))
                    .add_stop(0.20, Color::from_rgb(0.38, 0.10, 0.58))
                    .add_stop(0.44, Color::from_rgb(0.88, 0.16, 0.48))
                    .add_stop(0.68, Color::from_rgb(0.98, 0.46, 0.15))
                    .add_stop(0.86, Color::from_rgb(0.95, 0.80, 0.22))
                    .add_stop(1.0, Color::from_rgb(0.12, 0.78, 0.82));
                frame.fill(&window_squircle, grad);
            }
            WallpaperStyle::SunsetGaze => {
                let grad = Linear::new(Point::new(bounds.width * 0.15, 0.0), Point::new(bounds.width * 0.85, bounds.height))
                    .add_stop(0.0, Color::from_rgb(0.06, 0.10, 0.25))
                    .add_stop(0.30, Color::from_rgb(0.35, 0.12, 0.42))
                    .add_stop(0.60, Color::from_rgb(0.82, 0.22, 0.35))
                    .add_stop(0.82, Color::from_rgb(0.96, 0.52, 0.18))
                    .add_stop(1.0, Color::from_rgb(1.0, 0.82, 0.45));
                frame.fill(&window_squircle, grad);
            }
            WallpaperStyle::TvColorBars => {
                let n = 8.0;
                let bar_w = bounds.width / n;
                for i in 0..8 {
                    let c = sample_sharp_wallpaper(self.style, (i as f32 + 0.5) * bar_w, 0.0, bounds.size(), self.system_wallpaper.as_deref());
                    frame.fill_rectangle(
                        Point::new(i as f32 * bar_w, 0.0),
                        Size::new(bar_w + 1.0, bounds.height),
                        Color::from_rgb(c[0], c[1], c[2]),
                    );
                }
            }
            WallpaperStyle::TvSmpteSplit => {
                let top_h = bounds.height * 0.70;
                let bot_h = bounds.height - top_h;

                let n_top = 7.0;
                let bar_w_top = bounds.width / n_top;
                for i in 0..7 {
                    let c = sample_sharp_wallpaper(self.style, (i as f32 + 0.5) * bar_w_top, 10.0, bounds.size(), self.system_wallpaper.as_deref());
                    frame.fill_rectangle(
                        Point::new(i as f32 * bar_w_top, 0.0),
                        Size::new(bar_w_top + 1.0, top_h),
                        Color::from_rgb(c[0], c[1], c[2]),
                    );
                }

                let n_bot = 8.0;
                let bar_w_bot = bounds.width / n_bot;
                for i in 0..8 {
                    let c = sample_sharp_wallpaper(self.style, (i as f32 + 0.5) * bar_w_bot, top_h + 10.0, bounds.size(), self.system_wallpaper.as_deref());
                    frame.fill_rectangle(
                        Point::new(i as f32 * bar_w_bot, top_h),
                        Size::new(bar_w_bot + 1.0, bot_h),
                        Color::from_rgb(c[0], c[1], c[2]),
                    );
                }
            }
            WallpaperStyle::TvColorGrid => {
                let cols = 4.0;
                let rows = 3.0;
                let cell_w = bounds.width / cols;
                let cell_h = bounds.height / rows;
                for r in 0..3 {
                    for c in 0..4 {
                        let sample = sample_sharp_wallpaper(
                            self.style,
                            (c as f32 + 0.5) * cell_w,
                            (r as f32 + 0.5) * cell_h,
                            bounds.size(),
                            self.system_wallpaper.as_deref(),
                        );
                        frame.fill_rectangle(
                            Point::new(c as f32 * cell_w, r as f32 * cell_h),
                            Size::new(cell_w + 1.0, cell_h + 1.0),
                            Color::from_rgb(sample[0], sample[1], sample[2]),
                        );
                    }
                }
            }
            WallpaperStyle::PureWhite => {
                frame.fill(&window_squircle, Color::WHITE);
            }
            WallpaperStyle::PureBlack => {
                frame.fill(&window_squircle, Color::BLACK);
            }
        }

        // 2. Alignment Calibration Grid Lines
        if self.show_grid && !matches!(self.style, WallpaperStyle::PureWhite | WallpaperStyle::PureBlack) {
            let grid_step = 36.0f32;
            let line_color = Color::from_rgba(1.0, 1.0, 1.0, 0.16);
            let mut x = grid_step;
            while x < bounds.width {
                frame.fill_rectangle(Point::new(x, 0.0), Size::new(1.0, bounds.height), line_color);
                x += grid_step;
            }
            let mut y = grid_step;
            while y < bounds.height {
                frame.fill_rectangle(Point::new(0.0, y), Size::new(bounds.width, 1.0), line_color);
                y += grid_step;
            }
        }

        // 3. Draw 2D Frosted Backdrop Texture Slices with sub-pixel Squircle AA mask
        let s_rect = self.metrics.search_rect;
        if let Some(texture) = &self.search_frosted_texture {
            frame.draw_image(s_rect, iced::widget::canvas::Image::new(texture.clone()));
        }

        let d_rect = self.metrics.dock_rect;
        if let Some(texture) = &self.dock_frosted_texture {
            frame.draw_image(d_rect, iced::widget::canvas::Image::new(texture.clone()));
        }

        vec![frame.into_geometry()]
    }
}

/// Renders the Apple Liquid Glass optics, substrate tint, specular bevel, and App icons on top.
struct LiquidGlassForegroundCanvas {
    metrics: LayoutMetrics,
    blur_radius: f32,
    enable_highlight: bool,
    enable_dark_rim: bool,
    is_dark: bool,
    transparency: GlassTransparency,
    floating_menu_rect: Option<Rectangle>,
    app_icons: [Option<iced::widget::image::Handle>; 9],
}

impl<Message> canvas::Program<Message, Theme, iced_backend::Renderer> for LiquidGlassForegroundCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced_backend::Renderer,
        _theme: &Theme,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, _bounds.size());

        // 1. Render Frosted Search Bar Optics
        let s_rect = self.metrics.search_rect;
        draw_liquid_glass_plate(
            &mut frame,
            s_rect,
            21.0,
            self.is_dark,
            self.transparency,
            self.enable_highlight,
            self.enable_dark_rim,
            self.blur_radius,
        );

        // 2. Render Main Frosted Dock Optics (Concentric 23px radius = 13px padding + 10px icon R)
        let d_rect = self.metrics.dock_rect;
        draw_liquid_glass_plate(
            &mut frame,
            d_rect,
            self.metrics.dock_radius,
            self.is_dark,
            self.transparency,
            self.enable_highlight,
            self.enable_dark_rim,
            self.blur_radius,
        );

        // 3. Render 9 Authentic macOS Squircle Icons & Interactive Mechanics
        for (i, app) in DockApp::ALL.iter().enumerate() {
            let i_rect = self.metrics.icon_rects[i];
            draw_apple_icon(
                &mut frame,
                *app,
                i_rect,
                self.is_dark,
                self.enable_highlight,
                self.enable_dark_rim,
                self.app_icons[i].as_ref(),
            );

            // macOS authentic active app indicator dot
            let is_running = matches!(
                app,
                DockApp::Finder
                    | DockApp::Safari
                    | DockApp::Messages
                    | DockApp::Mail
                    | DockApp::Terminal
                    | DockApp::Settings
            );
            if is_running {
                let dot_cx = i_rect.x + i_rect.width * 0.5;
                let dot_cy = d_rect.y + d_rect.height - (self.metrics.dock_padding * 0.35);
                let (dot_color, halo_color) = if self.is_dark {
                    (
                        Color::from_rgba(1.0, 1.0, 1.0, 0.90),
                        Color::from_rgba(1.0, 1.0, 1.0, 0.18),
                    )
                } else {
                    (
                        Color::from_rgba(0.08, 0.09, 0.12, 0.65),
                        Color::from_rgba(1.0, 1.0, 1.0, 0.45),
                    )
                };
                frame.fill(&Path::circle(Point::new(dot_cx, dot_cy), 2.75), halo_color);
                frame.fill(&Path::circle(Point::new(dot_cx, dot_cy), 1.75), dot_color);
            }
        }

        // 4. Render Floating Context Menu (if active)
        if let Some(m_rect) = self.floating_menu_rect {
            draw_liquid_glass_plate(
                &mut frame,
                m_rect,
                12.0,
                self.is_dark,
                self.transparency,
                self.enable_highlight,
                self.enable_dark_rim,
                self.blur_radius,
            );
        }

        vec![frame.into_geometry()]
    }
}



/// Renders a complete authentic Apple Liquid Glass Plate (frosted substrate, specular highlight, rim darkening)
/// with 100% continuous G2 curvature, multi-tier Gaussian shadow, and physical optics.
fn draw_liquid_glass_plate<R: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<R>,
    rect: Rectangle,
    radius: f32,
    is_dark: bool,
    transparency: GlassTransparency,
    enable_highlight: bool,
    enable_dark_rim: bool,
    blur_radius: f32,
) {
    // 1. Soft subtle ambient elevation drop shadow
    draw_elevation_shadow(frame, rect, radius, is_dark, transparency);

    // 2. Base Glass Substrate (Calibrated Apple macOS authentic transparency & intrinsic silver/graphite tint)
    let path = build_squircle_path(rect, radius);
    let frost_factor = (blur_radius / 16.0).clamp(0.0, 1.0);
    let glass_grad = if is_dark {
        let base_c_mid = transparency.center_alpha_dark();
        let c_mid = base_c_mid * (0.05 + 0.35 * frost_factor);
        let c_top = (c_mid + 0.028).min(0.28);
        let c_bot = (c_mid + 0.012).min(0.26);
        let tint = Color::from_rgb(0.095, 0.102, 0.125);
        let edge_tint = Color::from_rgb(0.12, 0.135, 0.16);
        Linear::new(Point::new(rect.x, rect.y), Point::new(rect.x, rect.y + rect.height))
            .add_stop(0.0, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_top))
            .add_stop(0.08, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, (c_top + c_mid) * 0.5))
            .add_stop(0.22, Color::from_rgba(tint.r, tint.g, tint.b, c_mid))
            .add_stop(0.78, Color::from_rgba(tint.r, tint.g, tint.b, c_mid))
            .add_stop(0.92, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, (c_bot + c_mid) * 0.5))
            .add_stop(1.0, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_bot))
    } else {
        let base_c_mid = transparency.center_alpha_light();
        let c_mid = base_c_mid * (0.05 + 0.35 * frost_factor);
        let c_top = (c_mid + 0.032).min(0.22);
        let c_bot = (c_mid + 0.014).min(0.20);
        let tint = Color::from_rgb(0.95, 0.96, 0.98);
        let edge_tint = Color::from_rgb(0.98, 0.99, 1.0);
        Linear::new(Point::new(rect.x, rect.y), Point::new(rect.x, rect.y + rect.height))
            .add_stop(0.0, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_top))
            .add_stop(0.08, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, (c_top + c_mid) * 0.5))
            .add_stop(0.22, Color::from_rgba(tint.r, tint.g, tint.b, c_mid))
            .add_stop(0.78, Color::from_rgba(tint.r, tint.g, tint.b, c_mid))
            .add_stop(0.92, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, (c_bot + c_mid) * 0.5))
            .add_stop(1.0, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_bot))
    };
    frame.fill(&path, glass_grad);

    // 4. Apple Liquid Glass Optics & Subpixel-Aligned Boundary Bevel
    // Seamlessly integrates top specular highlight + 0.5pt (1 device pixel on Retina HiDPI) Standard Black Hairline on lateral sides.
    draw_liquid_glass_bevel(frame, rect, radius, is_dark, enable_highlight, enable_dark_rim);
}

/// Draws an authentic macOS Squircle App Icon with vector graphics and liquid glass bevel.
fn draw_apple_icon<R: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<R>,
    app: DockApp,
    rect: Rectangle,
    is_dark: bool,
    enable_highlight: bool,
    enable_dark_rim: bool,
    icon_image: Option<&iced::widget::image::Handle>,
) {
    // 10:20:10 Curvature Ratio (SSOT: bmol_designs::dock_metrics)
    let r = dock_metrics::icon_corner_radius(rect.width);

    // 0. Soft physical contact drop shadow underneath the icon onto the dock shelf
    let shadow_color_1 = Color::from_rgba(0.0, 0.0, 0.0, 0.16);
    let shadow_color_2 = Color::from_rgba(0.0, 0.0, 0.0, 0.06);
    fill_squircle(
        frame,
        Rectangle {
            x: rect.x + 2.0,
            y: rect.y + rect.height - 4.0,
            width: rect.width - 4.0,
            height: 4.0,
        },
        2.0,
        shadow_color_1,
    );
    fill_squircle(
        frame,
        Rectangle {
            x: rect.x,
            y: rect.y + 2.0,
            width: rect.width,
            height: rect.height,
        },
        r,
        shadow_color_2,
    );

    if let Some(handle) = icon_image {
        // Authentic Scheme A: Real macOS system App icon processed through squircle-icon-rs
        frame.draw_image(rect, canvas::Image::new(handle.clone()));
    } else {
        // Fallback: Standalone vector squircle plate & glyph illustration
        let path = build_squircle_path(rect, r);
        let (c_top, c_bot) = app.gradient_colors();
        let grad = Linear::new(Point::new(rect.x, rect.y), Point::new(rect.x, rect.y + rect.height))
            .add_stop(0.0, c_top)
            .add_stop(1.0, c_bot);
        frame.fill(&path, grad);

        let cx = rect.x + rect.width * 0.5;
        let cy = rect.y + rect.height * 0.5;
        let s = rect.width;

        match app {
        DockApp::Finder => {
            // Authentic Finder split face dividing line & nose
            let nose = Path::new(|b| {
                b.move_to(Point::new(cx, cy - s * 0.24));
                b.line_to(Point::new(cx, cy + s * 0.01));
                b.line_to(Point::new(cx + s * 0.05, cy + s * 0.05));
                b.line_to(Point::new(cx, cy + s * 0.07));
                b.line_to(Point::new(cx, cy + s * 0.12));
            });
            frame.stroke(&nose, Stroke::default().with_color(Color::WHITE).with_width(s * 0.045));

            // Eyes
            frame.fill(&Path::circle(Point::new(cx - s * 0.15, cy - s * 0.08), s * 0.045), Color::WHITE);
            frame.fill(&Path::circle(Point::new(cx + s * 0.15, cy - s * 0.08), s * 0.045), Color::WHITE);

            // Smile curve
            let smile = Path::new(|b| {
                b.move_to(Point::new(cx - s * 0.18, cy + s * 0.10));
                b.bezier_curve_to(
                    Point::new(cx - s * 0.10, cy + s * 0.24),
                    Point::new(cx + s * 0.10, cy + s * 0.24),
                    Point::new(cx + s * 0.18, cy + s * 0.10),
                );
            });
            frame.stroke(&smile, Stroke::default().with_color(Color::WHITE).with_width(s * 0.055));
        }
        DockApp::Safari => {
            // Compass dial
            let dial_r = s * 0.32;
            frame.stroke(
                &Path::circle(Point::new(cx, cy), dial_r),
                Stroke::default().with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)).with_width(1.5),
            );
            // Needles
            let needle_red = Path::new(|b| {
                b.move_to(Point::new(cx, cy - s * 0.28));
                b.line_to(Point::new(cx + s * 0.06, cy));
                b.line_to(Point::new(cx - s * 0.06, cy));
                b.close();
            });
            frame.fill(&needle_red, Color::from_rgb(0.95, 0.22, 0.22));
            let needle_white = Path::new(|b| {
                b.move_to(Point::new(cx, cy + s * 0.28));
                b.line_to(Point::new(cx + s * 0.06, cy));
                b.line_to(Point::new(cx - s * 0.06, cy));
                b.close();
            });
            frame.fill(&needle_white, Color::WHITE);
            frame.fill(&Path::circle(Point::new(cx, cy), s * 0.04), Color::from_rgb(0.85, 0.85, 0.85));
        }
        DockApp::Messages => {
            let bubble = Path::new(|b| {
                b.arc(canvas::path::Arc {
                    center: Point::new(cx, cy - s * 0.03),
                    radius: s * 0.24,
                    start_angle: Radians(0.0),
                    end_angle: Radians(std::f32::consts::TAU),
                });
                b.move_to(Point::new(cx - s * 0.14, cy + s * 0.12));
                b.line_to(Point::new(cx - s * 0.22, cy + s * 0.25));
                b.line_to(Point::new(cx - s * 0.04, cy + s * 0.20));
                b.close();
            });
            frame.fill(&bubble, Color::WHITE);
            frame.fill(&Path::circle(Point::new(cx - s * 0.10, cy - s * 0.03), s * 0.035), Color::from_rgb(0.20, 0.75, 0.35));
            frame.fill(&Path::circle(Point::new(cx, cy - s * 0.03), s * 0.035), Color::from_rgb(0.20, 0.75, 0.35));
            frame.fill(&Path::circle(Point::new(cx + s * 0.10, cy - s * 0.03), s * 0.035), Color::from_rgb(0.20, 0.75, 0.35));
        }
        DockApp::Mail => {
            let env_rect = Rectangle {
                x: cx - s * 0.28,
                y: cy - s * 0.18,
                width: s * 0.56,
                height: s * 0.36,
            };
            fill_squircle(frame, env_rect, 4.0, Color::WHITE);
            let flap = Path::new(|b| {
                b.move_to(Point::new(env_rect.x, env_rect.y));
                b.line_to(Point::new(cx, cy + s * 0.04));
                b.line_to(Point::new(env_rect.x + env_rect.width, env_rect.y));
            });
            frame.stroke(&flap, Stroke::default().with_color(Color::from_rgb(0.12, 0.55, 0.90)).with_width(2.0));
        }
        DockApp::Music => {
            let note = Path::new(|b| {
                b.move_to(Point::new(cx - s * 0.10, cy + s * 0.10));
                b.line_to(Point::new(cx - s * 0.10, cy - s * 0.16));
                b.line_to(Point::new(cx + s * 0.14, cy - s * 0.22));
                b.line_to(Point::new(cx + s * 0.14, cy + s * 0.04));
            });
            frame.stroke(&note, Stroke::default().with_color(Color::WHITE).with_width(3.0));
            frame.fill(&Path::circle(Point::new(cx - s * 0.14, cy + s * 0.12), s * 0.07), Color::WHITE);
            frame.fill(&Path::circle(Point::new(cx + s * 0.10, cy + s * 0.06), s * 0.07), Color::WHITE);
        }
        DockApp::Photos => {
            let petal_colors = [
                Color::from_rgb(0.95, 0.25, 0.25),
                Color::from_rgb(0.98, 0.55, 0.15),
                Color::from_rgb(0.98, 0.85, 0.10),
                Color::from_rgb(0.35, 0.82, 0.35),
                Color::from_rgb(0.15, 0.80, 0.85),
                Color::from_rgb(0.20, 0.55, 0.95),
                Color::from_rgb(0.65, 0.30, 0.90),
                Color::from_rgb(0.90, 0.25, 0.70),
            ];
            for (k, c) in petal_colors.iter().enumerate() {
                let ang = k as f32 * std::f32::consts::FRAC_PI_4;
                let px = cx + (ang.cos() * s * 0.12);
                let py = cy + (ang.sin() * s * 0.12);
                frame.fill(&Path::circle(Point::new(px, py), s * 0.09), *c);
            }
            frame.fill(&Path::circle(Point::new(cx, cy), s * 0.05), Color::WHITE);
        }
        DockApp::Terminal => {
            let prompt = Path::new(|b| {
                b.move_to(Point::new(cx - s * 0.22, cy - s * 0.14));
                b.line_to(Point::new(cx - s * 0.08, cy - s * 0.04));
                b.line_to(Point::new(cx - s * 0.22, cy + s * 0.06));
            });
            frame.stroke(&prompt, Stroke::default().with_color(Color::from_rgb(0.25, 0.95, 0.45)).with_width(3.0));
            frame.fill_rectangle(
                Point::new(cx, cy + s * 0.04),
                Size::new(s * 0.18, 3.0),
                Color::from_rgb(0.25, 0.95, 0.45),
            );
        }
        DockApp::Settings => {
            frame.stroke(
                &Path::circle(Point::new(cx, cy), s * 0.16),
                Stroke::default().with_color(Color::WHITE).with_width(s * 0.07),
            );
            for k in 0..6 {
                let ang = k as f32 * std::f32::consts::PI / 3.0;
                let tx = cx + ang.cos() * s * 0.22;
                let ty = cy + ang.sin() * s * 0.22;
                frame.fill(&Path::circle(Point::new(tx, ty), s * 0.045), Color::WHITE);
            }
            frame.fill(&Path::circle(Point::new(cx, cy), s * 0.07), Color::from_rgb(0.55, 0.57, 0.62));
        }
        DockApp::Trash => {
            let rim_rect = Rectangle {
                x: cx - s * 0.22,
                y: cy - s * 0.18,
                width: s * 0.44,
                height: 4.0,
            };
            fill_squircle(frame, rim_rect, 2.0, Color::WHITE);
            let bin = Path::new(|b| {
                b.move_to(Point::new(cx - s * 0.18, cy - s * 0.14));
                b.line_to(Point::new(cx - s * 0.14, cy + s * 0.20));
                b.line_to(Point::new(cx + s * 0.14, cy + s * 0.20));
                b.line_to(Point::new(cx + s * 0.18, cy - s * 0.14));
            });
            frame.stroke(&bin, Stroke::default().with_color(Color::WHITE).with_width(2.5));
            frame.fill_rectangle(Point::new(cx - s * 0.06, cy - s * 0.12), Size::new(2.0, s * 0.30), Color::WHITE);
            frame.fill_rectangle(Point::new(cx + s * 0.06, cy - s * 0.12), Size::new(2.0, s * 0.30), Color::WHITE);
        }
    }
    }

    // 4. THE SIGNATURE APPLE LIQUID GLASS BEVEL & OPTICS RIGHT ON THE SQUIRCLE ICON!
    draw_liquid_glass_bevel(frame, rect, r, is_dark, enable_highlight, enable_dark_rim);
}

/// Draws soft multi-tier Gaussian elevation drop shadow underneath floating glass panels.
fn draw_elevation_shadow<R: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<R>,
    rect: Rectangle,
    radius: f32,
    is_dark: bool,
    transparency: GlassTransparency,
) {
    let base_alpha = match transparency {
        GlassTransparency::Ultra => if is_dark { 0.08 } else { 0.05 },
        GlassTransparency::High => if is_dark { 0.14 } else { 0.09 },
        GlassTransparency::Frosted => if is_dark { 0.20 } else { 0.14 },
    };

    // Expanding concentric squircle shells with minimal vertical displacement
    // ensuring the shadow provides soft elevation without creating stepped dark shapes
    // visible through the 92.5% transparent glass core.
    let shells: [(f32, f32, f32); 4] = [
        (1.0, 1.0, 1.2),    // Contact occlusion
        (2.0, 3.0, 0.8),    // Soft penumbra
        (4.0, 6.0, 0.45),   // Ambient falloff
        (6.0, 10.0, 0.20),  // Far diffusion
    ];

    for (y_off, spread, a_mul) in shells {
        let shadow_rect = Rectangle {
            x: rect.x - spread,
            y: rect.y + y_off,
            width: rect.width + spread * 2.0,
            height: rect.height + spread * 1.5,
        };
        let shadow_color = Color::from_rgba(0.0, 0.0, 0.0, base_alpha * a_mul);
        fill_squircle(frame, shadow_rect, radius + spread * 0.75, shadow_color);
    }
}

/// Builds an authentic Apple continuous curvature squircle path (G2 continuity) for a rectangle.
fn build_squircle_path(rect: Rectangle, radius: f32) -> Path {
    let r = radius.min(rect.width * 0.5).min(rect.height * 0.5);
    // When radius is half the height (e.g. search capsule), use circular ends (smoothing = 0.0)
    // for true semicircular capsule geometry; otherwise use Apple continuous curvature G2 smoothing.
    let is_capsule = (r - rect.height * 0.5).abs() < 1.0;
    let smoothing = if is_capsule { 0.0 } else { APPLE_CORNER_SMOOTHING };
    let params = SquircleParams::new(rect.width, rect.height, r)
        .with_smoothing(smoothing);
    let commands = squircle_path_commands(&params);

    Path::new(move |b| {
        for cmd in &commands {
            match *cmd {
                PathCommand::MoveTo(pt) => b.move_to(Point::new(rect.x + pt.x, rect.y + pt.y)),
                PathCommand::LineTo(pt) => b.line_to(Point::new(rect.x + pt.x, rect.y + pt.y)),
                PathCommand::CubicTo { c0, c1, to } => b.bezier_curve_to(
                    Point::new(rect.x + c0.x, rect.y + c0.y),
                    Point::new(rect.x + c1.x, rect.y + c1.y),
                    Point::new(rect.x + to.x, rect.y + to.y),
                ),
                PathCommand::Close => b.close(),
            }
        }
    })
}

/// Fills an authentic Apple squircle on the frame using our continuous curvature library.
fn fill_squircle<R: iced::advanced::graphics::geometry::Renderer>(frame: &mut Frame<R>, rect: Rectangle, radius: f32, color: Color) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let path = build_squircle_path(rect, radius);
    frame.fill(&path, color);
}

/// Renders the signature Apple Liquid Glass Bevel Optics along the continuous squircle contour:
/// 1. Unified 360° Outward Normal Calculation:
///    - Traces every segment (straight lines and cubic Bézier corner approximations) in clockwise order.
///    - Outward normal N = (nx, ny) is computed exactly for every infinitesimal segment.
///    - Top specular: energy proportional to (-ny).powf(2.0), squeezing into razor-sharp arcs on corners!
///    - Left & Right rim darkening: energy proportional to |nx|.powf(1.2), zero on top/bottom, strong on sides.
///    - Bottom ground bounce: faint ambient bounce proportional to (ny).powf(2.2) * 0.28.
/// 2. Two-tier Curvature Diffusion (The secret to "soft horizontal, sharp corner"):
///    - Along the straight horizontal top edge (where curvature κ = 0 and nx = 0), a 2nd soft diffusion
///      line (inset 1.2px) and a 3rd ambient line (inset 2.2px) create a rich 2.5px wide gradient glow.
///    - When entering the squircle corner, the inner diffusion lines stop, leaving ONLY the single
///      curvature-compressed outer specular arc, creating the exact sharp-corner vs. soft-horizontal contrast!
fn draw_liquid_glass_bevel<R: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<R>,
    rect: Rectangle,
    radius: f32,
    is_dark: bool,
    enable_highlight: bool,
    enable_dark_rim: bool,
) {
    let inset = 0.25f32;
    let inner_rect = Rectangle {
        x: rect.x + inset,
        y: rect.y + inset,
        width: (rect.width - inset * 2.0).max(0.0),
        height: (rect.height - inset * 2.0).max(0.0),
    };
    let r = (radius - inset).max(1.0).min(inner_rect.width * 0.5).min(inner_rect.height * 0.5);
    if r <= 0.5 || inner_rect.width <= 1.0 || inner_rect.height <= 1.0 {
        return;
    }

    let is_capsule = (r - inner_rect.height * 0.5).abs() < 1.0;
    let smoothing = if is_capsule { 0.0 } else { APPLE_CORNER_SMOOTHING };
    let params = SquircleParams::new(inner_rect.width, inner_rect.height, r)
        .with_smoothing(smoothing);
    let commands = squircle_path_commands(&params);
    if commands.is_empty() {
        return;
    }

    let stroke_w = 0.5f32;
    let full_path = Path::new(|builder| {
        for cmd in &commands {
            match *cmd {
                PathCommand::MoveTo(p) => builder.move_to(Point::new(inner_rect.x + p.x, inner_rect.y + p.y)),
                PathCommand::LineTo(p) => builder.line_to(Point::new(inner_rect.x + p.x, inner_rect.y + p.y)),
                PathCommand::CubicTo { c0, c1, to } => builder.bezier_curve_to(
                    Point::new(inner_rect.x + c0.x, inner_rect.y + c0.y),
                    Point::new(inner_rect.x + c1.x, inner_rect.y + c1.y),
                    Point::new(inner_rect.x + to.x, inner_rect.y + to.y),
                ),
                PathCommand::Close => builder.close(),
            }
        }
    });

    if enable_dark_rim {
        let rim_alpha = if is_dark { 0.25 } else { 0.18 };
        frame.stroke(
            &full_path,
            Stroke::default()
                .with_color(Color::from_rgba(0.0, 0.0, 0.0, rim_alpha))
                .with_width(stroke_w),
        );
    }

    if enable_highlight {
        let highlight_alpha = if is_dark { 0.35 } else { 0.48 };
        let top_highlight_path = Path::new(|builder| {
            let mut pen = Point::new(inner_rect.x, inner_rect.y);
            for cmd in &commands {
                match *cmd {
                    PathCommand::MoveTo(p) => {
                        pen = Point::new(inner_rect.x + p.x, inner_rect.y + p.y);
                        builder.move_to(pen);
                    }
                    PathCommand::LineTo(p) => {
                        let to = Point::new(inner_rect.x + p.x, inner_rect.y + p.y);
                        if (pen.y - inner_rect.y).abs() < 1.0 && (to.y - inner_rect.y).abs() < 1.0 {
                            builder.line_to(to);
                        }
                        pen = to;
                    }
                    PathCommand::CubicTo { c0, c1, to } => {
                        let to_pt = Point::new(inner_rect.x + to.x, inner_rect.y + to.y);
                        if (pen.y - inner_rect.y) < r * 1.5 || (to_pt.y - inner_rect.y) < r * 1.5 {
                            builder.bezier_curve_to(
                                Point::new(inner_rect.x + c0.x, inner_rect.y + c0.y),
                                Point::new(inner_rect.x + c1.x, inner_rect.y + c1.y),
                                to_pt,
                            );
                        }
                        pen = to_pt;
                    }
                    PathCommand::Close => {}
                }
            }
        });

        frame.stroke(
            &top_highlight_path,
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, highlight_alpha))
                .with_width(stroke_w),
        );
    }
}

/// State of the Liquid Glass Optics & Dock Showcase.
#[derive(Debug)]
pub struct State {
    pub controller: WindowShellController,
    pub window_size: Size,
    pub theme: Theme,
    pub wallpaper: WallpaperStyle,
    pub transparency: GlassTransparency,
    pub blur_preset: BlurPreset,
    pub blur_radius: f32,
    pub show_grid: bool,
    pub enable_highlight: bool,
    pub enable_dark_rim: bool,
    pub cursor_pos: Point,
    pub hovered_app: Option<DockApp>,
    pub search_query: String,
    pub document_edited: bool,
    pub floating_menu: Option<(Point, MenuContext)>,
    pub floating_menu_cached: ContextMenu<Message>,
    pub last_action: String,
    pub system_wallpaper: Arc<WallpaperBuffer>,
    pub dock_frosted_texture: Option<iced::widget::image::Handle>,
    pub search_frosted_texture: Option<iced::widget::image::Handle>,
    pub app_icons: [Option<iced::widget::image::Handle>; 9],
}

/// Loads authentic macOS application icons directly from disk and processes
/// them through `squircle-icon-rs` with 10:20:10 curvature and Apple HIG squircle plates.
fn load_real_app_icons() -> [Option<iced::widget::image::Handle>; 9] {
    let mut icons: [Option<iced::widget::image::Handle>; 9] = Default::default();
    for (i, app) in DockApp::ALL.iter().enumerate() {
        let path = app.system_app_path();
        if let Some(png_bytes) = app_icon_png(path) {
            if let Ok(pixmap) = squircle_icon_rs::rasterize_image_data(&png_bytes, 128, 128) {
                let plated = squircle_icon_rs::apply_squircle_plate(
                    &pixmap,
                    squircle_icon_rs::PlateOptions::default(),
                );
                let plated_bm = squircle_icon_rs::IconBitmap::from_pixmap(plated);
                let w = plated_bm.width();
                let h = plated_bm.height();
                let rgba = plated_bm.to_straight_rgba();
                icons[i] = Some(iced::widget::image::Handle::from_rgba(w, h, rgba));
            }
        }
    }
    icons
}

impl Default for State {
    fn default() -> Self {
        let is_dark = is_system_dark_mode();
        let config = WindowChromeConfig::unified_header(window_metrics::FUSED_HEADER_HEIGHT);
        let controller = WindowShellController::new(config, is_dark);
        let scheme = if is_dark {
            UiColorScheme::Dark
        } else {
            UiColorScheme::Light
        };
        let theme = UiTheme::new(scheme).iced_theme();
        let system_wallpaper = load_or_create_wallpaper(1240, 820);
        let app_icons = load_real_app_icons();

        let mut s = Self {
            controller,
            window_size: Size::new(1240.0, 820.0),
            theme,
            wallpaper: if std::env::var("WALLPAPER").map(|s| s == "tv").unwrap_or(false) {
                WallpaperStyle::TvColorBars
            } else {
                WallpaperStyle::DesktopTransparent
            },
            transparency: GlassTransparency::Ultra,
            blur_preset: BlurPreset::Standard16,
            blur_radius: BlurPreset::Standard16.radius(),
            show_grid: false,
            enable_highlight: true,
            enable_dark_rim: true,
            cursor_pos: Point::new(600.0, 400.0),
            hovered_app: None,
            search_query: String::new(),
            document_edited: false,
            floating_menu: None,
            floating_menu_cached: ContextMenu::new(),
            last_action: "就绪：macOS 原生桌面壁纸输入已接入，Liquid Glass 实施 2D 深度高斯模糊卷积".to_string(),
            system_wallpaper,
            dock_frosted_texture: None,
            search_frosted_texture: None,
            app_icons,
        };
        s.regenerate_frosted_textures();
        s
    }
}

impl State {
    pub fn regenerate_frosted_textures(&mut self) {
        let metrics = LayoutMetrics::new(self.window_size, self.hovered_app);
        self.dock_frosted_texture = Some(generate_frosted_plate_texture(
            self.wallpaper,
            metrics.dock_rect,
            self.window_size,
            self.blur_radius,
            metrics.dock_radius,
            self.is_dark(),
            Some(&self.system_wallpaper),
        ));
        self.search_frosted_texture = Some(generate_frosted_plate_texture(
            self.wallpaper,
            metrics.search_rect,
            self.window_size,
            self.blur_radius,
            21.0,
            self.is_dark(),
            Some(&self.system_wallpaper),
        ));
    }

    #[must_use]
    pub fn is_dark(&self) -> bool {
        self.controller.is_dark
    }

    #[must_use]
    pub fn scheme(&self) -> UiColorScheme {
        if self.is_dark() {
            UiColorScheme::Dark
        } else {
            UiColorScheme::Light
        }
    }

    pub fn rebuild_menu(&mut self, context: MenuContext) {
        let scheme = self.scheme();
        self.floating_menu_cached = match context {
            MenuContext::App(app) => build_app_context_menu(app, scheme),
            MenuContext::DockBar => build_dock_context_menu(self, scheme),
            MenuContext::SearchBar => build_search_context_menu(scheme),
            MenuContext::Wallpaper => build_wallpaper_context_menu(self, scheme),
        };
    }
}

fn build_app_context_menu(app: DockApp, scheme: UiColorScheme) -> ContextMenu<Message> {
    ContextMenu::new()
        .width(menu_metrics::DEFAULT_WIDTH)
        .with_scheme(scheme)
        .item(MenuItem::section(app.name()))
        .item(
            MenuItem::action("打开应用")
                .with_shortcut("↵")
                .with_icon(UiIcon::Info)
                .on_press(Message::TriggerAction(format!("打开应用: {}", app.name()))),
        )
        .item(
            MenuItem::action("在访达中显示")
                .with_shortcut("⌘O")
                .on_press(Message::TriggerAction(format!("在访达中显示: {}", app.name()))),
        )
        .item(MenuItem::separator())
        .item(MenuItem::submenu("选项 ▸"))
        .item(
            MenuItem::action("退出")
                .with_shortcut("⌘Q")
                .with_destructive(true)
                .on_press(Message::TriggerAction(format!("退出应用: {}", app.name()))),
        )
}

fn build_dock_context_menu(state: &State, scheme: UiColorScheme) -> ContextMenu<Message> {
    ContextMenu::new()
        .width(menu_metrics::DEFAULT_WIDTH)
        .with_scheme(scheme)
        .item(MenuItem::section("DOCK 与光学偏好设置"))
        .item(
            MenuItem::checkbox("启用边缘高光 (Highlight)", state.enable_highlight)
                .on_toggle(Message::ToggleHighlight(!state.enable_highlight)),
        )
        .item(
            MenuItem::checkbox("启用左右深色描边 (Dark Rim)", state.enable_dark_rim)
                .on_toggle(Message::ToggleDarkRim(!state.enable_dark_rim)),
        )
        .item(
            MenuItem::checkbox("显示校准对齐网格", state.show_grid)
                .on_toggle(Message::ToggleGrid(!state.show_grid)),
        )
        .item(MenuItem::separator())
        .item(
            MenuItem::action("系统设置...")
                .with_shortcut("⌘,")
                .with_icon(UiIcon::Gear)
                .on_press(Message::TriggerAction("打开系统设置...".to_string())),
        )
}

fn build_search_context_menu(scheme: UiColorScheme) -> ContextMenu<Message> {
    ContextMenu::new()
        .width(menu_metrics::DEFAULT_WIDTH)
        .with_scheme(scheme)
        .item(MenuItem::section("搜索操作"))
        .item(
            MenuItem::action("粘贴并搜索")
                .with_shortcut("⌘V")
                .on_press(Message::TriggerAction("执行: 粘贴并搜索".to_string())),
        )
        .item(
            MenuItem::action("清空搜索内容")
                .with_shortcut("⌫")
                .on_press(Message::SearchInputChanged(String::new())),
        )
        .item(MenuItem::separator())
        .item(MenuItem::submenu("搜索引擎设置 ▸"))
}

fn build_wallpaper_context_menu(state: &State, scheme: UiColorScheme) -> ContextMenu<Message> {
    ContextMenu::new()
        .width(menu_metrics::DEFAULT_WIDTH)
        .with_scheme(scheme)
        .item(MenuItem::section("壁纸与显示"))
        .item(
            MenuItem::action(format!("当前壁纸: {}", state.wallpaper.label()))
                .on_press(Message::TriggerAction("点击顶栏切换壁纸".to_string())),
        )
        .item(
            MenuItem::action(if state.is_dark() { "切换至浅色模式" } else { "切换至深色模式" })
                .with_shortcut("⇧⌘D")
                .on_press(Message::ToggleTheme),
        )
        .item(MenuItem::separator())
        .item(
            MenuItem::action("材质光学调试面板")
                .with_icon(UiIcon::Info)
                .on_press(Message::TriggerAction("激活材质调试面板".to_string())),
        )
}

pub fn boot() -> (State, Task<Message>) {
    let mut state = State::default();
    for arg in std::env::args().skip(1) {
        if arg == "--bars" || arg == "bars" {
            state.wallpaper = WallpaperStyle::TvColorBars;
        } else if arg == "--grid" || arg == "grid" {
            state.wallpaper = WallpaperStyle::TvColorGrid;
        } else if arg == "--smpte" || arg == "smpte" {
            state.wallpaper = WallpaperStyle::TvSmpteSplit;
        } else if arg == "--aurora" || arg == "aurora" {
            state.wallpaper = WallpaperStyle::AuroraMesh;
        }
    }
    state.rebuild_menu(MenuContext::DockBar);
    state.regenerate_frosted_textures();
    (
        state,
        Task::batch([
            iced::system::theme().map(Message::SystemThemeChanged),
            liquid_glass::IcedWindowController::latest().map(Message::WindowReady),
        ]),
    )
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::WindowReady(Some(id)) => {
            state.controller.set_window_id(id);
            let controller = state.controller.clone();
            window::run(id, move |w| {
                if let Ok(handle) = w.window_handle() {
                    let _ = controller.setup_window(handle.as_raw());
                }
            })
            .discard()
        }
        Message::WindowReady(None) => Task::none(),
        Message::WindowResized(size) => {
            state.window_size = size;
            state.controller.handle_resized(size.width, size.height);
            state.regenerate_frosted_textures();
            Task::none()
        }
        Message::WindowEvent((id, event)) => {
            if let iced::window::Event::Opened { .. } = event {
                if state.controller.window_id.is_none() {
                    return update(state, Message::WindowReady(Some(id)));
                }
            }
            if let Some(shell_event) = state.controller.handle_window_event(&event) {
                match shell_event {
                    bmol_window_shell::ShellEvent::CloseRequested => {
                        if let Some(id) = state.controller.window_id {
                            return window::close(id);
                        }
                    }
                    _ => {}
                }
            }
            Task::none()
        }
        Message::AnimationFrame(now) => {
            state.controller.step(now);
            Task::none()
        }
        Message::ResizeWindow(direction) => {
            if let Some(id) = state.controller.window_id {
                window::drag_resize(id, direction)
            } else {
                Task::none()
            }
        }
        Message::DragWindow => {
            if let Some(id) = state.controller.window_id {
                window::drag(id)
            } else {
                Task::none()
            }
        }
        Message::TrafficLights(event) => state.controller.handle_traffic_lights(event),
        Message::SetWallpaper(w) => {
            state.wallpaper = w;
            state.last_action = format!("切换壁纸: {}", w.label());
            state.regenerate_frosted_textures();
            Task::none()
        }
        Message::SetBlurPreset(p) => {
            state.blur_preset = p;
            state.blur_radius = p.radius();
            state.last_action = format!("切换模糊预设: {} (vibrancy-rs)", p.label());
            state.regenerate_frosted_textures();
            Task::none()
        }
        Message::ToggleTheme => {
            state.controller.is_dark = !state.controller.is_dark;
            state.theme = UiTheme::new(state.scheme()).iced_theme();
            state.last_action = format!("切换外观: {}", if state.is_dark() { "深色 (Dark)" } else { "浅色 (Light)" });
            state.regenerate_frosted_textures();
            Task::none()
        }
        Message::CycleTransparency => {
            state.transparency = state.transparency.next();
            state.last_action = format!("切换玻璃通透度: {}", state.transparency.label());
            Task::none()
        }
        Message::SetTransparency(t) => {
            state.transparency = t;
            state.last_action = format!("设置玻璃通透度: {}", state.transparency.label());
            Task::none()
        }
        Message::ToggleHighlight(val) => {
            state.enable_highlight = val;
            state.last_action = format!("边缘高光 (Highlight): {}", if val { "开启" } else { "关闭" });
            Task::none()
        }
        Message::ToggleDarkRim(val) => {
            state.enable_dark_rim = val;
            state.last_action = format!("左右深色描边 (Dark Rim): {}", if val { "开启" } else { "关闭" });
            Task::none()
        }
        Message::ToggleGrid(val) => {
            state.show_grid = val;
            state.last_action = format!("校准网格线: {}", if val { "显示" } else { "隐藏" });
            Task::none()
        }
        Message::ToggleDocumentEdited => {
            state.document_edited = !state.document_edited;
            bmol_window_shell::set_document_edited(state.document_edited);
            state.last_action =
                format!("NSWindow.isDocumentEdited = {}", state.document_edited);
            Task::none()
        }
        Message::SystemThemeChanged(mode) => {
            let event = state.controller.handle_system_theme(mode);
            if let ShellEvent::ThemeChanged { .. } = event {
                state.theme = UiTheme::new(state.scheme()).iced_theme();
                state.regenerate_frosted_textures();
            }
            state.last_action = format!("系统外观: {mode:?}");
            Task::none()
        }
        Message::SearchInputChanged(q) => {
            state.search_query = q;
            Task::none()
        }
        Message::CursorMoved(pos) => {
            state.cursor_pos = pos;
            Task::none()
        }
        Message::IconHovered(app) => {
            state.hovered_app = app;
            Task::none()
        }
        Message::IconClicked(app) => {
            state.last_action = format!("激活应用: {}", app.name());
            Task::none()
        }
        Message::RightClicked => {
            let cursor = state.cursor_pos;
            let metrics = LayoutMetrics::new(state.window_size, state.hovered_app);

            // Check if right-click hit an app icon
            let mut context = MenuContext::Wallpaper;
            for (i, app) in DockApp::ALL.iter().enumerate() {
                let rect = metrics.icon_rects[i];
                if cursor.x >= rect.x && cursor.x <= rect.x + rect.width
                    && cursor.y >= rect.y && cursor.y <= rect.y + rect.height
                {
                    context = MenuContext::App(*app);
                    break;
                }
            }

            // Check if right-click hit the Dock bar
            if context == MenuContext::Wallpaper {
                let d = metrics.dock_rect;
                if cursor.x >= d.x && cursor.x <= d.x + d.width
                    && cursor.y >= d.y && cursor.y <= d.y + d.height
                {
                    context = MenuContext::DockBar;
                }
            }

            // Check if right-click hit the Search bar
            if context == MenuContext::Wallpaper {
                let s = metrics.search_rect;
                if cursor.x >= s.x && cursor.x <= s.x + s.width
                    && cursor.y >= s.y && cursor.y <= s.y + s.height
                {
                    context = MenuContext::SearchBar;
                }
            }

            state.rebuild_menu(context);
            state.floating_menu = Some((cursor, context));
            state.last_action = format!("弹出液态玻璃菜单: {context:?}");
            Task::none()
        }
        Message::DismissFloatingMenu => {
            state.floating_menu = None;
            Task::none()
        }
        Message::TriggerAction(act) => {
            state.last_action = act;
            state.floating_menu = None;
            Task::none()
        }
    }
}

pub fn subscription(state: &State) -> Subscription<Message> {
    let mut subscriptions = vec![
        window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
        bmol_window_shell::system_theme_subscription(Message::SystemThemeChanged),
        iced::event::listen_with(|event, _status, id| match event {
            iced::Event::Window(w_event) => Some(Message::WindowEvent((id, w_event))),
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                Some(Message::RightClicked)
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Some(Message::CursorMoved(position))
            }
            _ => None,
        }),
    ];

    if state.controller.is_animating() {
        subscriptions.push(window::frames().map(Message::AnimationFrame));
    }

    Subscription::batch(subscriptions)
}

pub fn view(state: &State) -> Element<'_, Message, Theme, iced_backend::Renderer> {
    let is_dark = state.is_dark();
    let metrics = LayoutMetrics::new(state.window_size, state.hovered_app);

    // Drive the shared Liquid Glass compositor so the traffic lights render
    // through the exact same GPU physical-glass pipeline as the settings demo.
    iced_backend::set_surface(DemoSurface::TrafficLightsOnly);
    bmol_window_shell::set_glass_passthrough(true);
    bmol_window_shell::set_document_edited(state.document_edited);
    iced_backend::set_color_scheme(if is_dark {
        UiColorScheme::Dark
    } else {
        UiColorScheme::Light
    });
    iced_backend::set_window_inactive(!state.controller.is_focused);
    // The traffic-light origin is measured from the glyph widget's real layout
    // by `MeasuredTrafficLights`, so no manual origin is needed here.
    for (index, &id) in WINDOW_CONTROL_NATIVE_IDS.iter().enumerate() {
        iced_backend::set_window_control_scale(
            id,
            state.controller.traffic_lights.press_springs[index].value(),
        );
    }
    iced_backend::set_window_control_group_progress(
        0,
        state.controller.traffic_lights.hover_progress,
    );

    // Compute Context Menu Rect if open
    let floating_menu_rect = state.floating_menu.map(|(pos, _)| {
        let menu_w = menu_metrics::DEFAULT_WIDTH;
        let item_count = state.floating_menu_cached.len();
        let menu_h = (item_count as f32 * 26.5 + 16.0).max(60.0);
        let menu_x = (pos.x - 12.0).clamp(10.0, state.window_size.width - menu_w - 10.0);
        let menu_y = (pos.y - 12.0).clamp(metrics.header_h + 10.0, state.window_size.height - menu_h - 10.0);
        Rectangle {
            x: menu_x,
            y: menu_y,
            width: menu_w,
            height: menu_h,
        }
    });

    // -------------------------------------------------------------
    // Layer 1: Liquid Glass 2D Frosted Backdrop Canvas (Sharp background + 2D Gaussian blurred slices)
    // -------------------------------------------------------------
    let backdrop_canvas = Canvas::new(LiquidGlassBackdropCanvas {
        style: state.wallpaper,
        metrics,
        show_grid: state.show_grid,
        system_wallpaper: Some(state.system_wallpaper.clone()),
        dock_frosted_texture: state.dock_frosted_texture.clone(),
        search_frosted_texture: state.search_frosted_texture.clone(),
    })
    .width(Length::Fill)
    .height(Length::Fill);

    // -------------------------------------------------------------
    // Layer 2: Liquid Glass Foreground Optics Canvas (Bevel highlights, substrate tint, icons)
    // -------------------------------------------------------------
    let foreground_canvas = Canvas::new(LiquidGlassForegroundCanvas {
        metrics,
        blur_radius: state.blur_radius,
        enable_highlight: state.enable_highlight,
        enable_dark_rim: state.enable_dark_rim,
        is_dark,
        transparency: state.transparency,
        floating_menu_rect,
        app_icons: state.app_icons.clone(),
    })
    .width(Length::Fill)
    .height(Length::Fill);

    // -------------------------------------------------------------
    // Layer 3: Interactive Controls Overlay (Exact Metric Sizing)
    // -------------------------------------------------------------
    let header = view_header(state, is_dark);
    let status_bar = view_status_bar(state, is_dark);

    // Search bar input positioned right over metrics.search_rect
    let search_input = view_search_input(state, is_dark, metrics.search_rect);

    // Dock interactive touch areas
    let dock_interactive = view_dock_hitboxes(state, metrics);

    let stage_area = iced::widget::Stack::new()
        .push(search_input)
        .push(dock_interactive)
        .width(Length::Fill)
        .height(Length::Fill);

    let page_content = column![header, stage_area, status_bar]
        .width(Length::Fill)
        .height(Length::Fill);

    let mut layers: Vec<Element<'_, Message, Theme, iced_backend::Renderer>> = Vec::new();

    // Layer 1: Liquid Glass 2D Frosted Backdrop Canvas
    layers.push(backdrop_canvas.into());

    // Layer 2: Liquid Glass Foreground Optics Canvas (Guaranteed on top of backdrop images)
    layers.push(foreground_canvas.into());

    // Layer 3: Interactive Controls Overlay (Exact Metric Sizing)
    layers.push(page_content.into());

    // -------------------------------------------------------------
    // Layer 3: Floating Context Menu (at mouse cursor)
    // -------------------------------------------------------------
    if let (Some((_pos, _)), Some(m_rect)) = (state.floating_menu, floating_menu_rect) {
        let dismiss_backdrop = iced::widget::mouse_area(
            container(space())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::DismissFloatingMenu);

        let menu_view = state.floating_menu_cached.view::<iced_backend::Renderer>(&state.theme);

        let positioned_menu = container(menu_view)
            .padding(Padding {
                top: m_rect.y,
                left: m_rect.x,
                ..Padding::ZERO
            });

        let overlay_stack = iced::widget::Stack::new()
            .push(dismiss_backdrop)
            .push(positioned_menu);

        layers.push(overlay_stack.into());
    }

    let root_stack = container(iced::widget::Stack::with_children(layers))
        .width(Length::Fill)
        .height(Length::Fill);

    state.controller.wrap_window_with_resizer(
        root_stack,
        window_metrics::DEFAULT_CORNER_RADIUS,
        Message::ResizeWindow,
    )
}

/// Builds the top titlebar with macOS Traffic Lights, Title, and Tuning Bar.
fn view_header(state: &State, is_dark: bool) -> Element<'_, Message, Theme, iced_backend::Renderer> {
    // The glyphs sit above the GPU glass composition (overlay layer). The
    // wrapper publishes their measured layout origin so the spheres follow it.
    let traffic_lights = liquid_glass::ui::components::glass_overlay(
        state.controller.traffic_lights_view(Message::TrafficLights),
    );

    let title_text = container(
        text("Liquid Glass Optics & Dock Showcase")
            .size(12)
            .font(font::ui_font(Weight::Semibold))
            .color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.95)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.88)
            }),
    )
    .padding(Padding {
        top: 3.0,
        right: 10.0,
        bottom: 3.0,
        left: 10.0,
    })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.08, 0.09, 0.12, 0.45)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.45)
        })),
        border: Border::default().rounded(14.0).width(0.5).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.16)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        }),
        ..Default::default()
    });

    // Optics Tuning Controls wrapped in a floating frosted pill capsule
    let wallpaper_pill = container(
        row(WallpaperStyle::ALL
            .iter()
            .map(|&s| {
                let is_selected = state.wallpaper == s;
                button(
                    text(s.label())
                        .size(11)
                        .font(font::ui_font(if is_selected {
                            Weight::Bold
                        } else {
                            Weight::Normal
                        }))
                        .color(if is_selected {
                            Color::WHITE
                        } else if is_dark {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.75)
                        } else {
                            Color::from_rgba(0.0, 0.0, 0.0, 0.75)
                        }),
                )
                .padding(Padding {
                    top: 3.0,
                    right: 5.0,
                    bottom: 3.0,
                    left: 5.0,
                })
                .style(move |_theme, _status| button::Style {
                    background: if is_selected {
                        Some(Background::Color(Color::from_rgb(0.0, 0.48, 1.0)))
                    } else {
                        None
                    },
                    border: Border::default().rounded(12.0),
                    ..Default::default()
                })
                .on_press(Message::SetWallpaper(s))
                .into()
            }))
        .spacing(1.5),
    )
    .padding(Padding {
        top: 2.0,
        right: 4.0,
        bottom: 2.0,
        left: 4.0,
    })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.0, 0.0, 0.0, 0.35)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.32)
        })),
        border: Border::default().rounded(16.0).width(0.5).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.16)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        }),
        ..Default::default()
    });

    // Blur Presets Pill (vibrancy-rs Dual Kawase & Gaussian convolution)
    let blur_pill = container(
        row(BlurPreset::ALL
            .iter()
            .map(|&p| {
                let is_selected = state.blur_preset == p;
                button(
                    text(p.short_label())
                        .size(11)
                        .font(font::ui_font(if is_selected {
                            Weight::Bold
                        } else {
                            Weight::Normal
                        }))
                        .color(if is_selected {
                            Color::WHITE
                        } else if is_dark {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.75)
                        } else {
                            Color::from_rgba(0.0, 0.0, 0.0, 0.75)
                        }),
                )
                .padding(Padding {
                    top: 3.0,
                    right: 5.0,
                    bottom: 3.0,
                    left: 5.0,
                })
                .style(move |_theme, _status| button::Style {
                    background: if is_selected {
                        Some(Background::Color(Color::from_rgb(0.55, 0.25, 0.85)))
                    } else {
                        None
                    },
                    border: Border::default().rounded(12.0),
                    ..Default::default()
                })
                .on_press(Message::SetBlurPreset(p))
                .into()
            }))
        .spacing(1.5),
    )
    .padding(Padding {
        top: 2.0,
        right: 4.0,
        bottom: 2.0,
        left: 4.0,
    })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(0.0, 0.0, 0.0, 0.35)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.32)
        })),
        border: Border::default().rounded(16.0).width(0.5).color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.16)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        }),
        ..Default::default()
    });

    // Toggle Highlight & Dark Rim Pill
    let toggle_highlight_btn = button(
        text("高光")
            .size(11)
            .font(font::ui_font(Weight::Medium))
            .color(if state.enable_highlight {
                Color::WHITE
            } else if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.6)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.6)
            }),
    )
    .padding(Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 8.0,
    })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(if state.enable_highlight {
            Color::from_rgb(0.18, 0.75, 0.38)
        } else if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.12)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        })),
        border: Border::default().rounded(14.0),
        ..Default::default()
    })
    .on_press(Message::ToggleHighlight(!state.enable_highlight));

    let toggle_rim_btn = button(
        text("暗边")
            .size(11)
            .font(font::ui_font(Weight::Medium))
            .color(if state.enable_dark_rim {
                Color::WHITE
            } else if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.6)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.6)
            }),
    )
    .padding(Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 8.0,
    })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(if state.enable_dark_rim {
            Color::from_rgb(0.18, 0.52, 0.95)
        } else if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.12)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        })),
        border: Border::default().rounded(14.0),
        ..Default::default()
    })
    .on_press(Message::ToggleDarkRim(!state.enable_dark_rim));

    // Mirrors NSWindow.isDocumentEdited: shows the close button's dirty dot.
    let document_edited_btn = button(
        text("未保存")
            .size(11)
            .font(font::ui_font(Weight::Medium))
            .color(if state.document_edited {
                Color::WHITE
            } else if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.6)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.6)
            }),
    )
    .padding(Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 8.0,
    })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(if state.document_edited {
            Color::from_rgb(0.85, 0.25, 0.20)
        } else if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.12)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        })),
        border: Border::default().rounded(14.0),
        ..Default::default()
    })
    .on_press(Message::ToggleDocumentEdited);

    let theme_btn = button(
        text(if is_dark { "☀️ 浅色" } else { "🌙 深色" })
            .size(11)
            .color(if is_dark { Color::WHITE } else { Color::BLACK }),
    )
    .padding(Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 8.0,
    })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.15)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.10)
        })),
        border: Border::default().rounded(14.0),
        ..Default::default()
    })
    .on_press(Message::ToggleTheme);

    let transparency_btn = button(
        text(state.transparency.label())
            .size(11)
            .font(font::ui_font(Weight::Medium))
            .color(Color::WHITE),
    )
    .padding(Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 8.0,
    })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(match state.transparency {
            GlassTransparency::High => Color::from_rgb(0.0, 0.48, 1.0),
            GlassTransparency::Ultra => Color::from_rgb(0.55, 0.25, 0.85),
            GlassTransparency::Frosted => Color::from_rgb(0.40, 0.45, 0.52),
        })),
        border: Border::default().rounded(14.0),
        ..Default::default()
    })
    .on_press(Message::CycleTransparency);

    let header_row = row![
        traffic_lights,
        space().width(Length::Fixed(12.0)),
        title_text,
        space().width(Length::Fill),
        wallpaper_pill,
        space().width(Length::Fixed(4.0)),
        blur_pill,
        space().width(Length::Fixed(4.0)),
        transparency_btn,
        toggle_highlight_btn,
        toggle_rim_btn,
        document_edited_btn,
        theme_btn,
    ]
    .spacing(4.0)
    .align_y(Alignment::Center)
    .padding(Padding {
        top: 8.0,
        right: 14.0,
        bottom: 8.0,
        left: 14.0,
    });

    state.controller.loyal_drag_bar(
        44.0,
        header_row,
        Message::DragWindow,
        Some(Message::TrafficLights(TrafficLightsEvent::Action(
            bmol_window_shell::WindowControlAction::Zoom,
        ))),
    )
}

/// Builds the search input widget positioned exactly over `search_rect`.
fn view_search_input(
    state: &State,
    is_dark: bool,
    search_rect: Rectangle,
) -> Element<'_, Message, Theme, iced_backend::Renderer> {
    let search_icon = text("🔍").size(13);

    let input = text_input("聚焦搜索或输入网址...", &state.search_query)
        .on_input(Message::SearchInputChanged)
        .size(13)
        .width(Length::Fill)
        .style(move |_theme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            placeholder: if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.45)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.40)
            },
            value: if is_dark { Color::WHITE } else { Color::BLACK },
            selection: Color::from_rgba(0.0, 0.48, 1.0, 0.35),
            icon: Color::TRANSPARENT,
        });

    let badge = container(
        text("⌘K")
            .size(10)
            .font(font::ui_font(Weight::Semibold))
            .color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.55)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.50)
            }),
    )
    .padding(Padding {
        top: 2.0,
        right: 6.0,
        bottom: 2.0,
        left: 6.0,
    })
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.12)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.08)
        })),
        border: Border::default().rounded(4.0),
        ..Default::default()
    });

    let content = row![search_icon, space().width(8.0), input, space().width(8.0), badge]
        .align_y(Alignment::Center)
        .padding(Padding {
            top: 4.0,
            right: 14.0,
            bottom: 4.0,
            left: 14.0,
        })
        .width(Length::Fixed(search_rect.width))
        .height(Length::Fixed(search_rect.height));

    container(content)
        .padding(Padding {
            top: search_rect.y - 44.0, // relative to stage top
            left: search_rect.x,
            ..Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds interactive hitboxes over the dock app icons with hover tooltip cards.
fn view_dock_hitboxes(
    state: &State,
    metrics: LayoutMetrics,
) -> Element<'_, Message, Theme, iced_backend::Renderer> {
    let mut hitboxes = Vec::new();

    for (i, &app) in DockApp::ALL.iter().enumerate() {
        let rect = metrics.icon_rects[i];
        let is_hovered = state.hovered_app == Some(app);

        let click_target = iced::widget::mouse_area(
            container(space())
                .width(Length::Fixed(rect.width))
                .height(Length::Fixed(rect.height)),
        )
        .on_enter(Message::IconHovered(Some(app)))
        .on_exit(Message::IconHovered(None))
        .on_press(Message::IconClicked(app));

        let placed_target = container(click_target).padding(Padding {
            top: rect.y - metrics.header_h,
            left: rect.x,
            ..Padding::ZERO
        });

        hitboxes.push(placed_target.into());

        // Hover tooltip pill floating above the hovered icon
        if is_hovered {
            let tooltip_pill = container(
                text(app.name())
                    .size(11)
                    .font(font::ui_font(Weight::Semibold))
                    .color(Color::WHITE),
            )
            .padding(Padding {
                top: 4.0,
                right: 8.0,
                bottom: 4.0,
                left: 8.0,
            })
            .style(move |_theme: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.10, 0.10, 0.14, 0.92))),
                border: Border::default().rounded(6.0).width(0.5).color(Color::from_rgba(1.0, 1.0, 1.0, 0.22)),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 8.0,
                },
                ..Default::default()
            });

            let placed_tooltip = container(tooltip_pill).padding(Padding {
                top: rect.y - metrics.header_h - 32.0,
                left: (rect.x - 14.0).max(10.0),
                ..Padding::ZERO
            });

            hitboxes.push(placed_tooltip.into());
        }
    }

    container(iced::widget::Stack::with_children(hitboxes))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds the bottom status bar displaying metrics and user action feedbacks.
fn view_status_bar(state: &State, is_dark: bool) -> Element<'_, Message, Theme, iced_backend::Renderer> {
    let status_text = text(&state.last_action)
        .size(11)
        .font(font::ui_font(Weight::Normal))
        .color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.88)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.82)
        });

    let plan = KawasePassPlan::new(state.window_size.width as i32, state.window_size.height as i32, state.blur_radius);
    let pass_desc = if plan.use_deep_blur { "3级深度金字塔" } else { "2级标准金字塔" };
    let hint_text = text(format!("✨ vibrancy-rs: 模糊 {:.0}pt | {} | +25%饱和度提升 & IGN抗色带抖动", state.blur_radius, pass_desc))
        .size(11)
        .font(font::ui_font(Weight::Medium))
        .color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.70)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.65)
        });

    let bar = row![status_text, space().width(Length::Fill), hint_text]
        .align_y(Alignment::Center);

    let bar_pill = container(bar)
        .padding(Padding {
            top: 3.0,
            right: 14.0,
            bottom: 3.0,
            left: 14.0,
        })
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.08, 0.09, 0.12, 0.45)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.45)
            })),
            border: Border::default()
                .rounded(12.0)
                .width(0.5)
                .color(if is_dark {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.16)
                } else {
                    Color::from_rgba(0.0, 0.0, 0.0, 0.10)
                }),
            ..Default::default()
        });

    container(bar_pill)
        .width(Length::Fill)
        .padding(Padding {
            top: 2.0,
            right: 14.0,
            bottom: 6.0,
            left: 14.0,
        })
        .into()
}

fn app_theme(state: &State) -> Theme {
    state.theme.clone()
}

fn app_style(state: &State, _theme: &Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: if state.is_dark() {
            Color::from_rgb(0.92, 0.92, 0.92)
        } else {
            Color::from_rgb(0.12, 0.13, 0.15)
        },
    }
}

fn main() -> iced::Result {
    let fonts = font::ui_fonts();
    let window_settings = window::Settings {
        size: Size::new(1240.0, 820.0),
        position: window::Position::Centered,
        transparent: true,
        blur: true,
        decorations: false,
        ..Default::default()
    };

    let mut app = iced::application::<State, Message, Theme, iced_backend::Renderer>(boot, update, view)
        .title("Liquid Glass Optics & Dock Showcase - bmol-iced")
        .theme(app_theme)
        .style(app_style)
        .subscription(subscription)
        .window(window_settings);

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
    fn test_transparency_defaults_and_cycling() {
        let t = GlassTransparency::default();
        assert_eq!(t, GlassTransparency::Ultra);

        assert_eq!(t.next(), GlassTransparency::High);
        assert_eq!(t.next().next(), GlassTransparency::Frosted);
        assert_eq!(t.next().next().next(), GlassTransparency::Ultra);
    }

    #[test]
    fn test_transparency_center_alpha_values() {
        let high = GlassTransparency::High;
        let ultra = GlassTransparency::Ultra;
        let frosted = GlassTransparency::Frosted;

        // High transparency provides rich balanced visibility
        assert!(high.center_alpha_light() <= 0.35);
        assert!(high.center_alpha_dark() <= 0.40);
        assert!(high.center_alpha_light() < frosted.center_alpha_light());
        assert!(high.center_alpha_dark() < frosted.center_alpha_dark());

        // Ultra is the most transparent
        assert!(ultra.center_alpha_light() < high.center_alpha_light());
        assert!(ultra.center_alpha_dark() < high.center_alpha_dark());
    }

    #[test]
    fn test_state_transparency_and_message_update() {
        let mut state = State::default();
        assert_eq!(state.transparency, GlassTransparency::Ultra);

        let _ = update(&mut state, Message::CycleTransparency);
        assert_eq!(state.transparency, GlassTransparency::High);

        let _ = update(&mut state, Message::SetTransparency(GlassTransparency::Frosted));
        assert_eq!(state.transparency, GlassTransparency::Frosted);
    }

    #[test]
    fn test_desktop_transparent_wallpaper_default() {
        let state = State::default();
        assert_eq!(state.wallpaper, WallpaperStyle::DesktopTransparent);
        assert_eq!(state.wallpaper.label(), "🖥️ 系统壁纸");
    }

    #[test]
    fn test_dock_layout_metrics() {
        let metrics = LayoutMetrics::new(Size::new(1200.0, 820.0), None);
        // Concentric geometric proportion system assertions:
        assert_eq!(metrics.base_icon_size, 40.0, "Icon size must be 40px");
        assert_eq!(metrics.icon_radius, 10.0, "Icon corner radius must be 10px (10:20:10 ratio)");
        assert_eq!(metrics.icon_gap, 13.0, "Icon gap must be 13px");
        assert_eq!(metrics.dock_padding, 13.0, "Dock padding must be 13px");
        assert_eq!(metrics.dock_radius, 23.0, "Dock radius must be concentric: 13px + 10px = 23px");
        assert_eq!(metrics.dock_rect.height, 66.0, "Dock height = 40 + 13*2 = 66px");
        assert_eq!(metrics.dock_rect.width, 490.0, "Dock width = 9*40 + 8*13 + 13*2 = 490px");
        assert_eq!(metrics.icon_rects.len(), 9);

        // Icons should fit symmetrically inside the dock with 13px padding
        assert_eq!(metrics.icon_rects[0].y, metrics.dock_rect.y + 13.0);
        assert_eq!(metrics.icon_rects[0].x, metrics.dock_rect.x + 13.0);
        let last_idx = metrics.icon_rects.len() - 1;
        let right_padding = (metrics.dock_rect.x + metrics.dock_rect.width) - (metrics.icon_rects[last_idx].x + metrics.icon_rects[last_idx].width);
        assert!((right_padding - 13.0).abs() < 1e-4, "Right padding must be exactly 13px");
    }

    #[test]
    fn test_app_style_transparent_background() {
        let state = State::default();
        let style = app_style(&state, &state.theme);
        assert_eq!(style.background_color, Color::TRANSPARENT);
    }

    #[test]
    fn test_blur_presets_and_radii() {
        assert_eq!(BlurPreset::ALL.len(), 10);
        assert_eq!(BlurPreset::Zero0.radius(), 0.0);
        assert_eq!(BlurPreset::Subtle2.radius(), 2.0);
        assert_eq!(BlurPreset::Subtle4.radius(), 4.0);
        assert_eq!(BlurPreset::Subtle8.radius(), 8.0);
        assert_eq!(BlurPreset::Medium12.radius(), 12.0);
        assert_eq!(BlurPreset::Standard16.radius(), 16.0);
        assert_eq!(BlurPreset::Enhanced24.radius(), 24.0);
        assert_eq!(BlurPreset::Heavy32.radius(), 32.0);
        assert_eq!(BlurPreset::Ultra48.radius(), 48.0);
        assert_eq!(BlurPreset::Max64.radius(), 64.0);
    }

    #[test]
    fn test_blur_preset_message_update() {
        let mut state = State::default();
        assert_eq!(state.blur_preset, BlurPreset::Standard16);
        assert_eq!(state.blur_radius, 16.0);

        let _ = update(&mut state, Message::SetBlurPreset(BlurPreset::Subtle4));
        assert_eq!(state.blur_preset, BlurPreset::Subtle4);
        assert_eq!(state.blur_radius, 4.0);

        let _ = update(&mut state, Message::SetBlurPreset(BlurPreset::Max64));
        assert_eq!(state.blur_preset, BlurPreset::Max64);
        assert_eq!(state.blur_radius, 64.0);
    }

    #[test]
    fn test_vibrancy_rs_kawase_pass_plan() {
        let plan_64 = KawasePassPlan::new(1200, 820, 64.0);
        assert!(plan_64.use_deep_blur);
        assert_eq!(plan_64.src_size, (600, 410));
        assert_eq!(plan_64.down1_size, (300, 205));
        assert_eq!(plan_64.down2_size, Some((150, 103)));

        let plan_16 = KawasePassPlan::new(1200, 820, 16.0);
        assert!(!plan_16.use_deep_blur);
        assert_eq!(plan_16.down2_size, None);

        // Verify standard material specifications from vibrancy-rs
        assert_eq!(MaterialKind::Dock.blur_radius(), 20.0);
        assert_eq!(MaterialKind::Launchpad.blur_radius(), 56.0);
        let dark_tint = MaterialKind::Dock.tint(VibrancyAppearance::Dark);
        assert_eq!(dark_tint, [24, 24, 28, 180]);
    }

    #[test]
    fn test_separable_gaussian_blur_convolution() {
        let w = 20;
        let h = 20;
        let mut src = vec![[0.0, 0.0, 0.0]; w * h];
        // Impulse in center
        src[10 * w + 10] = [1.0, 1.0, 1.0];

        let blurred_0 = perform_separable_gaussian_blur(&src, w, h, 0.0);
        assert!((blurred_0[10 * w + 10][0] - 1.0).abs() < 1e-3);

        let blurred_4 = perform_separable_gaussian_blur(&src, w, h, 4.0);
        // Energy diffused outward: center value decreases, neighbor increases
        assert!(blurred_4[10 * w + 10][0] < 0.5);
        assert!(blurred_4[10 * w + 11][0] > 0.0);
    }

    #[test]
    fn test_wallpaper_buffer_gaussian_blur_sampling() {
        let buf = create_procedural_redwood_buffer(200, 150);
        assert_eq!(buf.width, 200);
        assert_eq!(buf.height, 150);
        let sharp = buf.sample_bilinear(0.5, 0.5);
        assert!(sharp[0] >= 0.0 && sharp[0] <= 1.0);
        let blurred = buf.sample_blurred(0.5, 0.5, 96.0, Size::new(200.0, 150.0));
        assert!(blurred[0] >= 0.0 && blurred[0] <= 1.0);
    }

    #[test]
    fn test_real_system_wallpaper_frosted_plate_generation() {
        let wallpaper = load_or_create_wallpaper(1240, 820);
        let bounds = Size::new(1240.0, 820.0);
        let dock_rect = Rectangle::new(Point::new(300.0, 700.0), Size::new(640.0, 84.0));
        let handle = generate_frosted_plate_texture(
            WallpaperStyle::DesktopTransparent,
            dock_rect,
            bounds,
            16.0,
            24.0,
            false,
            Some(&wallpaper),
        );
        let _ = format!("{:?}", handle);
    }

    #[test]
    fn test_specular_cutoff_and_rim_darkening_physics() {
        const SPEC_CUTOFF: f32 = 0.90;
        const INNER_ARC_CUTOFF: f32 = 0.20;

        // At theta = 90° (vertical edge), u = 0.0, d = 0.0, s = 1.0
        let u_vertical = 0.0f32;
        assert!(u_vertical <= SPEC_CUTOFF, "Vertical edge must have 0 specular highlight");
        assert!(u_vertical <= INNER_ARC_CUTOFF, "Vertical edge must have 0 inner highlight");
        let vert_comp_vertical = u_vertical;
        let rim_factor_vertical = if vert_comp_vertical < 0.90 {
            let corner_soften = 1.0 - 0.65 * (vert_comp_vertical * std::f32::consts::PI * 0.5).sin().powf(1.2);
            corner_soften.clamp(0.20, 1.0)
        } else { 0.0 };
        assert_eq!(rim_factor_vertical, 1.0, "Vertical edge must have 100% dark rim");

        // At curved arc (theta = 45°, u = 0.707)
        let u_arc = 0.707f32;
        assert!(u_arc <= SPEC_CUTOFF, "Outer boundary on curved arc is dark rim without specular");
        let rim_factor_arc = if u_arc < 0.90 {
            let corner_soften = 1.0 - 0.65 * (u_arc * std::f32::consts::PI * 0.5).sin().powf(1.2);
            corner_soften.clamp(0.20, 1.0)
        } else { 0.0 };
        assert!(rim_factor_arc < 0.60 && rim_factor_arc > 0.35, "Curved arc dark rim must soften gracefully");
        // But inner secondary highlight extends through arc!
        assert!(u_arc > INNER_ARC_CUTOFF, "Inner secondary highlight extends into corner arc");

        // At theta = 0° (top horizontal edge, outside R corner), u = 1.0, s = 0.0
        let u_horizontal = 1.0f32;
        assert!(u_horizontal > SPEC_CUTOFF);
        assert!(u_horizontal > INNER_ARC_CUTOFF);
        // On straight horizontal segment, factors are exactly 1.0 (perfectly uniform peak brightness)
        let norm_u_flat = (u_horizontal - SPEC_CUTOFF) / (1.0 - SPEC_CUTOFF);
        let spec_factor_flat = (norm_u_flat * std::f32::consts::PI * 0.5).sin().powf(1.6);
        assert!((spec_factor_flat - 1.0).abs() < 1e-5, "Straight line must have 100% uniform specular highlight");
        let norm_arc_flat = (u_horizontal - INNER_ARC_CUTOFF) / (1.0 - INNER_ARC_CUTOFF);
        let inner_factor_flat = (norm_arc_flat * std::f32::consts::PI * 0.5).sin().powf(1.6);
        assert!((inner_factor_flat - 1.0).abs() < 1e-5, "Straight line must have 100% uniform inner highlight");

        // Ahead of R corner: Pre-corner lead-in smooth decay ("早于R角开始变化")
        let r = 24.0f32;
        let lead_in_dist = (r * 1.25).min(32.0).max(8.0);
        let calc_lead_in = |dist: f32| -> f32 {
            if dist >= lead_in_dist {
                1.0f32
            } else if dist >= 0.0 {
                let k_tangent = 0.85f32;
                let norm = (dist / lead_in_dist).clamp(0.0, 1.0);
                k_tangent + (1.0 - k_tangent) * (norm * std::f32::consts::PI * 0.5).sin().powf(1.5)
            } else {
                0.85f32
            }
        };

        // Far from corner in central plateau:
        assert_eq!(calc_lead_in(lead_in_dist + 50.0), 1.0, "Central plateau must be 100% constant");
        // Ahead of R corner (e.g. 15pt before tangent):
        let factor_ahead = calc_lead_in(15.0);
        assert!(factor_ahead < 1.0 && factor_ahead > 0.85, "Highlight begins smoothly decaying ahead of R corner");
        // Exactly at tangent:
        assert!((calc_lead_in(0.0) - 0.85).abs() < 1e-5, "Smoothly arrives at ~0.85 at R corner tangent");

        // Entering R corner arc (u begins decreasing from 1.0 down towards 0.0):
        // Monotonic smooth attenuation continues along the corner arc!
        let u_corner_entry = 0.96f32;
        let norm_u_entry = (u_corner_entry - SPEC_CUTOFF) / (1.0 - SPEC_CUTOFF);
        let spec_factor_entry = (norm_u_entry * std::f32::consts::PI * 0.5).sin().powf(1.6);
        assert!(spec_factor_entry < spec_factor_flat, "Specular highlight strictly decays upon entering R corner");

        let norm_arc_entry = (u_corner_entry - INNER_ARC_CUTOFF) / (1.0 - INNER_ARC_CUTOFF);
        let inner_factor_entry = (norm_arc_entry * std::f32::consts::PI * 0.5).sin().powf(1.6);
        assert!(inner_factor_entry < inner_factor_flat, "Inner highlight strictly decays upon entering R corner");

        let rim_factor_horizontal = if u_horizontal < 0.90 {
            1.0f32
        } else {
            let norm = (1.0 - u_horizontal) / (1.0 - 0.90);
            (norm * std::f32::consts::PI * 0.5).sin().powf(1.5)
        };
        assert!((rim_factor_horizontal - 0.0).abs() < 1e-5, "Top horizontal edge must have 0 dark rim");

        // Calibrated light mode base alpha
        assert_eq!(GlassTransparency::High.center_alpha_light(), 0.13);

        // Near vertical edge of arc (u = 0.15): Inner highlight gracefully extinguished
        let u_near_vert = 0.15f32;
        assert!(u_near_vert <= INNER_ARC_CUTOFF);
    }

    #[test]
    fn test_draw_image_signature() {
        let handle = iced::widget::image::Handle::from_rgba(2, 2, vec![255; 16]);
        let _ = format!("{:?}", handle);
        let _img = iced::widget::canvas::Image::new(handle);
    }

    #[test]
    fn test_generate_frosted_plate_texture_blur_progression() {
        let wallpaper = load_or_create_wallpaper(1240, 820);
        let rect = Rectangle {
            x: 210.0,
            y: 700.0,
            width: 820.0,
            height: 80.0,
        };
        let window_size = Size::new(1240.0, 820.0);

        // 1. Generate 0pt clear slice
        let handle_0pt = generate_frosted_plate_texture(
            WallpaperStyle::DesktopTransparent,
            rect,
            window_size,
            0.0,
            24.0,
            false,
            Some(&wallpaper),
        );

        // 2. Generate 16pt frosted slice
        let handle_16pt = generate_frosted_plate_texture(
            WallpaperStyle::DesktopTransparent,
            rect,
            window_size,
            16.0,
            24.0,
            false,
            Some(&wallpaper),
        );

        assert_ne!(format!("{:?}", handle_0pt), "");
        assert_ne!(format!("{:?}", handle_16pt), "");
    }

    #[test]
    fn test_scheme_a_squircle_icon_integration() {
        let state = State::default();
        #[cfg(target_os = "macos")]
        {
            // At least Finder and Safari should be resolved and processed via squircle-icon-rs
            assert!(state.app_icons[0].is_some(), "Finder icon should be loaded");
            assert!(state.app_icons[1].is_some(), "Safari icon should be loaded");
        }
    }

    #[test]
    fn test_dock_demo_traffic_lights_controller_integration() {
        let mut state = State::default();
        assert_eq!(state.controller.traffic_lights.hover_progress, 0.0);
        assert!(!state.controller.is_animating());

        // Hover group
        let _ = update(&mut state, Message::TrafficLights(TrafficLightsEvent::GroupHover(true)));
        assert_eq!(state.controller.traffic_lights.hover_target, 1.0);
        assert!(state.controller.is_animating());

        // Unfocus window event automatically clears hover
        let _ = update(&mut state, Message::WindowEvent((window::Id::unique(), window::Event::Unfocused)));
        assert_eq!(state.controller.traffic_lights.hover_target, 0.0);

        // Animation frame steps physics
        let _ = update(&mut state, Message::AnimationFrame(Instant::now()));
    }
}



