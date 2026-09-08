//! Popover squircle path generation, multi-tier soft shadows, and canvas program.

use iced::{
    Color, Point, Rectangle, Size, Theme, mouse,
    widget::canvas::{self, Frame, Geometry},
};
use bmol_designs::{
    menu_metrics,
    popover_metrics::{
        popover_arrow_profile_height_with_spline, PopoverArrowConfig, PopoverArrowEdge,
    },
};
use liquid_glass::geometry::APPLE_CORNER_SMOOTHING;

use crate::vibrancy::{
    sample_analytical_blurred_wallpaper, BlurPreset, WallpaperStyle,
};

/// An occlusion region where a context menu card or popup overlays the wallpaper,
/// requiring genuine continuous backdrop blur spatial convolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuOcclusion {
    pub bounds: Rectangle,
    pub corner_radius: f32,
    pub arrow: PopoverArrowConfig,
    pub is_dark: bool,
}

pub use liquid_glass::popover::{
    build_popover_path as build_popover_squircle_path,
    fill_popover, stroke_popover_rim,
};

/// Renders multi-layer soft drop shadows matching macOS Popover Window shadow geometry,
/// continuously wrapping both the squircle menu body and the popover arrow (触角).
pub fn render_soft_menu_shadow(
    frame: &mut Frame,
    occ: &MenuOcclusion,
) {
    liquid_glass::popover::render_popover_shadow(
        frame,
        occ.bounds,
        occ.corner_radius,
        occ.arrow,
        occ.is_dark,
    );
}

/// Renders a continuous, artifact-free blurred occlusion clipped to continuous squircle/rounded corners
/// and integrated popover arrow (触角).
pub fn render_blurred_occlusion(
    frame: &mut Frame,
    style: WallpaperStyle,
    occ: &MenuOcclusion,
    blur_radius: f32,
    bounds: Size,
) {
    let rect = occ.bounds;
    let corner_radius = occ.corner_radius;
    let arrow = occ.arrow;
    let is_dark = occ.is_dark;

    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }

    if matches!(style, WallpaperStyle::PureWhite) {
        fill_popover(frame, rect, corner_radius, arrow, Color::WHITE);
        return;
    }
    if matches!(style, WallpaperStyle::PureBlack) {
        fill_popover(frame, rect, corner_radius, arrow, Color::BLACK);
        return;
    }

    let slice_w = 2.0f32;
    let r = corner_radius.min(rect.width * 0.5).min(rect.height * 0.5);

    // Continuous Apple squircle analytical curvature clipping (exponent n = 3.32)
    let exp_n = 2.0 + (4.2 - 2.0) * APPLE_CORNER_SMOOTHING;
    let inv_exp_n = 1.0 / exp_n;
    let r_pow_n = r.powf(exp_n);

    let extra_left = if arrow.edge == PopoverArrowEdge::Left { arrow.height } else { 0.0 };
    let extra_right = if arrow.edge == PopoverArrowEdge::Right { arrow.height } else { 0.0 };

    let mut curr_x = rect.x - extra_left;
    let end_x = rect.x + rect.width + extra_right;

    let arrow_half_w = arrow.base_width * 0.5;
    let p = ((1.0 + APPLE_CORNER_SMOOTHING) * r).min(rect.width * 0.5).min(rect.height * 0.5);
    let min_x = rect.x + p + arrow_half_w;
    let max_x = (rect.x + rect.width - p - arrow_half_w).max(min_x);
    let arrow_center_x = (rect.x + rect.width * arrow.offset).clamp(min_x, max_x);

    let min_y = rect.y + p + arrow_half_w;
    let max_y = (rect.y + rect.height - p - arrow_half_w).max(min_y);
    let arrow_center_y = (rect.y + rect.height * arrow.offset).clamp(min_y, max_y);

    while curr_x < end_x {
        let actual_w = (end_x - curr_x).min(slice_w);
        let sample_x = curr_x + actual_w * 0.5;

        let local_x = sample_x - rect.x;
        let inset_y = if local_x >= 0.0 && local_x < rect.width {
            if local_x < r {
                let dx = r - local_x;
                let dy = (r_pow_n - dx.powf(exp_n)).max(0.0).powf(inv_exp_n);
                r - dy
            } else if local_x > rect.width - r {
                let dx = local_x - (rect.width - r);
                let dy = (r_pow_n - dx.powf(exp_n)).max(0.0).powf(inv_exp_n);
                r - dy
            } else {
                0.0
            }
        } else {
            0.0
        };

        let top_extension = if arrow.edge == PopoverArrowEdge::Top && (sample_x - arrow_center_x).abs() < arrow_half_w {
            let dist = ((sample_x - arrow_center_x).abs() / arrow_half_w).clamp(0.0, 1.0);
            popover_arrow_profile_height_with_spline(&arrow.spline, dist) * arrow.height
        } else {
            0.0
        };

        let bottom_extension = if arrow.edge == PopoverArrowEdge::Bottom && (sample_x - arrow_center_x).abs() < arrow_half_w {
            let dist = ((sample_x - arrow_center_x).abs() / arrow_half_w).clamp(0.0, 1.0);
            popover_arrow_profile_height_with_spline(&arrow.spline, dist) * arrow.height
        } else {
            0.0
        };

        let is_in_side_arrow = (arrow.edge == PopoverArrowEdge::Left && sample_x < rect.x)
            || (arrow.edge == PopoverArrowEdge::Right && sample_x > rect.x + rect.width);

        let (slice_top, slice_height) = if is_in_side_arrow {
            let dist_x = if arrow.edge == PopoverArrowEdge::Left {
                (rect.x - sample_x) / arrow.height
            } else {
                (sample_x - (rect.x + rect.width)) / arrow.height
            };
            let bell = popover_arrow_profile_height_with_spline(&arrow.spline, dist_x.clamp(0.0, 1.0));
            let current_half_w = arrow_half_w * bell;
            (arrow_center_y - current_half_w, current_half_w * 2.0)
        } else {
            let top = rect.y + inset_y - top_extension;
            let height = (rect.height - inset_y * 2.0 + top_extension + bottom_extension).max(0.0);
            (top, height)
        };

        if slice_height > 0.0 {
            let color = sample_analytical_blurred_wallpaper(
                style,
                sample_x,
                rect.y + rect.height * 0.5,
                bounds,
                blur_radius,
            );
            frame.fill_rectangle(
                Point::new(curr_x, slice_top),
                Size::new(actual_w.ceil(), slice_height),
                color,
            );
        }

        curr_x += actual_w;
    }

    // Physical glass base tint overlay over the entire unified squircle+arrow
    let (base_r, base_g, base_b, base_a) = if is_dark {
        menu_metrics::DARK_MENU_BASE_RGBA_F32
    } else {
        menu_metrics::LIGHT_MENU_BASE_RGBA_F32
    };
    fill_popover(frame, rect, corner_radius, arrow, Color::from_rgba(base_r, base_g, base_b, base_a));

    // 1px fine rim highlight tracing the complete unified silhouette
    let rim_color = if is_dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.18)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.12)
    };
    stroke_popover_rim(frame, rect, corner_radius, arrow, rim_color, 1.0);
}

