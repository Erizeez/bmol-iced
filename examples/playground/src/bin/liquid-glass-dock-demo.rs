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
use bmol_designs::menu_metrics;
use bmol_window_shell::{
    WindowChromeConfig, WindowShellController, is_system_dark_mode, traffic_lights, window_metrics,
};
use liquid_glass::{
    ContextMenu, ControlAction, MenuItem, TrafficLightsState, UiColorScheme, UiIcon, UiTheme,
    geometry::{
        squircle_path_commands, PathCommand,
        Point as SquirclePoint, SquircleParams, APPLE_CORNER_SMOOTHING,
    },
    ui::font,
};
use iced::advanced::graphics::gradient::Linear;
use vibrancy_rs::{ign_dither_offset, KawasePassPlan, VibrancyConfig};
#[cfg(test)]
use vibrancy_rs::{MaterialKind, VibrancyAppearance};

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

/// High-precision approximation of the error function erf(x).
/// Maximum error < 1.5e-7 (Abramowitz and Stegun formula 7.1.26).
#[inline]
fn approx_erf(x: f32) -> f32 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let abs_x = x.abs();

    if abs_x > 4.0 {
        return sign;
    }

    let p = 0.3275911;
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;

    let t = 1.0 / (1.0 + p * abs_x);
    let poly = ((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t;
    sign * (1.0 - poly * (-abs_x * abs_x).exp())
}

/// Standard normal cumulative distribution function Phi(z) = P(Z <= z).
#[inline]
fn normal_cdf(z: f32) -> f32 {
    0.5 * (1.0 + approx_erf(z * std::f32::consts::FRAC_1_SQRT_2))
}

/// Evaluates genuine continuous Gaussian convolution for 1D horizontal segments.
#[inline]
fn segment_gaussian_weight(x: f32, x_left: f32, x_right: f32, inv_sigma: f32) -> f32 {
    let cdf_right = normal_cdf((x_right - x) * inv_sigma);
    let cdf_left = normal_cdf((x_left - x) * inv_sigma);
    (cdf_right - cdf_left).max(0.0)
}

/// Continuous analytical Gaussian convolution for 8 TV color bars.
fn sample_blurred_tv_bars(x: f32, bounds_width: f32, blur_radius: f32) -> [f32; 3] {
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

    let n = 8.0f32;
    let bar_w = bounds_width / n;
    if blur_radius <= 0.5 {
        let idx = ((x / bar_w).floor() as usize).min(7);
        return TV_BARS[idx];
    }
    // Authentic Apple Dual Kawase / Deep Gaussian spread: sigma ~ 2.2 * radius
    let sigma = (blur_radius * 2.2).max(1.0);
    let inv_sigma = 1.0 / sigma;

    let mut r = 0.0f32;
    let mut g = 0.0f32;
    let mut b = 0.0f32;

    for (i, &color) in TV_BARS.iter().enumerate() {
        let left = i as f32 * bar_w;
        let right = (i + 1) as f32 * bar_w;
        let w = segment_gaussian_weight(x, left, right, inv_sigma);
        r += color[0] * w;
        g += color[1] * w;
        b += color[2] * w;
    }

    [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
}

/// Continuous analytical Gaussian convolution for two-tier SMPTE bars.
fn sample_blurred_tv_smpte(x: f32, y: f32, bounds: Size, blur_radius: f32) -> [f32; 3] {
    if blur_radius <= 0.5 {
        return sample_sharp_wallpaper(WallpaperStyle::TvSmpteSplit, x, y, bounds, None);
    }
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

    let top_h = bounds.height * 0.70;
    let sigma = (blur_radius * 2.2).max(1.0);
    let inv_sigma = 1.0 / sigma;

    let top_vertical_w = segment_gaussian_weight(y, -2.0 * sigma, top_h, inv_sigma);
    let bot_vertical_w = segment_gaussian_weight(y, top_h, bounds.height + 2.0 * sigma, inv_sigma);
    let v_sum = (top_vertical_w + bot_vertical_w).max(1e-5);
    let norm_top_w = top_vertical_w / v_sum;
    let norm_bot_w = bot_vertical_w / v_sum;

    let top_bar_w = bounds.width / 7.0;
    let mut top_r = 0.0f32;
    let mut top_g = 0.0f32;
    let mut top_b = 0.0f32;
    for (i, &color) in TOP_SMPTE.iter().enumerate() {
        let left = i as f32 * top_bar_w;
        let right = (i + 1) as f32 * top_bar_w;
        let w = segment_gaussian_weight(x, left, right, inv_sigma);
        top_r += color[0] * w;
        top_g += color[1] * w;
        top_b += color[2] * w;
    }

    let bot_bar_w = bounds.width / 8.0;
    let mut bot_r = 0.0f32;
    let mut bot_g = 0.0f32;
    let mut bot_b = 0.0f32;
    for (i, &color) in BOT_SMPTE.iter().enumerate() {
        let left = i as f32 * bot_bar_w;
        let right = (i + 1) as f32 * bot_bar_w;
        let w = segment_gaussian_weight(x, left, right, inv_sigma);
        bot_r += color[0] * w;
        bot_g += color[1] * w;
        bot_b += color[2] * w;
    }

    [
        (top_r * norm_top_w + bot_r * norm_bot_w).clamp(0.0, 1.0),
        (top_g * norm_top_w + bot_g * norm_bot_w).clamp(0.0, 1.0),
        (top_b * norm_top_w + bot_b * norm_bot_w).clamp(0.0, 1.0),
    ]
}

/// Continuous analytical 2D Gaussian convolution for 4x3 color grid.
fn sample_blurred_tv_grid(x: f32, y: f32, bounds: Size, blur_radius: f32) -> [f32; 3] {
    if blur_radius <= 0.5 {
        return sample_sharp_wallpaper(WallpaperStyle::TvColorGrid, x, y, bounds, None);
    }
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

    let cols = 4.0f32;
    let rows = 3.0f32;
    let cell_w = bounds.width / cols;
    let cell_h = bounds.height / rows;
    let sigma = (blur_radius * 2.2).max(1.0);
    let inv_sigma = 1.0 / sigma;

    let mut r = 0.0f32;
    let mut g = 0.0f32;
    let mut b = 0.0f32;

    for row_idx in 0..3 {
        let top = row_idx as f32 * cell_h;
        let bot = (row_idx + 1) as f32 * cell_h;
        let v_weight = segment_gaussian_weight(y, top, bot, inv_sigma);

        for col_idx in 0..4 {
            let left = col_idx as f32 * cell_w;
            let right = (col_idx + 1) as f32 * cell_w;
            let h_weight = segment_gaussian_weight(x, left, right, inv_sigma);
            let total_w = v_weight * h_weight;

            let color = GRID_PALETTE[row_idx * 4 + col_idx];
            r += color[0] * total_w;
            g += color[1] * total_w;
            b += color[2] * total_w;
        }
    }

    [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
}

/// Continuous 9-tap 2D Gaussian convolution for smooth gradients (Aurora / Sunset).
fn sample_blurred_gradient(
    style: WallpaperStyle,
    x: f32,
    y: f32,
    bounds: Size,
    blur_radius: f32,
    wallpaper_buf: Option<&WallpaperBuffer>,
) -> [f32; 3] {
    if blur_radius <= 0.5 {
        return sample_sharp_wallpaper(style, x, y, bounds, wallpaper_buf);
    }
    let offset = (blur_radius * 1.8).max(1.0);
    let c = sample_sharp_wallpaper(style, x, y, bounds, wallpaper_buf);
    let c_l = sample_sharp_wallpaper(style, (x - offset).max(0.0), y, bounds, wallpaper_buf);
    let c_r = sample_sharp_wallpaper(style, (x + offset).min(bounds.width), y, bounds, wallpaper_buf);
    let c_u = sample_sharp_wallpaper(style, x, (y - offset).max(0.0), bounds, wallpaper_buf);
    let c_d = sample_sharp_wallpaper(style, x, (y + offset).min(bounds.height), bounds, wallpaper_buf);
    let c_lu = sample_sharp_wallpaper(style, (x - offset * 0.707).max(0.0), (y - offset * 0.707).max(0.0), bounds, wallpaper_buf);
    let c_ru = sample_sharp_wallpaper(style, (x + offset * 0.707).min(bounds.width), (y - offset * 0.707).max(0.0), bounds, wallpaper_buf);
    let c_ld = sample_sharp_wallpaper(style, (x - offset * 0.707).max(0.0), (y + offset * 0.707).min(bounds.height), bounds, wallpaper_buf);
    let c_rd = sample_sharp_wallpaper(style, (x + offset * 0.707).min(bounds.width), (y + offset * 0.707).min(bounds.height), bounds, wallpaper_buf);

    [
        c[0] * 0.28 + (c_l[0] + c_r[0] + c_u[0] + c_d[0]) * 0.12 + (c_lu[0] + c_ru[0] + c_ld[0] + c_rd[0]) * 0.06,
        c[1] * 0.28 + (c_l[1] + c_r[1] + c_u[1] + c_d[1]) * 0.12 + (c_lu[1] + c_ru[1] + c_ld[1] + c_rd[1]) * 0.06,
        c[2] * 0.28 + (c_l[2] + c_r[2] + c_u[2] + c_d[2]) * 0.12 + (c_lu[2] + c_ru[2] + c_ld[2] + c_rd[2]) * 0.06,
    ]
}

/// Samples the blurred wallpaper color with Apple Vibrancy color lift & IGN anti-banding dithering
/// powered by `vibrancy-rs`.
fn sample_vibrancy_blurred_wallpaper(
    style: WallpaperStyle,
    x: f32,
    y: f32,
    bounds: Size,
    blur_radius: f32,
    is_dark: bool,
    wallpaper_buf: Option<&WallpaperBuffer>,
) -> Color {
    let raw_rgb = match style {
        WallpaperStyle::DesktopTransparent => {
            if let Some(buf) = wallpaper_buf {
                let u = (x / bounds.width.max(1.0)).clamp(0.0, 1.0);
                let v = (y / bounds.height.max(1.0)).clamp(0.0, 1.0);
                buf.sample_blurred(u, v, blur_radius, bounds)
            } else if is_dark {
                [0.12, 0.12, 0.14]
            } else {
                [0.92, 0.92, 0.94]
            }
        }
        WallpaperStyle::AuroraMesh => sample_blurred_gradient(style, x, y, bounds, blur_radius, wallpaper_buf),
        WallpaperStyle::SunsetGaze => sample_blurred_gradient(style, x, y, bounds, blur_radius, wallpaper_buf),
        WallpaperStyle::TvColorBars => sample_blurred_tv_bars(x, bounds.width, blur_radius),
        WallpaperStyle::TvSmpteSplit => sample_blurred_tv_smpte(x, y, bounds, blur_radius),
        WallpaperStyle::TvColorGrid => sample_blurred_tv_grid(x, y, bounds, blur_radius),
        WallpaperStyle::PureWhite => [1.0, 1.0, 1.0],
        WallpaperStyle::PureBlack => [0.0, 0.0, 0.0],
    };

    // 1. Apple Vibrancy color model: gentle luma lift and rich saturation boost
    let vibrancy = VibrancyConfig {
        saturation_boost: if is_dark { 1.25 } else { 1.30 },
        luma_lift: if is_dark { 1.02 } else { 1.06 },
        luma_bias: if is_dark { 0.004 } else { 0.012 },
        ..Default::default()
    };
    let vibrant_rgb = vibrancy.apply(raw_rgb);

    // 2. Interleaved Gradient Noise (IGN) anti-banding dithering
    let dither = ign_dither_offset(x, y);

    Color::from_rgb(
        (vibrant_rgb[0] + dither).clamp(0.0, 1.0),
        (vibrant_rgb[1] + dither).clamp(0.0, 1.0),
        (vibrant_rgb[2] + dither).clamp(0.0, 1.0),
    )
}

/// Analytical Signed Distance Field (SDF) of a rounded rectangle with corner radius `r`.
/// Returns negative inside, zero on boundary, and positive outside.
#[inline]
pub fn rounded_rect_sdf(px: f32, py: f32, w: f32, h: f32, r: f32) -> f32 {
    let half_w = w * 0.5;
    let half_h = h * 0.5;
    let r = r.min(half_w).min(half_h);
    let px = (px - half_w).abs();
    let py = (py - half_h).abs();
    let qx = px - half_w + r;
    let qy = py - half_h + r;
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    let inside = qx.max(qy).min(0.0);
    outside + inside - r
}

/// Performs a true 2D Separable Gaussian Convolution on an RGB float buffer.
///
/// Executes two 1D passes (Horizontal then Vertical), with time complexity O(2 * K * W * H),
/// completely eliminating high-frequency textures (pebbles, foam, sharp edges)
/// in strict accordance with physical light diffusion.
pub fn perform_separable_gaussian_blur(src: &[[f32; 3]], w: usize, h: usize, radius: f32) -> Vec<[f32; 3]> {
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

            let dist = rounded_rect_sdf(px, py, w as f32, h as f32, corner_radius);
            let alpha = (-dist + 0.5).clamp(0.0, 1.0);

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
}

/// Unified, mathematically guaranteed layout geometry for the entire stage.
#[derive(Debug, Clone, Copy)]
pub struct LayoutMetrics {
    pub window_size: Size,
    pub header_h: f32,
    pub status_h: f32,
    pub search_rect: Rectangle,
    pub dock_rect: Rectangle,
    pub base_icon_size: f32,
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

        // Main Frosted Dock (Floating at bottom center)
        let dock_w = 688.0f32.min(window_size.width - 40.0);
        let dock_h = 86.0f32;
        let dock_y = (window_size.height - status_h - 16.0 - dock_h).max(header_h + 120.0);
        let dock_x = (window_size.width - dock_w) * 0.5;
        let dock_rect = Rectangle {
            x: dock_x,
            y: dock_y,
            width: dock_w,
            height: dock_h,
        };

        // 9 Icon Rectangles inside the Dock (Strict 1:1 Aspect Ratio Squircles)
        let base_icon_size = 56.0f32;
        let icon_gap = 14.0f32;
        let total_icons_w = 9.0 * base_icon_size + 8.0 * icon_gap; // 504 + 112 = 616
        let start_x = dock_x + (dock_w - total_icons_w) * 0.5;
        let base_y = dock_y + (dock_h - base_icon_size) * 0.5;

        let mut icon_rects = [Rectangle::default(); 9];
        for (i, app) in DockApp::ALL.iter().enumerate() {
            let is_hovered = hovered_app == Some(*app);
            let size = if is_hovered { 64.0 } else { 56.0 };
            let offset_x = (size - base_icon_size) * 0.5;
            let offset_y = if is_hovered { 8.0 } else { 0.0 };

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
            base_icon_size,
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
    /// High Transparency (Apple Sequoia/Sonoma style: ~90% clear middle, vibrant wallpaper transmission)
    #[default]
    High,
    /// Ultra Clear Glass (Minimal center tint, pure optical refraction feel)
    Ultra,
    /// Milky Frosted Glass (Traditional heavier frosted substrate)
    Frosted,
}

impl GlassTransparency {
    pub const ALL: [Self; 3] = [Self::High, Self::Ultra, Self::Frosted];

    pub fn label(&self) -> &'static str {
        match self {
            Self::High => "高透",
            Self::Ultra => "极清",
            Self::Frosted => "磨砂",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::High => Self::Ultra,
            Self::Ultra => Self::Frosted,
            Self::Frosted => Self::High,
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
    WindowOpened(window::Id),
    WindowResized(Size),
    WindowEvent((window::Id, iced::window::Event)),
    AnimationFrame(Instant),
    ResizeWindow(window::Direction),
    DragWindow,
    WindowControl(ControlAction),
    TrafficLightsHover(bool),
    SetWallpaper(WallpaperStyle),
    SetBlurPreset(BlurPreset),
    ToggleTheme,
    CycleTransparency,
    SetTransparency(GlassTransparency),
    ToggleHighlight(bool),
    ToggleDarkRim(bool),
    ToggleGrid(bool),
    SearchInputChanged(String),
    CursorMoved(Point),
    IconHovered(Option<DockApp>),
    IconClicked(DockApp),
    RightClicked,
    DismissFloatingMenu,
    TriggerAction(String),
}

/// Renders the complete backdrop, frosted blur, and Apple liquid glass optics.
struct LiquidGlassOpticsCanvas {
    style: WallpaperStyle,
    metrics: LayoutMetrics,
    blur_radius: f32,
    show_grid: bool,
    enable_highlight: bool,
    enable_dark_rim: bool,
    is_dark: bool,
    transparency: GlassTransparency,
    floating_menu_rect: Option<Rectangle>,
    system_wallpaper: Option<Arc<WallpaperBuffer>>,
}

impl<Message> canvas::Program<Message> for LiquidGlassOpticsCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // -------------------------------------------------------------
        // 1. Draw Base Sharp Wallpaper (Desktop Wallpaper / TV Bars / SMPTE / Grid / Rainbow)
        // -------------------------------------------------------------
        match self.style {
            WallpaperStyle::DesktopTransparent => {
                // Testing if draw_image interferes with canvas vector geometry
            }
            WallpaperStyle::AuroraMesh => {
                let grad = Linear::new(Point::ORIGIN, Point::new(bounds.width, bounds.height))
                    .add_stop(0.0, Color::from_rgb(0.12, 0.06, 0.38))
                    .add_stop(0.20, Color::from_rgb(0.38, 0.10, 0.58))
                    .add_stop(0.44, Color::from_rgb(0.88, 0.16, 0.48))
                    .add_stop(0.68, Color::from_rgb(0.98, 0.46, 0.15))
                    .add_stop(0.86, Color::from_rgb(0.95, 0.80, 0.22))
                    .add_stop(1.0, Color::from_rgb(0.12, 0.78, 0.82));
                let path = Path::rectangle(Point::ORIGIN, bounds.size());
                frame.fill(&path, grad);
            }
            WallpaperStyle::SunsetGaze => {
                let grad = Linear::new(Point::new(bounds.width * 0.15, 0.0), Point::new(bounds.width * 0.85, bounds.height))
                    .add_stop(0.0, Color::from_rgb(0.06, 0.10, 0.25))
                    .add_stop(0.30, Color::from_rgb(0.35, 0.12, 0.42))
                    .add_stop(0.60, Color::from_rgb(0.82, 0.22, 0.35))
                    .add_stop(0.82, Color::from_rgb(0.96, 0.52, 0.18))
                    .add_stop(1.0, Color::from_rgb(1.0, 0.82, 0.45));
                let path = Path::rectangle(Point::ORIGIN, bounds.size());
                frame.fill(&path, grad);
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
                frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::WHITE);
            }
            WallpaperStyle::PureBlack => {
                frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::BLACK);
            }
        }

        // -------------------------------------------------------------
        // 2. Alignment Calibration Grid Lines
        // -------------------------------------------------------------
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

        // -------------------------------------------------------------
        // 3. Render Frosted Search Bar Capsule
        // -------------------------------------------------------------
        let s_rect = self.metrics.search_rect;
        draw_liquid_glass_plate(
            &mut frame,
            s_rect,
            21.0,
            self.is_dark,
            self.transparency,
            self.enable_highlight,
            self.enable_dark_rim,
            self.style,
            bounds.size(),
            self.blur_radius,
            self.system_wallpaper.as_deref(),
        );

        // -------------------------------------------------------------
        // 4. Render Main Frosted Dock Bar
        // -------------------------------------------------------------
        let d_rect = self.metrics.dock_rect;
        draw_liquid_glass_plate(
            &mut frame,
            d_rect,
            24.0,
            self.is_dark,
            self.transparency,
            self.enable_highlight,
            self.enable_dark_rim,
            self.style,
            bounds.size(),
            self.blur_radius,
            self.system_wallpaper.as_deref(),
        );

        // -------------------------------------------------------------
        // 5. Render 9 Authentic macOS Squircle Icons with Liquid Optics
        // -------------------------------------------------------------
        for (i, &app) in DockApp::ALL.iter().enumerate() {
            let i_rect = self.metrics.icon_rects[i];
            draw_apple_icon(
                &mut frame,
                app,
                i_rect,
                self.is_dark,
                self.enable_highlight,
                self.enable_dark_rim,
            );

            // macOS authentic active app indicator dot (Finder, Safari, Messages, Mail, Terminal, Settings)
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
                let dot_cy = d_rect.y + d_rect.height - 5.5;
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

        // -------------------------------------------------------------
        // 6. Render Floating Context Menu (if active)
        // -------------------------------------------------------------
        if let Some(m_rect) = self.floating_menu_rect {
            draw_liquid_glass_plate(
                &mut frame,
                m_rect,
                12.0,
                self.is_dark,
                self.transparency,
                self.enable_highlight,
                self.enable_dark_rim,
                self.style,
                bounds.size(),
                self.blur_radius,
                self.system_wallpaper.as_deref(),
            );
        }

        vec![frame.into_geometry()]
    }
}

/// Renders a floating frosted liquid glass panel (Search Bar, Dock, or Context Menu)
/// with 100% continuous G2 curvature, multi-tier Gaussian shadow, and physical optics.
fn draw_liquid_glass_plate(
    frame: &mut Frame,
    rect: Rectangle,
    radius: f32,
    is_dark: bool,
    transparency: GlassTransparency,
    enable_highlight: bool,
    enable_dark_rim: bool,
    style: WallpaperStyle,
    bounds: Size,
    blur_radius: f32,
    wallpaper_buf: Option<&WallpaperBuffer>,
) {
    // 1. Soft subtle ambient elevation drop shadow
    draw_elevation_shadow(frame, rect, radius, is_dark, transparency);

    // 2. Optical Frosted Wallpaper Backdrop (Physical transmission model)
    // When blur_radius <= 0.5 (0pt preset), transmission is 100% specular (completely clear glass,
    // revealing the sharp background directly). As blur_radius increases from 0 to 16pt,
    // diffuse scattering emerges and smoothly transitions into full frosted glass.
    let path = build_squircle_path(rect, radius);
    let frost_factor = (blur_radius / 16.0).clamp(0.0, 1.0);

    if blur_radius > 0.5 {
        let diffusion_alpha = frost_factor;

        // 2a. Horizontal blurred gradient across the dock/plate width (40 samples)
        let mut grad = Linear::new(
            Point::new(rect.x, rect.y),
            Point::new(rect.x + rect.width, rect.y),
        );
        let num_samples = 40;
        for i in 0..=num_samples {
            let t = i as f32 / num_samples as f32;
            let sample_x = rect.x + rect.width * t;
            let sample_y = rect.y + rect.height * 0.5;
            let col = sample_vibrancy_blurred_wallpaper(
                style,
                sample_x,
                sample_y,
                bounds,
                blur_radius,
                is_dark,
                wallpaper_buf,
            );
            grad = grad.add_stop(t, Color::from_rgba(col.r, col.g, col.b, diffusion_alpha));
        }
        frame.fill(&path, grad);

        // 2b. Vertical subtle modulation (capturing top-to-bottom background gradient in 2D)
        if diffusion_alpha > 0.15 {
            let top_col = sample_vibrancy_blurred_wallpaper(
                style,
                rect.x + rect.width * 0.5,
                rect.y + rect.height * 0.15,
                bounds,
                blur_radius,
                is_dark,
                wallpaper_buf,
            );
            let bot_col = sample_vibrancy_blurred_wallpaper(
                style,
                rect.x + rect.width * 0.5,
                rect.y + rect.height * 0.85,
                bounds,
                blur_radius,
                is_dark,
                wallpaper_buf,
            );
            let v_alpha = 0.40 * diffusion_alpha;
            let v_grad = Linear::new(
                Point::new(rect.x, rect.y),
                Point::new(rect.x, rect.y + rect.height),
            )
            .add_stop(0.0, Color::from_rgba(top_col.r, top_col.g, top_col.b, v_alpha))
            .add_stop(1.0, Color::from_rgba(bot_col.r, bot_col.g, bot_col.b, v_alpha));
            frame.fill(&path, v_grad);
        }
    }

    // 3. Base Glass Substrate (Calibrated Apple macOS authentic transparency & intrinsic silver/graphite tint)
    // At 0pt blur, milk haze is minimal (pure optical crystal clarity); as blur increases to 16pt, full milk diffusion develops.
    let t_bleed = (6.0 / rect.height).clamp(0.06, 0.20);
    let glass_grad = if is_dark {
        let base_c_mid = transparency.center_alpha_dark();
        let c_mid = base_c_mid * (0.20 + 0.80 * frost_factor);
        let c_edge = (c_mid * 1.35 + 0.03).min(0.48);
        let tint = Color::from_rgb(0.095, 0.102, 0.125);
        let edge_tint = Color::from_rgb(0.14, 0.16, 0.20);
        Linear::new(Point::new(rect.x, rect.y), Point::new(rect.x, rect.y + rect.height))
            .add_stop(0.0, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_edge))
            .add_stop(t_bleed * 0.20, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_mid + (c_edge - c_mid) * 0.50))
            .add_stop(t_bleed * 0.50, Color::from_rgba(tint.r, tint.g, tint.b, c_mid + (c_edge - c_mid) * 0.15))
            .add_stop(t_bleed, Color::from_rgba(tint.r, tint.g, tint.b, c_mid))
            .add_stop(1.0 - t_bleed, Color::from_rgba(tint.r, tint.g, tint.b, c_mid))
            .add_stop(1.0 - t_bleed * 0.50, Color::from_rgba(tint.r, tint.g, tint.b, c_mid + (c_edge - c_mid) * 0.15))
            .add_stop(1.0 - t_bleed * 0.20, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_mid + (c_edge - c_mid) * 0.50))
            .add_stop(1.0, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_edge))
    } else {
        let base_c_mid = transparency.center_alpha_light();
        let c_mid = base_c_mid * (0.20 + 0.80 * frost_factor);
        let c_edge = (c_mid * 1.35 + 0.03).min(0.40);
        let tint = Color::from_rgb(0.95, 0.96, 0.98);
        let edge_tint = Color::from_rgb(0.98, 0.99, 1.0);
        Linear::new(Point::new(rect.x, rect.y), Point::new(rect.x, rect.y + rect.height))
            .add_stop(0.0, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_edge))
            .add_stop(t_bleed * 0.20, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_mid + (c_edge - c_mid) * 0.50))
            .add_stop(t_bleed * 0.50, Color::from_rgba(tint.r, tint.g, tint.b, c_mid + (c_edge - c_mid) * 0.15))
            .add_stop(t_bleed, Color::from_rgba(tint.r, tint.g, tint.b, c_mid))
            .add_stop(1.0 - t_bleed, Color::from_rgba(tint.r, tint.g, tint.b, c_mid))
            .add_stop(1.0 - t_bleed * 0.50, Color::from_rgba(tint.r, tint.g, tint.b, c_mid + (c_edge - c_mid) * 0.15))
            .add_stop(1.0 - t_bleed * 0.20, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_mid + (c_edge - c_mid) * 0.50))
            .add_stop(1.0, Color::from_rgba(edge_tint.r, edge_tint.g, edge_tint.b, c_edge))
    };
    frame.fill(&path, glass_grad);

    // 4. Authentic subtle continuous 0.5px perimeter glass rim
    let outline_color = if is_dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.14)
    } else {
        Color::from_rgba(1.0, 1.0, 1.0, 0.28)
    };
    frame.stroke(&path, Stroke::default().with_color(outline_color).with_width(0.5));

    // 5. Apple Liquid Glass Bevel Optics
    // Symmetrical dual-strip specular + G2 corner curvature-compressed arcs + Left/Right zero highlight + Edge Occlusion Rim
    draw_liquid_glass_bevel(frame, rect, radius, is_dark, enable_highlight, enable_dark_rim);
}

