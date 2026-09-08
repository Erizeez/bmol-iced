//! Television wallpaper styles, blur presets, and continuous analytical Gaussian / Dual-Kawase convolution sampling.

use iced::{Color, Size};

/// Television color block backgrounds and calibration test patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallpaperStyle {
    #[default]
    TvColorBars,
    TvSmpteSplit,
    TvColorGrid,
    PureWhite,
    PureBlack,
}

impl WallpaperStyle {
    pub const ALL: [Self; 5] = [
        Self::TvColorBars,
        Self::TvSmpteSplit,
        Self::TvColorGrid,
        Self::PureWhite,
        Self::PureBlack,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TvColorBars => "TV 彩条 (8色)",
            Self::TvSmpteSplit => "SMPTE 双层彩条",
            Self::TvColorGrid => "彩色网格色块",
            Self::PureWhite => "Pure White (255)",
            Self::PureBlack => "Pure Black (0)",
        }
    }
}

/// Blur strength presets demonstrating the dramatic difference of heavy frosted glass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlurPreset {
    #[default]
    UltraHeavy64,
    Heavy48,
    Medium32,
    Subtle16,
}

impl BlurPreset {
    pub const ALL: [Self; 4] = [Self::UltraHeavy64, Self::Heavy48, Self::Medium32, Self::Subtle16];

    #[must_use]
    pub const fn radius(self) -> f32 {
        match self {
            Self::UltraHeavy64 => 64.0,
            Self::Heavy48 => 48.0,
            Self::Medium32 => 32.0,
            Self::Subtle16 => 16.0,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::UltraHeavy64 => "64pt (Ultra-Heavy HIG)",
            Self::Heavy48 => "48pt (Heavy)",
            Self::Medium32 => "32pt (Medium)",
            Self::Subtle16 => "16pt (Subtle)",
        }
    }
}

/// High-precision approximation of the error function erf(x).
/// Maximum error < 1.5e-7 (Abramowitz and Stegun formula 7.1.26).
#[inline]
#[must_use]
pub fn approx_erf(x: f32) -> f32 {
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
#[must_use]
pub fn normal_cdf(z: f32) -> f32 {
    0.5 * (1.0 + approx_erf(z * std::f32::consts::FRAC_1_SQRT_2))
}

/// Evaluates genuine continuous Gaussian / Dual-Kawase convolution for 1D horizontal segments.
///
/// For an interval [x_left, x_right] with uniform color, the Gaussian convolution integral at x is:
/// weight = Phi((x_right - x) / sigma) - Phi((x_left - x) / sigma).
///
/// This provides 100% artifact-free, aliasing-free continuous blur without sparse sampling spikes.
#[inline]
#[must_use]
pub fn segment_gaussian_weight(x: f32, x_left: f32, x_right: f32, inv_sigma: f32) -> f32 {
    let cdf_right = normal_cdf((x_right - x) * inv_sigma);
    let cdf_left = normal_cdf((x_left - x) * inv_sigma);
    (cdf_right - cdf_left).max(0.0)
}

/// Continuous analytical Gaussian convolution for 8 TV color bars.
#[must_use]
pub fn sample_blurred_tv_bars(x: f32, bounds_width: f32, blur_radius: f32) -> [f32; 3] {
    const TV_BARS: [[f32; 3]; 8] = [
        [1.0, 1.0, 1.0], // 0: 白 (255, 255, 255)
        [1.0, 1.0, 0.0], // 1: 黄 (255, 255, 0)
        [0.0, 1.0, 1.0], // 2: 青 (0, 255, 255)
        [0.0, 1.0, 0.0], // 3: 绿 (0, 255, 0)
        [1.0, 0.0, 1.0], // 4: 洋红 (255, 0, 255)
        [1.0, 0.0, 0.0], // 5: 红 (255, 0, 0)
        [0.0, 0.0, 1.0], // 6: 蓝 (0, 0, 255)
        [0.0, 0.0, 0.0], // 7: 黑 (0, 0, 0)
    ];

    let n = 8.0f32;
    let bar_w = bounds_width / n;
    let sigma = (blur_radius * 0.40).max(1.0);
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
#[must_use]
pub fn sample_blurred_tv_smpte(x: f32, y: f32, bounds: Size, blur_radius: f32) -> [f32; 3] {
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
    let sigma = (blur_radius * 0.40).max(1.0);
    let inv_sigma = 1.0 / sigma;

    // Vertical blending between top 70% and bottom 30%
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
#[must_use]
pub fn sample_blurred_tv_grid(x: f32, y: f32, bounds: Size, blur_radius: f32) -> [f32; 3] {
    const GRID_PALETTE: [[f32; 3]; 12] = [
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.5, 0.0],
        [0.5, 0.0, 0.5],
        [0.0, 0.5, 0.5],
        [0.5, 0.5, 0.5],
    ];

    let cols = 4.0f32;
    let rows = 3.0f32;
    let cell_w = bounds.width / cols;
    let cell_h = bounds.height / rows;
    let sigma = (blur_radius * 0.40).max(1.0);
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

/// Continuous analytical blurred wallpaper color at (x, y).
#[must_use]
pub fn sample_analytical_blurred_wallpaper(
    style: WallpaperStyle,
    x: f32,
    y: f32,
    bounds: Size,
    blur_radius: f32,
) -> Color {
    match style {
        WallpaperStyle::PureWhite => Color::WHITE,
        WallpaperStyle::PureBlack => Color::BLACK,
        WallpaperStyle::TvColorBars => {
            let rgb = sample_blurred_tv_bars(x, bounds.width, blur_radius);
            Color::from_rgb(rgb[0], rgb[1], rgb[2])
        }
        WallpaperStyle::TvSmpteSplit => {
            let rgb = sample_blurred_tv_smpte(x, y, bounds, blur_radius);
            Color::from_rgb(rgb[0], rgb[1], rgb[2])
        }
        WallpaperStyle::TvColorGrid => {
            let rgb = sample_blurred_tv_grid(x, y, bounds, blur_radius);
            Color::from_rgb(rgb[0], rgb[1], rgb[2])
        }
    }
}
