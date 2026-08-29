use std::f64::consts::{FRAC_PI_2, PI, SQRT_2};

use crate::path::{CubicBezier, Path, PathBuilder, Point};

/// Parameters controlling one G2 corner construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct G2Profile {
    pub extended_fraction: f64,
    pub arc_fraction: f64,
    pub bezier_curvature_scale: f64,
    pub arc_curvature_scale: f64,
}

impl G2Profile {
    /// Apple-like rounded-rectangle defaults from Kyant0/Capsule.
    pub const ROUNDED_RECTANGLE: Self = Self {
        extended_fraction: 0.528_665_1,
        arc_fraction: 5.0 / 9.0,
        bezier_curvature_scale: 1.073_205_1,
        arc_curvature_scale: 1.073_205_1,
    };

    /// G2 capsule defaults from Kyant0/Capsule.
    pub const CAPSULE: Self = Self {
        extended_fraction: 0.528_665_1 * 0.75,
        arc_fraction: 0.0,
        bezier_curvature_scale: 1.0,
        arc_curvature_scale: 1.0,
    };

    /// A profile equivalent to ordinary circular rounded corners.
    pub const G1_EQUIVALENT: Self = Self {
        extended_fraction: 0.0,
        arc_fraction: 1.0,
        bezier_curvature_scale: 1.0,
        arc_curvature_scale: 1.0,
    };

    #[must_use]
    pub fn new(
        extended_fraction: f64,
        arc_fraction: f64,
        bezier_curvature_scale: f64,
        arc_curvature_scale: f64,
    ) -> Self {
        Self {
            extended_fraction: finite_non_negative(extended_fraction),
            arc_fraction: finite_or(arc_fraction, 0.0).clamp(0.0, 1.0),
            bezier_curvature_scale: finite_non_negative(bezier_curvature_scale),
            arc_curvature_scale: finite_or(arc_curvature_scale, 1.0).max(f64::EPSILON),
        }
    }

    #[must_use]
    pub fn lerp(self, stop: Self, fraction: f64) -> Self {
        Self::new(
            lerp(self.extended_fraction, stop.extended_fraction, fraction),
            lerp(self.arc_fraction, stop.arc_fraction, fraction),
            lerp(self.bezier_curvature_scale, stop.bezier_curvature_scale, fraction),
            lerp(self.arc_curvature_scale, stop.arc_curvature_scale, fraction),
        )
    }

    #[must_use]
    pub fn bezier(self) -> CubicBezier {
        let arc_radians = FRAC_PI_2 * self.arc_fraction;
        let bezier_radians = (FRAC_PI_2 - arc_radians) * 0.5;
        let sin = bezier_radians.sin();
        let cos = bezier_radians.cos();

        if approximately_equal(self.bezier_curvature_scale, 1.0)
            && approximately_equal(self.arc_curvature_scale, 1.0)
        {
            let half_tan = sin / (1.0 + cos);
            return CubicBezier::new(
                Point::new(-self.extended_fraction, 0.0),
                Point::new((1.0 - 1.5 / (1.0 + cos)) * half_tan, 0.0),
                Point::new(half_tan, 0.0),
                Point::new(sin, 1.0 - cos),
            );
        }

        let radius_scale = 1.0 / self.arc_curvature_scale;
        let arc_center =
            Point::new(0.0, 1.0) + Point::new(1.0 / SQRT_2, -1.0 / SQRT_2) * (1.0 - radius_scale);
        let arc_start = arc_center + Point::new(sin, -cos) * radius_scale;
        generate_g2_bezier_with_zero_start_curvature(
            Point::new(-self.extended_fraction, 0.0),
            arc_start,
            Point::new(1.0, 0.0),
            Point::new(cos, sin),
            self.bezier_curvature_scale,
        )
    }
}

impl Default for G2Profile {
    fn default() -> Self {
        Self::ROUNDED_RECTANGLE
    }
}

/// The long axis of a resolved capsule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapsuleAxis {
    Horizontal,
    Vertical,
    Circle,
}

/// Geometry shared by CPU paths and the GPU capsule SDF.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedCapsule {
    pub axis: CapsuleAxis,
    pub radius: f64,
    /// The normalized shoulder Bézier. `None` means the capsule is a circle.
    pub shoulder: Option<CubicBezier>,
    pub path: Path,
}