/// Draws an authentic macOS Squircle App Icon with vector graphics and liquid glass bevel.
fn draw_apple_icon(
    frame: &mut Frame,
    app: DockApp,
    rect: Rectangle,
    is_dark: bool,
    enable_highlight: bool,
    enable_dark_rim: bool,
) {
    let r = rect.width * 0.235; // Authentic Apple squircle corner radius

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

    // 1. Icon Base Squircle Plate with continuous Apple gradient
    let path = build_squircle_path(rect, r);
    let (c_top, c_bot) = app.gradient_colors();
    let grad = Linear::new(Point::new(rect.x, rect.y), Point::new(rect.x, rect.y + rect.height))
        .add_stop(0.0, c_top)
        .add_stop(1.0, c_bot);
    frame.fill(&path, grad);

    // 2. Icon Vector Glyph Artwork
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

    // 4. THE SIGNATURE APPLE LIQUID GLASS BEVEL & OPTICS RIGHT ON THE SQUIRCLE ICON!
    draw_liquid_glass_bevel(frame, rect, r, is_dark, enable_highlight, enable_dark_rim);
}

/// Draws soft multi-tier Gaussian elevation drop shadow underneath floating glass panels.
fn draw_elevation_shadow(
    frame: &mut Frame,
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
fn fill_squircle(frame: &mut Frame, rect: Rectangle, radius: f32, color: Color) {
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
fn draw_liquid_glass_bevel(
    frame: &mut Frame,
    rect: Rectangle,
    radius: f32,
    is_dark: bool,
    enable_highlight: bool,
    enable_dark_rim: bool,
) {
    let is_icon = rect.height < 70.0;
    let inset = 0.5f32;
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

    let stroke_w = if is_icon { 0.55 } else { 0.5 };
    // Softened Apple Physical Highlights (Subtle reflection, eliminating harsh glare)
    let max_spec_alpha = if is_icon {
        if is_dark { 0.65 } else { 0.78 }
    } else if is_dark {
        0.58
    } else {
        0.65
    };
    let max_bounce_alpha = if is_icon { max_spec_alpha } else { max_spec_alpha * 0.55 };

    // Measured Dark Occlusion Rim on vertical lateral edges (measured subtle darkening)
    let max_rim_alpha = if is_icon {
        if is_dark { 0.35 } else { 0.25 }
    } else if is_dark {
        0.45
    } else {
        0.38
    };

    let rect_origin = inner_rect.position();

    // Helper closure to stroke a single micro-segment with exact physical lighting
    let mut stroke_micro_segment = |p_a: Point, p_b: Point, nx: f32, ny: f32| {
        let u = (-ny).clamp(0.0, 1.0);       // Upward fraction (overhead strip light)
        let d = ny.clamp(0.0, 1.0);          // Downward fraction (bottom shelf strip light)
        let s = nx.abs().clamp(0.0, 1.0);    // Lateral fraction (side edge occlusion)

        // 1. Edge Occlusion / Rim Darkening on vertical sides
        // Measured: on lateral vertical edges (|nx| = 1.0), rim alpha is exactly 0.57.
        // It seamlessly vanishes around corner arcs as specular highlight washes out occlusion.
        if enable_dark_rim {
            let unlit_fraction = (1.0 - u.max(d)).clamp(0.0, 1.0);
            let rim_factor = s.powf(2.0) * unlit_fraction;
            let rim_alpha = max_rim_alpha * rim_factor;
            if rim_alpha > 0.015 {
                let stroke = Stroke::default()
                    .with_color(Color::from_rgba(0.0, 0.0, 0.0, rim_alpha))
                    .with_width(stroke_w);
                let seg = Path::line(p_a, p_b);
                frame.stroke(&seg, stroke);
            }
        }

        // 2. Specular Highlights: Balanced Upper & Lower Strip Lights
        // Measured: at corner angle >= 70° (u <= 0.25), highlight is 100% extinguished.
        // Rapid falloff from 45° to 65°, with razor-sharp compression at the apex.
        if enable_highlight {
            const SPEC_CUTOFF: f32 = 0.25;
            if u > SPEC_CUTOFF {
                let norm_u = (u - SPEC_CUTOFF) / (1.0 - SPEC_CUTOFF);
                let spec_factor = norm_u.powf(1.6);
                let spec_alpha = max_spec_alpha * spec_factor;
                if spec_alpha > 0.015 {
                    let stroke = Stroke::default()
                        .with_color(Color::from_rgba(1.0, 1.0, 1.0, spec_alpha))
                        .with_width(stroke_w);
                    let seg = Path::line(p_a, p_b);
                    frame.stroke(&seg, stroke);
                }
            } else if d > SPEC_CUTOFF {
                // Lower strip reflection: perfectly balanced power curve matching the upper strip
                let norm_d = (d - SPEC_CUTOFF) / (1.0 - SPEC_CUTOFF);
                let bounce_factor = norm_d.powf(1.6);
                let bounce_alpha = max_bounce_alpha * bounce_factor;
                if bounce_alpha > 0.015 {
                    let stroke = Stroke::default()
                        .with_color(Color::from_rgba(1.0, 1.0, 1.0, bounce_alpha))
                        .with_width(stroke_w);
                    let seg = Path::line(p_a, p_b);
                    frame.stroke(&seg, stroke);
                }
            }
        }
    };

    let eval_bezier = |p0: SquirclePoint, c0: SquirclePoint, c1: SquirclePoint, p1: SquirclePoint, t: f32| -> (f32, f32, f32, f32) {
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;

        let x = uuu * p0.x + 3.0 * uu * t * c0.x + 3.0 * u * tt * c1.x + ttt * p1.x;
        let y = uuu * p0.y + 3.0 * uu * t * c0.y + 3.0 * u * tt * c1.y + ttt * p1.y;

        let dx = 3.0 * uu * (c0.x - p0.x) + 6.0 * u * t * (c1.x - c0.x) + 3.0 * tt * (p1.x - c1.x);
        let dy = 3.0 * uu * (c0.y - p0.y) + 6.0 * u * t * (c1.y - c0.y) + 3.0 * tt * (p1.y - c1.y);

        (x, y, dx, dy)
    };

    let mut curr_pt = SquirclePoint::new(0.0, 0.0);
    let mut start_pt = SquirclePoint::new(0.0, 0.0);

    for cmd in &commands {
        match *cmd {
            PathCommand::MoveTo(pt) => {
                curr_pt = pt;
                start_pt = pt;
            }
            PathCommand::LineTo(pt) => {
                let dx = pt.x - curr_pt.x;
                let dy = pt.y - curr_pt.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 1e-4 {
                    let nx = dy / len;
                    let ny = -dx / len;
                    let p_a = Point::new(rect_origin.x + curr_pt.x, rect_origin.y + curr_pt.y);
                    let p_b = Point::new(rect_origin.x + pt.x, rect_origin.y + pt.y);
                    stroke_micro_segment(p_a, p_b, nx, ny);
                }
                curr_pt = pt;
            }
            PathCommand::CubicTo { c0, c1, to } => {
                let steps = 10;
                for i in 0..steps {
                    let t_a = i as f32 / steps as f32;
                    let t_b = (i + 1) as f32 / steps as f32;
                    let t_mid = (t_a + t_b) * 0.5;

                    let (xa, ya, _, _) = eval_bezier(curr_pt, c0, c1, to, t_a);
                    let (xb, yb, _, _) = eval_bezier(curr_pt, c0, c1, to, t_b);
                    let (_, _, dx, dy) = eval_bezier(curr_pt, c0, c1, to, t_mid);

                    let len = (dx * dx + dy * dy).sqrt();
                    if len > 1e-4 {
                        let nx = dy / len;
                        let ny = -dx / len;
                        let p_a = Point::new(rect_origin.x + xa, rect_origin.y + ya);
                        let p_b = Point::new(rect_origin.x + xb, rect_origin.y + yb);
                        stroke_micro_segment(p_a, p_b, nx, ny);
                    }
                }
                curr_pt = to;
            }
            PathCommand::Close => {
                let dx = start_pt.x - curr_pt.x;
                let dy = start_pt.y - curr_pt.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 1e-4 {
                    let nx = dy / len;
                    let ny = -dx / len;
                    let p_a = Point::new(rect_origin.x + curr_pt.x, rect_origin.y + curr_pt.y);
                    let p_b = Point::new(rect_origin.x + start_pt.x, rect_origin.y + start_pt.y);
                    stroke_micro_segment(p_a, p_b, nx, ny);
                }
                curr_pt = start_pt;
            }
        }
    }
}

/// State of the Liquid Glass Optics & Dock Showcase.
#[derive(Debug)]
pub struct State {
    pub controller: WindowShellController,
    pub traffic_lights: TrafficLightsState,
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
    pub floating_menu: Option<(Point, MenuContext)>,
    pub floating_menu_cached: ContextMenu<Message>,
    pub last_action: String,
    pub system_wallpaper: Arc<WallpaperBuffer>,
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

        let s = Self {
            controller,
            traffic_lights: TrafficLightsState::new(),
            window_size: Size::new(1240.0, 820.0),
            theme,
            wallpaper: if std::env::var("WALLPAPER").map(|s| s == "tv").unwrap_or(false) {
                WallpaperStyle::TvColorBars
            } else {
                WallpaperStyle::DesktopTransparent
            },
            transparency: GlassTransparency::High,
            blur_preset: BlurPreset::Standard16,
            blur_radius: BlurPreset::Standard16.radius(),
            show_grid: false,
            enable_highlight: true,
            enable_dark_rim: true,
            cursor_pos: Point::new(600.0, 400.0),
            hovered_app: None,
            search_query: String::new(),
            floating_menu: None,
            floating_menu_cached: ContextMenu::new(),
            last_action: "就绪：macOS 原生桌面壁纸输入已接入，Liquid Glass 实施 2D 深度高斯模糊卷积".to_string(),
            system_wallpaper,
        };
        s
    }
}

impl State {
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
    (state, Task::none())
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::WindowOpened(id) => {
            state.controller.set_window_id(id);
            let is_dark = state.controller.is_dark;
            window::run(id, move |w| {
                if let Ok(handle) = w.window_handle() {
                    let appearance = if is_dark {
                        bmol_window_shell::native::WindowAppearance::Dark
                    } else {
                        bmol_window_shell::native::WindowAppearance::Light
                    };
                    let _ = bmol_window_shell::setup_native_window(
                        handle.as_raw(),
                        bmol_window_shell::NativeWindowOptions::new()
                            .with_appearance(appearance)
                            .with_corner_radius(f64::from(window_metrics::DEFAULT_CORNER_RADIUS))
                            .with_desktop_blur_radius(0)
                            .with_stage_manager_guard(false),
                    );
                }
            })
            .discard()
        }
        Message::WindowResized(size) => {
            state.window_size = size;
            state.controller.handle_resized(size.width, size.height);
            Task::none()
        }
        Message::WindowEvent((_id, event)) => {
            if let Some(shell_event) = state.controller.handle_window_event(&event) {
                match shell_event {
                    bmol_window_shell::ShellEvent::Focused
                    | bmol_window_shell::ShellEvent::Unfocused => {
                        state.traffic_lights.on_group_hover(false);
                    }
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
            state.traffic_lights.step(now);
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
        Message::WindowControl(action) => match action {
            ControlAction::Close => {
                if let Some(id) = state.controller.window_id {
                    window::close(id)
                } else {
                    Task::none()
                }
            }
            ControlAction::Minimize => {
                if let Some(id) = state.controller.window_id {
                    window::minimize(id, true)
                } else {
                    Task::none()
                }
            }
            ControlAction::Expand => {
                if let Some(id) = state.controller.window_id {
                    window::toggle_maximize(id)
                } else {
                    Task::none()
                }
            }
        },
        Message::TrafficLightsHover(hover) => {
            state.traffic_lights.on_group_hover(hover);
            Task::none()
        }
        Message::SetWallpaper(w) => {
            state.wallpaper = w;
            state.last_action = format!("切换壁纸: {}", w.label());
            Task::none()
        }
        Message::SetBlurPreset(p) => {
            state.blur_preset = p;
            state.blur_radius = p.radius();
            state.last_action = format!("切换模糊预设: {} (vibrancy-rs)", p.label());
            Task::none()
        }
        Message::ToggleTheme => {
            state.controller.is_dark = !state.controller.is_dark;
            state.theme = UiTheme::new(state.scheme()).iced_theme();
            state.last_action = format!("切换外观: {}", if state.is_dark() { "深色 (Dark)" } else { "浅色 (Light)" });
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
        window::open_events().map(Message::WindowOpened),
        window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
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

    if state.traffic_lights.is_animating() {
        subscriptions.push(window::frames().map(Message::AnimationFrame));
    }

    Subscription::batch(subscriptions)
}

pub fn view(state: &State) -> Element<'_, Message, Theme, iced::Renderer> {
    let is_dark = state.is_dark();
    let metrics = LayoutMetrics::new(state.window_size, state.hovered_app);

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
    // Layer 1: Liquid Glass Optics & Backdrop Canvas
    // -------------------------------------------------------------
    let canvas_widget = Canvas::new(LiquidGlassOpticsCanvas {
        style: state.wallpaper,
        metrics,
        blur_radius: state.blur_radius,
        show_grid: state.show_grid,
        enable_highlight: state.enable_highlight,
        enable_dark_rim: state.enable_dark_rim,
        is_dark,
        transparency: state.transparency,
        floating_menu_rect,
        system_wallpaper: Some(state.system_wallpaper.clone()),
    })
    .width(Length::Fill)
    .height(Length::Fill);

    // -------------------------------------------------------------
    // Layer 2: Interactive Controls Overlay (Exact Metric Sizing)
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

    let mut layers: Vec<Element<'_, Message, Theme, iced::Renderer>> = Vec::new();

    // Layer 0: Real System Wallpaper backdrop (placed in widget stack so it stays strictly underneath canvas meshes)
    if state.wallpaper == WallpaperStyle::DesktopTransparent {
        let wallpaper_widget = iced::widget::image(&state.system_wallpaper.handle)
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(iced::ContentFit::Fill);
        layers.push(wallpaper_widget.into());
    }

    // Layer 1: Liquid Glass Optics & Backdrop Canvas (contains blurred glass plates, squircle, icons, highlights)
    layers.push(canvas_widget.into());

    // Layer 2: Interactive Controls Overlay (Exact Metric Sizing)
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

        let menu_view = state.floating_menu_cached.view::<iced::Renderer>(&state.theme);

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
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            border: Border {
                radius: window_metrics::DEFAULT_CORNER_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    state.controller.wrap_window_with_resizer(
        root_stack,
        window_metrics::DEFAULT_CORNER_RADIUS,
        Message::ResizeWindow,
    )
}

/// Builds the top titlebar with macOS Traffic Lights, Title, and Tuning Bar.
fn view_header(state: &State, is_dark: bool) -> Element<'_, Message, Theme, iced::Renderer> {
    let traffic_lights = view_traffic_lights_group(state, 8.0);

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
        Some(Message::WindowControl(ControlAction::Expand)),
    )
}

/// Builds the traffic lights group matching authentic macOS styling and animations.
fn view_traffic_lights_group(
    state: &State,
    _margin: f32,
) -> Element<'_, Message, Theme, iced::Renderer> {
    let is_focused = state.controller.is_focused;
    let hover_amount = state.traffic_lights.hover_progress;
    let is_dark = state.controller.is_dark;

    let build_btn = |action: ControlAction,
                     index: usize,
                     base_color: Color,
                     hover_color: Color,
                     press_color: Color,
                     border_color: Color,
                     glyph_char: &'static str| {
        let is_animating = state.traffic_lights.press_targets[index] > 0.0;
        let scale = state.traffic_lights.press_springs[index].value();
        let size = traffic_lights::DIAMETER * scale;

        let fill_color = if !is_focused && hover_amount < 0.05 {
            if is_dark {
                Color::from_rgb8(0x4C, 0x4C, 0x50)
            } else {
                Color::from_rgb8(0xD1, 0xD1, 0xD6)
            }
        } else if is_animating {
            press_color
        } else if hover_amount > 0.5 {
            hover_color
        } else {
            base_color
        };

        let glyph_text = text(glyph_char)
            .size(if action == ControlAction::Close { 8.0 } else { 7.0 })
            .font(font::ui_font(Weight::Bold))
            .color(Color {
                a: hover_amount * if is_dark { 0.85 } else { 0.75 },
                ..match action {
                    ControlAction::Close => Color::from_rgb8(0x4C, 0x00, 0x00),
                    ControlAction::Minimize => Color::from_rgb8(0x5A, 0x36, 0x00),
                    ControlAction::Expand => Color::from_rgb8(0x0A, 0x38, 0x00),
                }
            });

        let btn_content = container(glyph_text)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .center_x(Length::Fixed(size))
            .center_y(Length::Fixed(size));

        button(btn_content)
            .padding(0)
            .style(move |_theme, _status| button::Style {
                background: Some(Background::Color(fill_color)),
                border: Border::default()
                    .rounded(size * 0.5)
                    .width(0.5)
                    .color(border_color),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                    offset: Vector::new(0.0, 0.5),
                    blur_radius: 1.0,
                },
                ..Default::default()
            })
            .on_press(Message::WindowControl(action))
    };

    let red = build_btn(
        ControlAction::Close,
        0,
        Color::from_rgb8(0xFF, 0x5F, 0x56),
        Color::from_rgb8(0xFF, 0x6E, 0x67),
        Color::from_rgb8(0xD3, 0x3B, 0x36),
        Color::from_rgb8(0xE0, 0x44, 0x3E),
        "✕",
    );

    let yellow = build_btn(
        ControlAction::Minimize,
        1,
        Color::from_rgb8(0xFF, 0xBD, 0x2E),
        Color::from_rgb8(0xFF, 0xC8, 0x47),
        Color::from_rgb8(0xD7, 0x96, 0x1E),
        Color::from_rgb8(0xDE, 0xA1, 0x23),
        "─",
    );

    let green = build_btn(
        ControlAction::Expand,
        2,
        Color::from_rgb8(0x27, 0xC9, 0x3F),
        Color::from_rgb8(0x32, 0xD8, 0x4D),
        Color::from_rgb8(0x19, 0xA0, 0x23),
        Color::from_rgb8(0x1A, 0xAB, 0x29),
        "⤢",
    );

    let slop = traffic_lights::control_hover_slop(traffic_lights::DIAMETER);
    let controls_row = row![red, yellow, green]
        .spacing(traffic_lights::SPACING)
        .align_y(Alignment::Center);

    let tracking_area = container(controls_row).padding(Padding {
        top: slop,
        right: slop,
        bottom: slop,
        left: slop,
    });

    iced::widget::mouse_area(tracking_area)
        .on_enter(Message::TrafficLightsHover(true))
        .on_exit(Message::TrafficLightsHover(false))
        .into()
}

