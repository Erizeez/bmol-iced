//! Thin re-export shim for the shared Liquid Glass Iced backend.
//!
//! The reusable custom Iced `Renderer`, `Compositor`, scene builders, and
//! window-control state now live in the `bmol-glass-iced` library crate so that
//! every demo (and downstream app) renders through the same GPU glass pipeline.
//!
//! This module keeps the existing `#[path = "../iced_backend.rs"] mod iced_backend;`
//! imports in the demo binaries working unchanged.

pub use bmol_glass_iced::*;