/// Four radii ordered clockwise from the top-left corner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadii {
    pub top_left: f64,
    pub top_right: f64,
    pub bottom_right: f64,
    pub bottom_left: f64,
}

impl CornerRadii {
    #[must_use]
    pub const fn uniform(radius: f64) -> Self {
        Self { top_left: radius, top_right: radius, bottom_right: radius, bottom_left: radius }
    }
}

/// G2 continuous-corner path generator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct G2Continuity {
    pub rounded_rectangle_profile: G2Profile,
    pub capsule_profile: G2Profile,
}

impl Default for G2Continuity {
    fn default() -> Self {
        Self {
            rounded_rectangle_profile: G2Profile::ROUNDED_RECTANGLE,
            capsule_profile: G2Profile::CAPSULE,
        }
    }
}

impl G2Continuity {
    #[must_use]
    pub const fn new(rounded_rectangle_profile: G2Profile, capsule_profile: G2Profile) -> Self {
        Self { rounded_rectangle_profile, capsule_profile }
    }

    #[must_use]
    pub fn capsule(self, width: f64, height: f64) -> ResolvedCapsule {
        let width = finite_non_negative(width);
        let height = finite_non_negative(height);
        if (width - height).abs() <= f64::EPSILON {
            let radius = width * 0.5;
            return ResolvedCapsule {
                axis: CapsuleAxis::Circle,
                radius,
                shoulder: None,
                path: Path::circle(Point::new(radius, radius), radius),
            };
        }
        if width > height {
            self.horizontal_capsule(width, height)
        } else {
            self.vertical_capsule(width, height)
        }
    }

    #[must_use]
    pub fn rounded_rectangle(self, width: f64, height: f64, radii: CornerRadii) -> Path {
        let width = finite_non_negative(width);
        let height = finite_non_negative(height);
        let radii = sanitize_radii(width, height, radii);
        let all_equal = approximately_equal(radii.top_left, radii.top_right)
            && approximately_equal(radii.top_right, radii.bottom_right)
            && approximately_equal(radii.bottom_right, radii.bottom_left);
        if all_equal
            && ((radii.top_left + radii.top_right - width).abs() <= f64::EPSILON
                || (radii.top_left + radii.bottom_left - height).abs() <= f64::EPSILON)
        {
            return self.capsule(width, height).path;
        }
        self.standard_rounded_rectangle(width, height, radii)
    }

    fn capsule_shoulder(self, half_long: f64, radius: f64) -> CubicBezier {
        let ratio = non_capsule_ratio(half_long, radius, self.capsule_profile.extended_fraction);
        let profile = G2Profile::new(
            self.capsule_profile.extended_fraction * ratio,
            self.capsule_profile.arc_fraction,
            lerp(
                self.capsule_profile.bezier_curvature_scale,
                self.rounded_rectangle_profile.bezier_curvature_scale,
                ratio,
            ),
            1.0,
        );
        profile.bezier()
    }

    fn horizontal_capsule(self, width: f64, height: f64) -> ResolvedCapsule {
        let radius = height * 0.5;
        let center_x = width * 0.5;
        let shoulder = self.capsule_shoulder(center_x, radius);
        let scaled = shoulder * radius;
        let offset = -radius * -shoulder.p0.x;
        let arc_radians = FRAC_PI_2 * self.capsule_profile.arc_fraction;
        let bezier_radians = (FRAC_PI_2 - arc_radians) * 0.5;
        let sweep = (bezier_radians + arc_radians) * 2.0;

        let mut path = PathBuilder::new();
        path.move_to(Point::new(0.0, radius));
        path.arc_to(Point::new(radius, radius), radius, FRAC_PI_2 + bezier_radians, sweep);

        let mut x = radius;
        let mut y = 0.0;
        path.cubic_to(
            Point::new(x - scaled.p2.x, y + scaled.p2.y),
            Point::new(x - scaled.p1.x, y + scaled.p1.y),
            Point::new(x - (scaled.p0.x).max(offset), y + scaled.p0.y),
        );

        x = width - radius;
        path.line_to(Point::new(x + offset, y));
        path.cubic_to(
            Point::new(x + scaled.p1.x, y + scaled.p1.y),
            Point::new(x + scaled.p2.x, y + scaled.p2.y),
            Point::new(x + scaled.p3.x, y + scaled.p3.y),
        );

        path.arc_to(
            Point::new(width - radius, radius),
            radius,
            -(FRAC_PI_2 - bezier_radians),
            sweep,
        );

        x = width - radius;
        y = height;
        path.cubic_to(
            Point::new(x + scaled.p2.x, y - scaled.p2.y),
            Point::new(x + scaled.p1.x, y - scaled.p1.y),
            Point::new(x + scaled.p0.x.max(offset), y - scaled.p0.y),
        );

        x = radius;
        path.line_to(Point::new(x - offset, y));
        path.cubic_to(
            Point::new(x - scaled.p1.x, y - scaled.p1.y),
            Point::new(x - scaled.p2.x, y - scaled.p2.y),
            Point::new(x - scaled.p3.x, y - scaled.p3.y),
        );

        ResolvedCapsule {
            axis: CapsuleAxis::Horizontal,
            radius,
            shoulder: Some(shoulder),
            path: path.build(),
        }
    }