/// Canvas program rendering television color test blocks with continuous analytical backdrop blur occlusions.
#[derive(Debug)]
pub struct WallpaperCanvas {
    pub style: WallpaperStyle,
    pub blur_preset: BlurPreset,
    pub occlusions: Vec<MenuOcclusion>,
    pub show_calibration_grid: bool,
}

impl<Message> canvas::Program<Message> for WallpaperCanvas {
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

        // 1. Draw base sharp television wallpaper
        match self.style {
            WallpaperStyle::TvColorBars => {
                // 8 standard TV primary & secondary color bars across screen
                let colors = [
                    Color::WHITE,                   // 0: 白 (255, 255, 255)
                    Color::from_rgb(1.0, 1.0, 0.0), // 1: 黄 (255, 255, 0)
                    Color::from_rgb(0.0, 1.0, 1.0), // 2: 青 (0, 255, 255)
                    Color::from_rgb(0.0, 1.0, 0.0), // 3: 绿 (0, 255, 0)
                    Color::from_rgb(1.0, 0.0, 1.0), // 4: 洋红 (255, 0, 255)
                    Color::from_rgb(1.0, 0.0, 0.0), // 5: 红 (255, 0, 0)
                    Color::from_rgb(0.0, 0.0, 1.0), // 6: 蓝 (0, 0, 255)
                    Color::BLACK,                   // 7: 黑 (0, 0, 0)
                ];
                let n = colors.len() as f32;
                let bar_w = bounds.width / n;
                for (i, &c) in colors.iter().enumerate() {
                    let x = i as f32 * bar_w;
                    let rect_w = if i == colors.len() - 1 {
                        bounds.width - x
                    } else {
                        bar_w.ceil()
                    };
                    frame.fill_rectangle(
                        Point::new(x, 0.0),
                        Size::new(rect_w, bounds.height),
                        c,
                    );
                }
            }
            WallpaperStyle::TvSmpteSplit => {
                let top_h = bounds.height * 0.70;
                let bot_h = bounds.height - top_h;

                let top_colors = [
                    Color::WHITE,                   // 白
                    Color::from_rgb(1.0, 1.0, 0.0), // 黄
                    Color::from_rgb(0.0, 1.0, 1.0), // 青
                    Color::from_rgb(0.0, 1.0, 0.0), // 绿
                    Color::from_rgb(1.0, 0.0, 1.0), // 洋红
                    Color::from_rgb(1.0, 0.0, 0.0), // 红
                    Color::from_rgb(0.0, 0.0, 1.0), // 蓝
                ];
                let top_n = top_colors.len() as f32;
                let top_bar_w = bounds.width / top_n;
                for (i, &c) in top_colors.iter().enumerate() {
                    let x = i as f32 * top_bar_w;
                    frame.fill_rectangle(
                        Point::new(x, 0.0),
                        Size::new(top_bar_w.ceil(), top_h),
                        c,
                    );
                }

                let bot_colors = [
                    Color::from_rgb(0.0, 0.0, 1.0), // 蓝
                    Color::BLACK,                   // 黑
                    Color::from_rgb(1.0, 0.0, 1.0), // 洋红
                    Color::BLACK,                   // 黑
                    Color::from_rgb(0.0, 1.0, 1.0), // 青
                    Color::BLACK,                   // 黑
                    Color::WHITE,                   // 白
                    Color::BLACK,                   // 黑
                ];
                let bot_n = bot_colors.len() as f32;
                let bot_bar_w = bounds.width / bot_n;
                for (i, &c) in bot_colors.iter().enumerate() {
                    let x = i as f32 * bot_bar_w;
                    frame.fill_rectangle(
                        Point::new(x, top_h),
                        Size::new(bot_bar_w.ceil(), bot_h),
                        c,
                    );
                }
            }
            WallpaperStyle::TvColorGrid => {
                let cols = 4;
                let rows = 3;
                let cell_w = bounds.width / cols as f32;
                let cell_h = bounds.height / rows as f32;
                let palette = [
                    Color::WHITE,
                    Color::from_rgb(1.0, 1.0, 0.0),
                    Color::from_rgb(0.0, 1.0, 1.0),
                    Color::from_rgb(0.0, 1.0, 0.0),
                    Color::from_rgb(1.0, 0.0, 1.0),
                    Color::from_rgb(1.0, 0.0, 0.0),
                    Color::from_rgb(0.0, 0.0, 1.0),
                    Color::BLACK,
                    Color::from_rgb(1.0, 0.5, 0.0),
                    Color::from_rgb(0.5, 0.0, 0.5),
                    Color::from_rgb(0.0, 0.5, 0.5),
                    Color::from_rgb(0.5, 0.5, 0.5),
                ];
                for r in 0..rows {
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let x = c as f32 * cell_w;
                        let y = r as f32 * cell_h;
                        frame.fill_rectangle(
                            Point::new(x, y),
                            Size::new(cell_w.ceil(), cell_h.ceil()),
                            palette[idx % palette.len()],
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

        // 2. Draw high-frequency test patterns & calibration grid on sharp background
        if self.show_calibration_grid
            && !matches!(self.style, WallpaperStyle::PureWhite | WallpaperStyle::PureBlack)
        {
            let grid_step = 36.0f32;
            let line_color = Color::from_rgba(1.0, 1.0, 1.0, 0.16);
            let dark_line_color = Color::from_rgba(0.0, 0.0, 0.0, 0.18);

            // Vertical fine grid lines
            let mut x = grid_step;
            while x < bounds.width {
                frame.fill_rectangle(Point::new(x, 0.0), Size::new(1.0, bounds.height), line_color);
                x += grid_step;
            }

            // Horizontal fine grid lines
            let mut y = grid_step;
            while y < bounds.height {
                frame.fill_rectangle(Point::new(0.0, y), Size::new(bounds.width, 1.0), dark_line_color);
                y += grid_step;
            }

            // Central crosshair & calibration test target
            let cx = (bounds.width * 0.5).round();
            let cy = (bounds.height * 0.5).round();
            frame.fill_rectangle(Point::new(cx - 60.0, cy - 1.0), Size::new(120.0, 2.0), Color::WHITE);
            frame.fill_rectangle(Point::new(cx - 1.0, cy - 60.0), Size::new(2.0, 120.0), Color::WHITE);
        }

        // 3. Render authentic Apple multi-tier soft drop shadows and continuous analytical backdrop blur
        if !self.occlusions.is_empty() {
            let blur_radius = self.blur_preset.radius();

            // Step 3a: Soft ambient contact shadow & elevation drop shadow behind each menu card
            for occ in &self.occlusions {
                render_soft_menu_shadow(&mut frame, occ);
            }

            // Step 3b: Continuous analytical backdrop blur inside each menu container squircle + arrow
            for occ in &self.occlusions {
                render_blurred_occlusion(
                    &mut frame,
                    self.style,
                    occ,
                    blur_radius,
                    bounds.size(),
                );
            }
        }

        vec![frame.into_geometry()]
    }
}
