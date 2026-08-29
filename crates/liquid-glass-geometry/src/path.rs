use std::f64::consts::PI;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A double-precision point in logical-pixel space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    #[must_use]
    pub fn normalized(self) -> Self {
        let length = self.length();
        if length > f64::EPSILON { self / length } else { Self::default() }
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Point {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f64> for Point {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<f64> for Point {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Point {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

/// Four control points defining a cubic Bézier segment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CubicBezier {
    pub p0: Point,
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
}

impl CubicBezier {
    #[must_use]
    pub const fn new(p0: Point, p1: Point, p2: Point, p3: Point) -> Self {
        Self { p0, p1, p2, p3 }
    }

    #[must_use]
    pub fn point(self, t: f64) -> Point {
        let one_minus_t = 1.0 - t;
        self.p0 * one_minus_t.powi(3)
            + self.p1 * (3.0 * one_minus_t.powi(2) * t)
            + self.p2 * (3.0 * one_minus_t * t * t)
            + self.p3 * t.powi(3)
    }

    #[must_use]
    pub fn derivative(self, t: f64) -> Point {
        let one_minus_t = 1.0 - t;
        (self.p1 - self.p0) * (3.0 * one_minus_t.powi(2))
            + (self.p2 - self.p1) * (6.0 * one_minus_t * t)
            + (self.p3 - self.p2) * (3.0 * t * t)
    }

    #[must_use]
    pub fn second_derivative(self, t: f64) -> Point {
        (self.p2 - self.p1 * 2.0 + self.p0) * (6.0 * (1.0 - t))
            + (self.p3 - self.p2 * 2.0 + self.p1) * (6.0 * t)
    }

    #[must_use]
    pub fn curvature(self, t: f64) -> f64 {
        let first = self.derivative(t);
        let second = self.second_derivative(t);
        let denominator = first.length().powi(3);
        if denominator <= f64::EPSILON {
            0.0
        } else {
            (first.x * second.y - first.y * second.x).abs() / denominator
        }
    }
}

impl Mul<f64> for CubicBezier {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.p0 * rhs, self.p1 * rhs, self.p2 * rhs, self.p3 * rhs)
    }
}

/// One exact segment of a continuous-corner path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathSegment {
    Line { from: Point, to: Point },
    Arc { center: Point, radius: f64, start_angle: f64, sweep_angle: f64 },
    Circle { center: Point, radius: f64 },
    Cubic(CubicBezier),
}

impl PathSegment {
    #[must_use]
    pub fn from(self) -> Point {
        match self {
            Self::Line { from, .. } => from,
            Self::Arc { center, radius, start_angle, .. } => {
                center + Point::new(start_angle.cos(), start_angle.sin()) * radius
            }
            Self::Circle { center, radius } => center + Point::new(radius, 0.0),
            Self::Cubic(bezier) => bezier.p0,
        }
    }

    #[must_use]
    pub fn to(self) -> Point {
        match self {
            Self::Line { to, .. } => to,
            Self::Arc { center, radius, start_angle, sweep_angle } => {
                let angle = start_angle + sweep_angle;
                center + Point::new(angle.cos(), angle.sin()) * radius
            }
            Self::Circle { center, radius } => center + Point::new(radius, 0.0),
            Self::Cubic(bezier) => bezier.p3,
        }
    }
}

/// An ordered closed-shape path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    segments: Vec<PathSegment>,
}

impl Path {
    pub(crate) fn circle(center: Point, radius: f64) -> Self {
        Self { segments: vec![PathSegment::Circle { center, radius }] }
    }

    #[must_use]
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }

    #[must_use]
    pub fn to_svg_path_data(&self) -> String {
        use std::fmt::Write as _;

        let Some(first) = self.segments.first() else {
            return String::new();
        };
        let start = first.from();
        let mut output = format!("M{} {}", start.x, start.y);
        for segment in &self.segments {
            match *segment {
                PathSegment::Line { to, .. } => {
                    let _ = write!(output, "L{} {}", to.x, to.y);
                }
                PathSegment::Arc { radius, sweep_angle, .. } => {
                    let to = segment.to();
                    let large_arc = i32::from(sweep_angle.abs() > PI);
                    let sweep = i32::from(sweep_angle > 0.0);
                    let _ = write!(
                        output,
                        "A{radius} {radius} 0 {large_arc} {sweep} {} {}",
                        to.x, to.y
                    );
                }
                PathSegment::Circle { center, radius } => {
                    let _ = write!(
                        output,
                        "M{} {}A{radius} {radius} 0 1 0 {} {}A{radius} {radius} 0 1 0 {} {}Z",
                        center.x + radius,
                        center.y,
                        center.x - radius,
                        center.y,
                        center.x + radius,
                        center.y
                    );
                }
                PathSegment::Cubic(bezier) => {
                    let _ = write!(
                        output,
                        "C{} {},{} {},{} {}",
                        bezier.p1.x,
                        bezier.p1.y,
                        bezier.p2.x,
                        bezier.p2.y,
                        bezier.p3.x,
                        bezier.p3.y
                    );
                }
            }
        }
        output
    }
}

pub(crate) struct PathBuilder {
    start: Point,
    current: Point,
    moved: bool,
    segments: Vec<PathSegment>,
}

impl PathBuilder {
    pub(crate) fn new() -> Self {
        Self {
            start: Point::default(),
            current: Point::default(),
            moved: false,
            segments: Vec::new(),
        }
    }

    pub(crate) fn move_to(&mut self, point: Point) {
        assert!(!self.moved, "move_to can only be called once");
        self.start = point;
        self.current = point;
        self.moved = true;
    }

    pub(crate) fn line_to(&mut self, to: Point) {
        self.segments.push(PathSegment::Line { from: self.current, to });
        self.current = to;
    }

    pub(crate) fn arc_to(
        &mut self,
        center: Point,
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
    ) {
        let segment = PathSegment::Arc { center, radius, start_angle, sweep_angle };
        self.current = segment.to();
        self.segments.push(segment);
    }

    pub(crate) fn cubic_to(&mut self, p1: Point, p2: Point, p3: Point) {
        let segment = PathSegment::Cubic(CubicBezier::new(self.current, p1, p2, p3));
        self.current = p3;
        self.segments.push(segment);
    }

    pub(crate) fn close(&mut self) {
        self.line_to(self.start);
    }

    pub(crate) fn build(self) -> Path {
        Path { segments: self.segments }
    }
}