    fn vertical_capsule(self, width: f64, height: f64) -> ResolvedCapsule {
        let radius = width * 0.5;
        let center_y = height * 0.5;
        let shoulder = self.capsule_shoulder(center_y, radius);
        let scaled = shoulder * radius;
        let offset = -radius * -shoulder.p0.x;
        let arc_radians = FRAC_PI_2 * self.capsule_profile.arc_fraction;
        let bezier_radians = (FRAC_PI_2 - arc_radians) * 0.5;
        let sweep = (bezier_radians + arc_radians) * 2.0;

        let mut path = PathBuilder::new();
        let mut x = 0.0;
        let mut y = radius;
        path.move_to(Point::new(x, y - offset));
        path.cubic_to(
            Point::new(x + scaled.p1.y, y - scaled.p1.x),
            Point::new(x + scaled.p2.y, y - scaled.p2.x),
            Point::new(x + scaled.p3.y, y - scaled.p3.x),
        );

        path.arc_to(Point::new(radius, radius), radius, -(PI - bezier_radians), sweep);

        x = width;
        y = radius;
        path.cubic_to(
            Point::new(x - scaled.p2.y, y - scaled.p2.x),
            Point::new(x - scaled.p1.y, y - scaled.p1.x),
            Point::new(x - scaled.p0.y, y - scaled.p0.x.max(offset)),
        );

        y = height - radius;
        path.line_to(Point::new(x, y + offset));
        path.cubic_to(
            Point::new(x - scaled.p1.y, y + scaled.p1.x),
            Point::new(x - scaled.p2.y, y + scaled.p2.x),
            Point::new(x - scaled.p3.y, y + scaled.p3.x),
        );

        path.arc_to(Point::new(radius, height - radius), radius, bezier_radians, sweep);

        x = 0.0;
        y = height - radius;
        path.cubic_to(
            Point::new(x + scaled.p2.y, y + scaled.p2.x),
            Point::new(x + scaled.p1.y, y + scaled.p1.x),
            Point::new(x + scaled.p0.y, y + scaled.p0.x.max(offset)),
        );

        ResolvedCapsule {
            axis: CapsuleAxis::Vertical,
            radius,
            shoulder: Some(shoulder),
            path: path.build(),
        }
    }