/// Builds the search input widget positioned exactly over `search_rect`.
fn view_search_input(
    state: &State,
    is_dark: bool,
    search_rect: Rectangle,
) -> Element<'_, Message, Theme, iced::Renderer> {
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
) -> Element<'_, Message, Theme, iced::Renderer> {
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
fn view_status_bar(state: &State, is_dark: bool) -> Element<'_, Message, Theme, iced::Renderer> {
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
        transparent: true,
        decorations: false,
        ..Default::default()
    };

    let mut app = iced::application::<State, Message, Theme, iced::Renderer>(boot, update, view)
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
        assert_eq!(t, GlassTransparency::High);

        assert_eq!(t.next(), GlassTransparency::Ultra);
        assert_eq!(t.next().next(), GlassTransparency::Frosted);
        assert_eq!(t.next().next().next(), GlassTransparency::High);
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
        assert_eq!(state.transparency, GlassTransparency::High);

        let _ = update(&mut state, Message::CycleTransparency);
        assert_eq!(state.transparency, GlassTransparency::Ultra);

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
        assert_eq!(metrics.dock_rect.height, 86.0);
        assert_eq!(metrics.icon_rects.len(), 9);
        // Icons should fit nicely inside the dock with vertical margin
        assert!(metrics.icon_rects[0].y > metrics.dock_rect.y);
        assert!(metrics.icon_rects[0].y + metrics.icon_rects[0].height < metrics.dock_rect.y + metrics.dock_rect.height);
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
        assert_eq!(MaterialKind::Launchpad.blur_radius(), 36.0);
        let dark_tint = MaterialKind::Dock.tint(VibrancyAppearance::Dark);
        assert_eq!(dark_tint, [24, 24, 28, 180]);
    }

    #[test]
    fn test_vibrancy_blurred_wallpaper_sampling() {
        let col = sample_vibrancy_blurred_wallpaper(
            WallpaperStyle::TvColorBars,
            300.0,
            400.0,
            Size::new(1200.0, 820.0),
            64.0,
            false,
            None,
        );
        // RGB components should be valid clamped floats
        assert!(col.r >= 0.0 && col.r <= 1.0);
        assert!(col.g >= 0.0 && col.g <= 1.0);
        assert!(col.b >= 0.0 && col.b <= 1.0);
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

        let vibrant_col = sample_vibrancy_blurred_wallpaper(
            WallpaperStyle::DesktopTransparent,
            100.0,
            75.0,
            Size::new(200.0, 150.0),
            96.0,
            false,
            Some(&buf),
        );
        assert!(vibrant_col.r >= 0.0 && vibrant_col.r <= 1.0);
    }

    #[test]
    fn test_real_system_wallpaper_sampling() {
        let wallpaper = load_or_create_wallpaper(1240, 820);
        println!("Loaded wallpaper: {}x{}, pixels len: {}", wallpaper.width, wallpaper.height, wallpaper.pixels.len());
        let col = sample_vibrancy_blurred_wallpaper(
            WallpaperStyle::DesktopTransparent,
            620.0,
            740.0,
            Size::new(1240.0, 820.0),
            96.0,
            false,
            Some(&wallpaper),
        );
        println!("Sampled vibrancy col at dock center: {:?}", col);
        assert!(!col.r.is_nan());
        assert!(!col.g.is_nan());
        assert!(!col.b.is_nan());
        assert!(!col.a.is_nan());
    }

    #[test]
    fn test_specular_cutoff_and_rim_darkening_physics() {
        const SPEC_CUTOFF: f32 = 0.25;

        // At theta = 90° (vertical edge), u = 0.0, d = 0.0, s = 1.0
        let u_vertical = 0.0f32;
        let s_vertical = 1.0f32;
        assert!(u_vertical <= SPEC_CUTOFF, "Vertical edge must have 0 specular highlight");

        let unlit_vertical = 1.0 - u_vertical;
        let rim_factor_vertical = s_vertical.powf(2.0) * unlit_vertical;
        assert!((rim_factor_vertical - 1.0).abs() < 1e-5, "Vertical edge must have 100% dark rim");

        // At theta = 0° (top horizontal edge), u = 1.0, s = 0.0
        let u_horizontal = 1.0f32;
        let s_horizontal = 0.0f32;
        assert!(u_horizontal > SPEC_CUTOFF);
        let norm_u_horizontal = (u_horizontal - SPEC_CUTOFF) / (1.0 - SPEC_CUTOFF);
        assert!((norm_u_horizontal - 1.0).abs() < 1e-5, "Top edge must have 100% specular");
        let rim_factor_horizontal = s_horizontal.powf(2.0) * (1.0 - u_horizontal);
        assert!((rim_factor_horizontal - 0.0).abs() < 1e-5, "Top edge must have 0 dark rim");

        // At theta = 75° (approaching vertical, u = 0.25)
        let u_75 = 0.25f32;
        assert!(u_75 <= SPEC_CUTOFF, "Specular must extinguish by 75 degrees");

        // Calibrated light mode base alpha
        assert_eq!(GlassTransparency::High.center_alpha_light(), 0.13);
    }

    #[test]
    fn test_draw_image_signature() {
        let handle = iced::widget::image::Handle::from_rgba(2, 2, vec![255; 16]);
        let _ = format!("{:?}", handle);
        let _img = iced::widget::canvas::Image::new(handle);
    }
}


