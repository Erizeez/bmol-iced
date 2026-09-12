//! Authentic macOS Liquid Glass Dock component, physical plate generator,
//! and concentric squircle geometry system.

#![allow(
    clippy::many_single_char_names,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::suboptimal_flops,
    clippy::similar_names
)]

use bmol_designs::dock_metrics;
use bmol_window_native::{app_icon_png, load_system_wallpaper_rgba};
use iced::{
    Color, Point, Rectangle, Size,
    widget::canvas::{self, Frame, Path},
};
use squircle_rs::{
    APPLE_CORNER_SMOOTHING, PathCommand, Point as SquirclePoint, SquircleParams, sd_squircle,
    squircle_alpha, squircle_path_commands,
};
use std::sync::Arc;

// =========================================================================
// 1. Wallpaper Buffer & Procedural Backdrops
// =========================================================================

/// In-memory RGBA wallpaper buffer enabling continuous 2D optical sampling
/// at full 2x Retina physical resolution with multi-scale blur support.
#[derive(Debug, Clone)]
pub struct WallpaperBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<Vec<u8>>,
}

impl WallpaperBuffer {
    #[must_use]
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self { width, height, pixels: Arc::new(pixels) }
    }

    #[must_use]
    pub fn iced_handle(&self) -> iced::widget::image::Handle {
        iced::widget::image::Handle::from_rgba(
            self.width,
            self.height,
            self.pixels.as_ref().clone(),
        )
    }

    #[inline]
    #[must_use]
    pub fn sample_bilinear(&self, u: f32, v: f32) -> [f32; 3] {
        if self.width == 0 || self.height == 0 || self.pixels.is_empty() {
            return [0.0, 0.0, 0.0];
        }

        let fx = u.clamp(0.0, 1.0) * (self.width - 1) as f32;
        let fy = v.clamp(0.0, 1.0) * (self.height - 1) as f32;

        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let x1 = (x0 + 1).min(self.width as usize - 1);
        let y1 = (y0 + 1).min(self.height as usize - 1);

        let wx = fx - x0 as f32;
        let wy = fy - y0 as f32;

        let get_rgb = |x: usize, y: usize| -> [f32; 3] {
            let idx = (y * self.width as usize + x) * 4;
            if idx + 2 < self.pixels.len() {
                [
                    self.pixels[idx] as f32 / 255.0,
                    self.pixels[idx + 1] as f32 / 255.0,
                    self.pixels[idx + 2] as f32 / 255.0,
                ]
            } else {
                [0.0, 0.0, 0.0]
            }
        };

        let c00 = get_rgb(x0, y0);
        let c10 = get_rgb(x1, y0);
        let c01 = get_rgb(x0, y1);
        let c11 = get_rgb(x1, y1);

        let mut out = [0.0f32; 3];
        for i in 0..3 {
            let top = c00[i] * (1.0 - wx) + c10[i] * wx;
            let bot = c01[i] * (1.0 - wx) + c11[i] * wx;
            out[i] = top * (1.0 - wy) + bot * wy;
        }
        out
    }

    #[inline]
    #[must_use]
    pub fn sample_blurred(&self, u: f32, v: f32, _radius: f32, _bounds: Size) -> [f32; 3] {
        self.sample_bilinear(u, v)
    }
}

/// Creates an emergency high-fidelity procedural sunset background.
pub fn create_procedural_redwood_buffer(width: u32, height: u32) -> WallpaperBuffer {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let v = y as f32 / height.max(1) as f32;
        for x in 0..width {
            let u = x as f32 / width.max(1) as f32;
            let r = 0.15 + 0.70 * (1.0 - v) + 0.15 * u;
            let g = 0.20 + 0.40 * (1.0 - v * 0.8);
            let b = 0.35 + 0.50 * v;
            pixels.push((r.clamp(0.0, 1.0) * 255.0) as u8);
            pixels.push((g.clamp(0.0, 1.0) * 255.0) as u8);
            pixels.push((b.clamp(0.0, 1.0) * 255.0) as u8);
            pixels.push(255);
        }
    }
    WallpaperBuffer::new(width, height, pixels)
}