    #[allow(clippy::similar_names)]
    fn standard_rounded_rectangle(self, width: f64, height: f64, r: CornerRadii) -> Path {
        // This follows Kyant0/Capsule's progressive transition between its
        // capsule and rounded-rectangle profiles when an edge becomes short.
        let cx = width * 0.5;
        let cy = height * 0.5;
        let p = self.rounded_rectangle_profile;
        let c = self.capsule_profile;

        let tlv = non_capsule_ratio(cy, r.top_left, p.extended_fraction);
        let tlh = non_capsule_ratio(cx, r.top_left, p.extended_fraction);
        let trh = non_capsule_ratio(cx, r.top_right, p.extended_fraction);
        let trv = non_capsule_ratio(cy, r.top_right, p.extended_fraction);
        let brv = non_capsule_ratio(cy, r.bottom_right, p.extended_fraction);
        let brh = non_capsule_ratio(cx, r.bottom_right, p.extended_fraction);
        let blh = non_capsule_ratio(cx, r.bottom_left, p.extended_fraction);
        let blv = non_capsule_ratio(cy, r.bottom_left, p.extended_fraction);

        let tl = tlv.min(tlh);
        let tr = trh.min(trv);
        let br = brv.min(brh);
        let bl = blh.min(blv);

        let ext_tl = lerp(c.extended_fraction, p.extended_fraction, tl);
        let ext_tr = lerp(c.extended_fraction, p.extended_fraction, tr);
        let ext_br = lerp(c.extended_fraction, p.extended_fraction, br);
        let ext_bl = lerp(c.extended_fraction, p.extended_fraction, bl);

        let offsets = [
            -r.top_left * ext_tl * tlv,
            -r.top_left * ext_tl * tlh,
            -r.top_right * ext_tr * trh,
            -r.top_right * ext_tr * trv,
            -r.bottom_right * ext_br * brv,
            -r.bottom_right * ext_br * brh,
            -r.bottom_left * ext_bl * blh,
            -r.bottom_left * ext_bl * blv,
        ];
        let ratios = [tlv, tlh, trh, trv, brv, brh, blh, blv];
        let corner_ratios = [tl, tl, tr, tr, br, br, bl, bl];
        let radii = [
            r.top_left,
            r.top_left,
            r.top_right,
            r.top_right,
            r.bottom_right,
            r.bottom_right,
            r.bottom_left,
            r.bottom_left,
        ];
        let shoulders = std::array::from_fn::<_, 8, _>(|index| {
            let corner_ratio = corner_ratios[index];
            let half_ratio = ratios[index];
            let profile = G2Profile::new(
                lerp(c.extended_fraction, p.extended_fraction, corner_ratio) * half_ratio,
                lerp(c.arc_fraction, p.arc_fraction, corner_ratio),
                lerp(c.bezier_curvature_scale, p.bezier_curvature_scale, half_ratio),
                1.0 + (p.arc_curvature_scale - 1.0) * corner_ratio,
            );
            profile.bezier() * radii[index]
        });
        let arc_fractions = [
            lerp(c.arc_fraction, p.arc_fraction, tl),
            lerp(c.arc_fraction, p.arc_fraction, tr),
            lerp(c.arc_fraction, p.arc_fraction, br),
            lerp(c.arc_fraction, p.arc_fraction, bl),
        ];
        let arc_scales = [
            1.0 + (p.arc_curvature_scale - 1.0) * tl,
            1.0 + (p.arc_curvature_scale - 1.0) * tr,
            1.0 + (p.arc_curvature_scale - 1.0) * br,
            1.0 + (p.arc_curvature_scale - 1.0) * bl,
        ];

        build_rounded_rectangle_path(
            width,
            height,
            r,
            &shoulders,
            &offsets,
            arc_fractions,
            arc_scales,
        )
    }
}

