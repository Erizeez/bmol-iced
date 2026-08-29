//! Renderer-independent continuous-corner geometry.
//!
//! The G2 construction and default profiles are adapted from
//! [Kyant0/Capsule](https://github.com/Kyant0/Capsule), licensed under
//! Apache-2.0. This crate keeps the geometry independent from Iced and `wgpu`.

#![deny(unsafe_code)]

mod g2;
mod path;

pub use g2::{CapsuleAxis, CornerRadii, G2Continuity, G2Profile, ResolvedCapsule};
pub use path::{CubicBezier, Path, PathSegment, Point};