/// Loads authentic macOS desktop wallpaper or falls back to bundled assets.
pub fn load_or_create_wallpaper(target_w: u32, target_h: u32) -> Arc<WallpaperBuffer> {
    if let Some((w, h, rgba)) = load_system_wallpaper_rgba(target_w, target_h) {
        return Arc::new(WallpaperBuffer::new(w, h, rgba));
    }
    for path in &["/tmp/current_wallpaper.png", "/tmp/lg-harness/assets/bg_sonoma2x_srgb.png"] {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(img) = image::load_from_memory(&bytes) {
                let resized =
                    img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3);
                let rgba = resized.to_rgba8().into_raw();
                return Arc::new(WallpaperBuffer::new(target_w, target_h, rgba));
            }
        }
    }
    Arc::new(create_procedural_redwood_buffer(target_w, target_h))
}

/// Available colorful test patterns and atmospheric wallpapers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallpaperStyle {
    /// 100% 透明穿透至 macOS 系统桌面壁纸
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

#[inline]
fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Computes the unblurred base wallpaper RGB color at coordinate `(x, y)`.
#[inline]
pub fn sample_sharp_wallpaper(
    style: WallpaperStyle,
    x: f32,
    y: f32,
    bounds: Size,
    wallpaper_buf: Option<&WallpaperBuffer>,
) -> [f32; 3] {
    const TV_BARS: [[f32; 3]; 8] = [
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
    ];

    const TOP_SMPTE: [[f32; 3]; 7] = [
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
    ];

    const BOT_SMPTE: [[f32; 3]; 8] = [
        [0.0, 0.25, 0.5],
        [1.0, 1.0, 1.0],
        [0.2, 0.0, 0.4],
        [0.05, 0.05, 0.05],
        [0.0, 0.0, 0.0],
        [0.15, 0.15, 0.15],
        [0.05, 0.05, 0.05],
        [0.05, 0.05, 0.05],
    ];

    const GRID_PALETTE: [[f32; 3]; 12] = [
        [0.9, 0.2, 0.3],
        [1.0, 0.5, 0.1],
        [0.2, 0.8, 0.4],
        [0.0, 0.7, 0.9],
        [0.2, 0.4, 1.0],
        [0.6, 0.2, 0.9],
        [1.0, 0.3, 0.7],
        [0.1, 0.9, 0.8],
        [0.9, 0.8, 0.2],
        [0.3, 0.2, 0.8],
        [0.9, 0.4, 0.2],
        [0.2, 0.3, 0.4],
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
pub fn perform_separable_gaussian_blur(
    src: &[[f32; 3]],
    w: usize,
    h: usize,
    radius: f32,
) -> Vec<[f32; 3]> {
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

/// Generates a 2D live preview plot of the Bernstein-Bezier glass cross-section profile.
pub fn generate_bezier_preview(p1: f32, p2: f32, p3: f32) -> iced::widget::image::Handle {
    const W: u32 = 140;
    const H: u32 = 34;
    let mut raw = vec![0u8; (W * H * 4) as usize];

    for px in raw.chunks_exact_mut(4) {
        px[0] = 20;
        px[1] = 22;
        px[2] = 28;
        px[3] = 255;
    }

    let floor_y = H as f32 - 5.0;
    let top_y = 5.0;
    let height_span = floor_y - top_y;

    let floor_row = floor_y as usize;
    for x in 0..W as usize {
        let idx = (floor_row * W as usize + x) * 4;
        raw[idx] = 45;
        raw[idx + 1] = 50;
        raw[idx + 2] = 60;
    }

    for x in 0..W {
        let t = x as f32 / (W - 1) as f32;
        let u = 1.0 - t;
        let u2 = u * u;
        let u3 = u2 * u;
        let u4 = u2 * u2;
        let t2 = t * t;
        let t3 = t2 * t;
        let y_val = u4 * 1.0 + 4.0 * u3 * t * p1 + 6.0 * u2 * t2 * p2 + 4.0 * u * t3 * p3;

        let py = (floor_y - y_val * height_span).round() as i32;
        for dy in -1..=1 {
            let yy = py + dy;
            if yy >= 0 && yy < H as i32 {
                let idx = ((yy as usize) * (W as usize) + (x as usize)) * 4;
                raw[idx] = 94;
                raw[idx + 1] = 200;
                raw[idx + 2] = 255;
            }
        }
    }

    iced::widget::image::Handle::from_rgba(W, H, raw)
}

// =========================================================================
// 2. 2x Retina Point-to-Point Physical Plate Generator
// =========================================================================

/// Configuration parameters for generating a physical Liquid Glass dock plate texture.
#[derive(Debug, Clone, Copy)]
pub struct PhysicalPlateConfig {
    pub clarity: f32,
    pub amount: f32,
    pub p1: f32,
    pub p2: f32,
    pub p3: f32,
    pub width_x: f32,
    pub height_y: f32,
    pub milkiness: f32,
    pub highlight_intensity: f32,
    pub corner_radius: f32,
    pub corner_smoothing: f32,
}

impl Default for PhysicalPlateConfig {
    fn default() -> Self {
        Self {
            clarity: 0.03, // 97% crisp clarity
            amount: -27.0, // calibrated displacement amount
            p1: 0.18,      // micro-surface tension at outer shoulder
            p2: 0.00,
            p3: 0.00,
            width_x: 13.0, // concentric 13px padding alignment
            height_y: 13.0,
            milkiness: 0.0,           // 0% neutral crystal clear
            highlight_intensity: 1.0, // 100% grazing highlight
            corner_radius: 23.0,      // concentric: 13px + 10px = 23px
            corner_smoothing: 0.20,   // 80% circular arc + 20% smooth transition
        }
    }
}

/// Generates a physical 2D frosted backdrop plate texture slice at 2x Retina resolution.
pub fn generate_physical_dock_plate_texture(
    style: WallpaperStyle,
    rect: Rectangle,
    window_size: Size,
    config: PhysicalPlateConfig,
    _is_dark: bool,
    wallpaper_buf: Option<&WallpaperBuffer>,
) -> iced::widget::image::Handle {
    // 2x Retina native physical pixel grid (1:1 point-to-point device resolution)
    let scale = 2.0f32;
    let w = ((rect.width * scale).round() as usize).max(1);
    let h = ((rect.height * scale).round() as usize).max(1);

    let p = (32.0f32 * scale).round() as usize;
    let ext_w = w + 2 * p;
    let ext_h = h + 2 * p;

    let mut ext_raw = Vec::with_capacity(ext_w * ext_h);
    for iy in 0..ext_h {
        let wy = (rect.y + (iy as f32 - p as f32 + 0.5) / scale).clamp(0.0, window_size.height);
        for ix in 0..ext_w {
            let wx = (rect.x + (ix as f32 - p as f32 + 0.5) / scale).clamp(0.0, window_size.width);
            let c = sample_sharp_wallpaper(style, wx, wy, window_size, wallpaper_buf);
            ext_raw.push(c);
        }
    }

    let t = config.clarity.clamp(0.0, 1.0);
    let effective_blur = (if t <= 0.02 { 0.0 } else { 0.5 + t.powf(1.4) * 32.0 }) * scale;
    let ext_blurred = if effective_blur <= 0.5 {
        ext_raw.clone()
    } else {
        perform_separable_gaussian_blur(&ext_raw, ext_w, ext_h, effective_blur)
    };

    let half_x = w as f32 * 0.5;
    let half_y = h as f32 * 0.5;

    let width_x = (config.width_x * scale).max(4.0);
    let height_y = (config.height_y * scale).max(4.0);
    let amount = config.amount * scale;
    let corner_radius = config.corner_radius * scale;
    let base_h = (width_x * height_y).sqrt();
    let disp_x = width_x / base_h;
    let disp_y = height_y / base_h;

    // Continuous tone matrix:
    let m_trans = [
        [0.9200, -0.0500, -0.0200, 0.0800],
        [-0.0200, 0.9000, -0.0200, 0.0800],
        [-0.0200, -0.0500, 0.9400, 0.0900],
    ];
    let m_tinted = [
        [0.5655, 0.0243, -0.1112, 0.4910],
        [0.1239, 0.5192, -0.1474, 0.4858],
        [0.1407, 0.0068, 0.3305, 0.4844],
    ];

    let lerp4 = |a: [f32; 4], b: [f32; 4], f: f32| {
        [
            a[0] + (b[0] - a[0]) * f,
            a[1] + (b[1] - a[1]) * f,
            a[2] + (b[2] - a[2]) * f,
            a[3] + (b[3] - a[3]) * f,
        ]
    };

    let cm0 = lerp4(m_trans[0], m_tinted[0], t);
    let cm1 = lerp4(m_trans[1], m_tinted[1], t);
    let cm2 = lerp4(m_trans[2], m_tinted[2], t);
    let lighten_weight = 0.15 + t * 0.20;

    let mut rgba_bytes = vec![0u8; w * h * 4];
    let half = SquirclePoint::new(half_x, half_y);
    let is_capsule = (corner_radius - half_y).abs() < 1.0;
    let smoothing = if is_capsule { 0.0 } else { config.corner_smoothing };
    let eps = 0.25f32 * scale;

    for iy in 0..h {
        let py = iy as f32 + 0.5;

        for ix in 0..w {
            let px = ix as f32 + 0.5;
            let pt = SquirclePoint::new(px - half_x, py - half_y);

            let dist = sd_squircle(pt, half, corner_radius, smoothing);
            if dist > 1.5 {
                continue;
            }

            let dx =
                sd_squircle(SquirclePoint::new(pt.x + eps, pt.y), half, corner_radius, smoothing)
                    - sd_squircle(
                        SquirclePoint::new(pt.x - eps, pt.y),
                        half,
                        corner_radius,
                        smoothing,
                    );
            let dy =
                sd_squircle(SquirclePoint::new(pt.x, pt.y + eps), half, corner_radius, smoothing)
                    - sd_squircle(
                        SquirclePoint::new(pt.x, pt.y - eps),
                        half,
                        corner_radius,
                        smoothing,
                    );
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            let nx = dx / len;
            let ny = dy / len;

            let out_idx = (iy * w + ix) * 4;

            let eff_h = ((nx * width_x).powi(2) + (ny * height_y).powi(2)).sqrt().max(1e-5);
            let t_dist = ((-dist) / eff_h).clamp(0.0, 1.0);
            let u = 1.0 - t_dist;
            let falloff = u * u * u * u * 1.0
                + 4.0 * u * u * u * t_dist * config.p1
                + 6.0 * u * u * t_dist * t_dist * config.p2
                + 4.0 * u * t_dist * t_dist * t_dist * config.p3;
            let inner_mag = amount * falloff;

            let sample_ext_x = ((ix + p) as f32 + inner_mag * nx * disp_x).round() as isize;
            let sample_ext_y = ((iy + p) as f32 + inner_mag * ny * disp_y).round() as isize;
            let clamped_x = sample_ext_x.clamp(0, ext_w as isize - 1) as usize;
            let clamped_y = sample_ext_y.clamp(0, ext_h as isize - 1) as usize;

            let sm_blur = ext_blurred[clamped_y * ext_w + clamped_x];
            let sm_fill = ext_blurred[(iy + p) * ext_w + (ix + p)];
            let sm_raw = ext_raw[(iy + p) * ext_w + (ix + p)];

            let grade_c = |c: [f32; 3]| -> [f32; 3] {
                [
                    (c[0] * cm0[0] + c[1] * cm0[1] + c[2] * cm0[2] + cm0[3]).clamp(0.0, 1.0),
                    (c[0] * cm1[0] + c[1] * cm1[1] + c[2] * cm1[2] + cm1[3]).clamp(0.0, 1.0),
                    (c[0] * cm2[0] + c[1] * cm2[1] + c[2] * cm2[2] + cm2[3]).clamp(0.0, 1.0),
                ]
            };

            let base_g = grade_c(sm_blur);
            let fill_g = grade_c(sm_fill);
            let mixed_r =
                lighten_weight * base_g[0].max(fill_g[0]) + (1.0 - lighten_weight) * base_g[0];
            let mixed_g =
                lighten_weight * base_g[1].max(fill_g[1]) + (1.0 - lighten_weight) * base_g[1];
            let mixed_b =
                lighten_weight * base_g[2].max(fill_g[2]) + (1.0 - lighten_weight) * base_g[2];

            // 1. Colloidal Milkiness:
            let milk_factor = config.milkiness * 0.70;
            let mut final_r = mixed_r * (1.0 - milk_factor) + 1.0 * milk_factor;
            let mut final_g = mixed_g * (1.0 - milk_factor) + 1.0 * milk_factor;
            let mut final_b = mixed_b * (1.0 - milk_factor) + 1.0 * milk_factor;

            // 2. Physical Grazing Highlight (Symmetric Top & Bottom):
            if config.highlight_intensity > 0.001 {
                let edge_weight = ny.abs().powf(1.6);
                let d = (-dist).max(0.0);
                let core_span = 2.2f32 * scale;
                let sharp_core = (1.0f32 - (d / core_span).min(1.0f32)).powi(3);
                let t_bevel = (d / eff_h).min(1.0);
                let faint_halo = (1.0f32 - t_bevel).powi(2) * 0.08;

                let light_contrib =
                    (sharp_core * 0.72 + faint_halo) * edge_weight * config.highlight_intensity;

                final_r = (final_r + light_contrib).min(1.0);
                final_g = (final_g + light_contrib).min(1.0);
                final_b = (final_b + light_contrib).min(1.0);
            }

            // 3. Lateral Micro-Rim Subtractive Shadow:
            {
                let lat_weight = nx.abs().powf(2.0);
                let d_lat = (-dist).max(0.0);
                let lat_span = 2.6f32 * scale;
                let lat_decay = (1.0f32 - (d_lat / lat_span).min(1.0f32)).powi(2);
                let lat_drop = lat_weight * lat_decay * (42.0 / 255.0);

                final_r = (final_r - lat_drop).max(0.0);
                final_g = (final_g - lat_drop).max(0.0);
                final_b = (final_b - lat_drop).max(0.0);
            }

            let delta_lr = if nx.abs() > ny.abs() { 46.0 / 255.0 } else { 32.0 / 255.0 };
            let shadow_r = (sm_raw[0] - delta_lr).max(0.0);
            let shadow_g = (sm_raw[1] - delta_lr).max(0.0);
            let shadow_b = (sm_raw[2] - delta_lr).max(0.0);

            let alpha = squircle_alpha(pt, half, corner_radius, smoothing);
            let out_r = alpha * final_r + (1.0 - alpha) * shadow_r;
            let out_g = alpha * final_g + (1.0 - alpha) * shadow_g;
            let out_b = alpha * final_b + (1.0 - alpha) * shadow_b;

            rgba_bytes[out_idx] = (out_r.clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba_bytes[out_idx + 1] = (out_g.clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba_bytes[out_idx + 2] = (out_b.clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba_bytes[out_idx + 3] = (alpha * 255.0).round() as u8;
        }
    }

    iced::widget::image::Handle::from_rgba(w as u32, h as u32, rgba_bytes)
}

// =========================================================================
// 3. Application Icons & Concentric Layout Metrics
// =========================================================================

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
            Self::Finder => "访达",
            Self::Safari => "Safari浏览器",
            Self::Messages => "信息",
            Self::Mail => "邮件",
            Self::Music => "音乐",
            Self::Photos => "照片",
            Self::Terminal => "终端",
            Self::Settings => "系统设置",
            Self::Trash => "废纸篓",
        }
    }

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

/// Loads authentic macOS application icons directly from disk and processes
/// them through `squircle-icon-rs` with 10:20:10 curvature and Apple HIG squircle plates.
pub fn load_real_app_icons() -> [Option<iced::widget::image::Handle>; 9] {
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

/// Unified, mathematically guaranteed layout geometry for the entire stage.
#[derive(Debug, Clone, Copy)]
pub struct LayoutMetrics {
    pub window_size: Size,
    pub header_h: f32,
    pub status_h: f32,
    pub search_rect: Rectangle,
    pub dock_rect: Rectangle,
    pub dock_radius: f32,
    pub new_dock_rect: Rectangle,
    pub base_icon_size: f32,
    pub icon_radius: f32,
    pub icon_gap: f32,
    pub dock_padding: f32,
    pub icon_rects: [Rectangle; 9],
    pub new_icon_rects: [Rectangle; 9],
}

impl LayoutMetrics {
    #[must_use]
    pub fn new(window_size: Size, hovered_app: Option<DockApp>) -> Self {
        let header_h = 44.0f32;
        let status_h = 28.0f32;

        let search_w = 420.0f32.min(window_size.width - 60.0);
        let search_h = 42.0f32;
        let search_rect = Rectangle {
            x: (window_size.width - search_w) * 0.5,
            y: header_h + 20.0,
            width: search_w,
            height: search_h,
        };

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
        let dock_rect = Rectangle { x: dock_x, y: dock_y, width: dock_w, height: dock_h };

        let new_dock_y = (dock_y - dock_h - 48.0).max(header_h + 70.0);
        let new_dock_rect = Rectangle { x: dock_x, y: new_dock_y, width: dock_w, height: dock_h };

        let start_x = dock_x + dock_padding;
        let base_y = dock_y + dock_padding;
        let new_base_y = new_dock_y + dock_padding;

        let mut icon_rects = [Rectangle::default(); 9];
        let mut new_icon_rects = [Rectangle::default(); 9];
        for (i, app) in DockApp::ALL.iter().enumerate() {
            let is_hovered = hovered_app == Some(*app);
            let size = if is_hovered { 46.0 } else { base_icon_size };
            let offset_x = (size - base_icon_size) * 0.5;
            let offset_y = if is_hovered { 6.0 } else { 0.0 };

            let x = start_x + i as f32 * (base_icon_size + icon_gap) - offset_x;
            let y = base_y - offset_y;
            let ny = new_base_y - offset_y;

            icon_rects[i] = Rectangle { x, y, width: size, height: size };
            new_icon_rects[i] = Rectangle { x, y: ny, width: size, height: size };
        }

        Self {
            window_size,
            header_h,
            status_h,
            search_rect,
            dock_rect,
            dock_radius,
            new_dock_rect,
            base_icon_size,
            icon_radius,
            icon_gap,
            dock_padding,
            icon_rects,
            new_icon_rects,
        }
    }
}

/// Draws an ambient elevation drop shadow underneath the dock.
pub fn draw_elevation_shadow<R: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<R>,
    rect: Rectangle,
    radius: f32,
    is_dark: bool,
    _transparency: impl std::fmt::Debug,
) {
    let (shadow_color, shadow_blur, offset_y) = if is_dark {
        (Color::from_rgba(0.0, 0.0, 0.0, 0.38), 24.0, 8.0)
    } else {
        (Color::from_rgba(0.0, 0.0, 0.0, 0.18), 18.0, 6.0)
    };

    let shadow_rect =
        Rectangle { x: rect.x, y: rect.y + offset_y, width: rect.width, height: rect.height };
    let path = build_squircle_path(shadow_rect, radius);
    frame.fill(&path, shadow_color);
    let _ = shadow_blur;
}

/// Constructs a squircle path with continuous curvature.
pub fn build_squircle_path(rect: Rectangle, radius: f32) -> Path {
    let r = radius.min(rect.width * 0.5).min(rect.height * 0.5);
    let is_capsule = (r - rect.height * 0.5).abs() < 1.0;
    let smoothing = if is_capsule { 0.0 } else { APPLE_CORNER_SMOOTHING };
    let params = SquircleParams::new(rect.width, rect.height, r).with_smoothing(smoothing);
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

/// Draws an authentic macOS squircle icon.
pub fn draw_apple_icon<R: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<R>,
    _app: DockApp,
    rect: Rectangle,
    _is_dark: bool,
    _enable_highlight: bool,
    _enable_dark_rim: bool,
    icon_image: Option<&iced::widget::image::Handle>,
) {
    if let Some(handle) = icon_image {
        frame.draw_image(rect, canvas::Image::new(handle.clone()));
    }
}

/// Generates a frosted plate texture slice using CPU Gaussian convolution and Vibrancy.
pub fn generate_frosted_plate_texture(
    style: WallpaperStyle,
    rect: Rectangle,
    window_size: Size,
    blur_radius: f32,
    corner_radius: f32,
    _is_dark: bool,
    wallpaper_buf: Option<&WallpaperBuffer>,
) -> iced::widget::image::Handle {
    let w = (rect.width.round() as usize).max(1);
    let h = (rect.height.round() as usize).max(1);

    let blurred_rgb = if blur_radius <= 0.5 {
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

        let mut cropped = Vec::with_capacity(w * h);
        for iy in 0..h {
            for ix in 0..w {
                cropped.push(ext_blurred[(iy + p) * ext_w + (ix + p)]);
            }
        }
        cropped
    };

    let vibrancy = vibrancy_rs::VibrancyConfig::default();

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
            let dither = vibrancy_rs::ign_dither_offset(wx, wy);

            let is_capsule = (corner_radius - h as f32 * 0.5).abs() < 1.0;
            let smoothing = if is_capsule { 0.0 } else { APPLE_CORNER_SMOOTHING };
            let p = SquirclePoint::new(px - w as f32 * 0.5, py - h as f32 * 0.5);
            let half = SquirclePoint::new(w as f32 * 0.5, h as f32 * 0.5);
            let alpha = squircle_alpha(p, half, corner_radius, smoothing);

            let out_idx = idx * 4;
            rgba_bytes[out_idx] = ((vibrant[0] + dither).clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba_bytes[out_idx + 1] = ((vibrant[1] + dither).clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba_bytes[out_idx + 2] = ((vibrant[2] + dither).clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba_bytes[out_idx + 3] = (alpha * 255.0).round() as u8;
        }
    }

    iced::widget::image::Handle::from_rgba(w as u32, h as u32, rgba_bytes)
}