fn generate_g2_bezier_with_zero_start_curvature(
    start: Point,
    end: Point,
    start_tangent: Point,
    end_tangent: Point,
    end_curvature: f64,
) -> CubicBezier {
    let a2 = 1.5 * end_curvature;
    let b = start_tangent.x * end_tangent.y - start_tangent.y * end_tangent.x;
    let delta = end - start;
    let c1 = -delta.y * start_tangent.x + delta.x * start_tangent.y;
    let c2 = delta.y * end_tangent.x - delta.x * end_tangent.y;
    let lambda0 = -c2 / b - a2 * c1 * c1 / b.powi(3);
    let lambda3 = -c1 / b;
    CubicBezier::new(
        start,
        start
            + Point::new(
                (lambda0 * start_tangent.x).max(0.0),
                (lambda0 * start_tangent.y).max(0.0),
            ),
        end - Point::new((lambda3 * end_tangent.x).max(0.0), (lambda3 * end_tangent.y).max(0.0)),
        end,
    )
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn build_rounded_rectangle_path(
    width: f64,
    height: f64,
    radii: CornerRadii,
    shoulders: &[CubicBezier; 8],
    offsets: &[f64; 8],
    arc_fractions: [f64; 4],
    arc_scales: [f64; 4],
) -> Path {
    let mut path = PathBuilder::new();
    let mut current_x = 0.0;
    let mut current_y = radii.top_left;
    path.move_to(Point::new(current_x, current_y - offsets[0]));

    if radii.top_left > 0.0 {
        path.cubic_to(
            Point::new(current_x + shoulders[0].p1.y, current_y - shoulders[0].p1.x),
            Point::new(current_x + shoulders[0].p2.y, current_y - shoulders[0].p2.x),
            Point::new(current_x + shoulders[0].p3.y, current_y - shoulders[0].p3.x),
        );
        arc_scaled(
            &mut path,
            Point::new(radii.top_left, radii.top_left),
            radii.top_left,
            1.0 / arc_scales[0],
            PI + FRAC_PI_2 * (1.0 - arc_fractions[0]) * 0.5,
            FRAC_PI_2 * arc_fractions[0],
        );
        current_x = radii.top_left;
        current_y = 0.0;
        path.cubic_to(
            Point::new(current_x - shoulders[1].p2.x, current_y + shoulders[1].p2.y),
            Point::new(current_x - shoulders[1].p1.x, current_y + shoulders[1].p1.y),
            Point::new(
                current_x - shoulders[1].p0.x.max(offsets[1]),
                current_y + shoulders[1].p0.y,
            ),
        );
    }

    current_x = width - radii.top_right;
    path.line_to(Point::new(current_x + offsets[2], 0.0));
    if radii.top_right > 0.0 {
        path.cubic_to(
            Point::new(current_x + shoulders[2].p1.x, shoulders[2].p1.y),
            Point::new(current_x + shoulders[2].p2.x, shoulders[2].p2.y),
            Point::new(current_x + shoulders[2].p3.x, shoulders[2].p3.y),
        );
        arc_scaled(
            &mut path,
            Point::new(width - radii.top_right, radii.top_right),
            radii.top_right,
            1.0 / arc_scales[1],
            -FRAC_PI_2 + FRAC_PI_2 * (1.0 - arc_fractions[1]) * 0.5,
            FRAC_PI_2 * arc_fractions[1],
        );
        current_x = width;
        current_y = radii.top_right;
        path.cubic_to(
            Point::new(current_x - shoulders[3].p2.y, current_y - shoulders[3].p2.x),
            Point::new(current_x - shoulders[3].p1.y, current_y - shoulders[3].p1.x),
            Point::new(
                current_x - shoulders[3].p0.y,
                current_y - shoulders[3].p0.x.max(offsets[3]),
            ),
        );
    }

    current_y = height - radii.bottom_right;
    path.line_to(Point::new(width, current_y + offsets[4]));
    if radii.bottom_right > 0.0 {
        path.cubic_to(
            Point::new(width - shoulders[4].p1.y, current_y + shoulders[4].p1.x),
            Point::new(width - shoulders[4].p2.y, current_y + shoulders[4].p2.x),
            Point::new(width - shoulders[4].p3.y, current_y + shoulders[4].p3.x),
        );
        arc_scaled(
            &mut path,
            Point::new(width - radii.bottom_right, height - radii.bottom_right),
            radii.bottom_right,
            1.0 / arc_scales[2],
            FRAC_PI_2 * (1.0 - arc_fractions[2]) * 0.5,
            FRAC_PI_2 * arc_fractions[2],
        );
        current_x = width - radii.bottom_right;
        current_y = height;
        path.cubic_to(
            Point::new(current_x + shoulders[5].p2.x, current_y - shoulders[5].p2.y),
            Point::new(current_x + shoulders[5].p1.x, current_y - shoulders[5].p1.y),
            Point::new(
                current_x + shoulders[5].p0.x.max(offsets[5]),
                current_y - shoulders[5].p0.y,
            ),
        );
    }

    current_x = radii.bottom_left;
    path.line_to(Point::new(current_x - offsets[6], height));
    if radii.bottom_left > 0.0 {
        path.cubic_to(
            Point::new(current_x - shoulders[6].p1.x, height - shoulders[6].p1.y),
            Point::new(current_x - shoulders[6].p2.x, height - shoulders[6].p2.y),
            Point::new(current_x - shoulders[6].p3.x, height - shoulders[6].p3.y),
        );
        arc_scaled(
            &mut path,
            Point::new(radii.bottom_left, height - radii.bottom_left),
            radii.bottom_left,
            1.0 / arc_scales[3],
            FRAC_PI_2 + FRAC_PI_2 * (1.0 - arc_fractions[3]) * 0.5,
            FRAC_PI_2 * arc_fractions[3],
        );
        current_x = 0.0;
        current_y = height - radii.bottom_left;
        path.cubic_to(
            Point::new(current_x + shoulders[7].p2.y, current_y + shoulders[7].p2.x),
            Point::new(current_x + shoulders[7].p1.y, current_y + shoulders[7].p1.x),
            Point::new(
                current_x + shoulders[7].p0.y,
                current_y + shoulders[7].p0.x.max(offsets[7]),
            ),
        );
    }
    path.close();
    path.build()
}

fn arc_scaled(
    path: &mut PathBuilder,
    center: Point,
    radius: f64,
    radius_scale: f64,
    start_angle: f64,
    sweep_angle: f64,
) {
    let center_angle = start_angle + sweep_angle * 0.5;
    let adjusted_center =
        center + Point::new(center_angle.cos(), center_angle.sin()) * radius * (1.0 - radius_scale);
    path.arc_to(adjusted_center, radius * radius_scale, start_angle, sweep_angle);
}

fn sanitize_radii(width: f64, height: f64, radii: CornerRadii) -> CornerRadii {
    let maximum = width.min(height) * 0.5;
    CornerRadii {
        top_left: finite_non_negative(radii.top_left).min(maximum),
        top_right: finite_non_negative(radii.top_right).min(maximum),
        bottom_right: finite_non_negative(radii.bottom_right).min(maximum),
        bottom_left: finite_non_negative(radii.bottom_left).min(maximum),
    }
}

fn non_capsule_ratio(half_extent: f64, radius: f64, extended_fraction: f64) -> f64 {
    if radius <= f64::EPSILON || extended_fraction <= f64::EPSILON {
        1.0
    } else {
        ((half_extent / radius - 1.0) / extended_fraction).clamp(0.0, 1.0)
    }
}

fn lerp(start: f64, stop: f64, fraction: f64) -> f64 {
    start + (stop - start) * fraction
}

fn finite_non_negative(value: f64) -> f64 {
    finite_or(value, 0.0).max(0.0)
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= f64::EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PathSegment;

    #[test]
    fn capsule_profile_matches_upstream_defaults() {
        assert!((G2Profile::CAPSULE.extended_fraction - 0.396_498_825).abs() < 1.0e-12);
        assert!(G2Profile::CAPSULE.arc_fraction.abs() < 1.0e-12);
        assert!((G2Profile::ROUNDED_RECTANGLE.arc_fraction - 5.0 / 9.0).abs() < 1.0e-12);
    }

    #[test]
    fn horizontal_capsule_uses_trimmed_arcs_and_g2_shoulders() {
        let capsule = G2Continuity::default().capsule(72.0, 36.0);
        assert_eq!(capsule.axis, CapsuleAxis::Horizontal);
        assert_eq!(capsule.path.segments().len(), 8);
        let shoulder = capsule.shoulder.expect("non-circular capsule has a shoulder");
        assert!(shoulder.curvature(0.0) < 1.0e-10);
        assert!(
            (shoulder.curvature(1.0) - G2Profile::ROUNDED_RECTANGLE.bezier_curvature_scale).abs()
                < 1.0e-10
        );
        let PathSegment::Arc { sweep_angle, .. } = capsule.path.segments()[0] else {
            panic!("first capsule segment must be the left arc");
        };
        assert!((sweep_angle - FRAC_PI_2).abs() < 1.0e-12);
    }

    #[test]
    fn square_capsule_is_an_exact_circle() {
        let capsule = G2Continuity::default().capsule(36.0, 36.0);
        assert_eq!(capsule.axis, CapsuleAxis::Circle);
        assert!(capsule.shoulder.is_none());
        assert!(matches!(capsule.path.segments(), [PathSegment::Circle { .. }]));
    }

    #[test]
    fn rounded_rectangle_builds_all_four_g2_corners() {
        let path =
            G2Continuity::default().rounded_rectangle(160.0, 90.0, CornerRadii::uniform(18.0));
        assert_eq!(path.segments().len(), 16);
        assert_eq!(
            path.segments()
                .iter()
                .filter(|segment| matches!(segment, PathSegment::Cubic(_)))
                .count(),
            8
        );
    }
}
