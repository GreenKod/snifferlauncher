//! # `sniffer_pkg` — Hybrid Package & Component Layer
//!
//! This crate provides the **Rust-native package layer** that sits between the
//! immutable core engine (`sniffer_core` / `sniffer_render`) and the scripted
//! plugin layer (`sniffer_plugin`).
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────┐
//! │  Plugin Layer  (JS / QuickJS)        │
//! ├──────────────────────────────────────┤
//! │  Bridge  (sniffer_plugin::bridge)    │
//! ├──────────────────────────────────────┤
//! │  Package Layer  ◄── THIS CRATE       │
//! │   LauncherPackage  (services)        │
//! │   WidgetPackage    (native GPU UI)   │
//! │   PackageRegistry  (lifecycle)       │
//! ├──────────────────────────────────────┤
//! │  Core Engine  (sniffer_core/render)  │
//! └──────────────────────────────────────┘
//! ```
//!
//! ## Loading Modes
//!
//! | Mode    | Mechanism                        | Feature flag        |
//! |---------|----------------------------------|---------------------|
//! | Static  | `registry.register_service(pkg)` | `pkg_perfmon`, …    |
//! | Dynamic | `registry.load_dynamic(path)`    | `dynamic`           |
//!
//! Dynamic packages must export `pkg_create` / `pkg_destroy` with C ABI.
//! See [`loader`] for the full ABI contract and safety requirements.

pub mod error;
pub mod loader;
pub mod package;
pub mod registry;

// ---------------------------------------------------------------------------
// Optional built-in package implementations
// ---------------------------------------------------------------------------

#[cfg(feature = "pkg_perfmon")]
pub mod perf_monitor;

#[cfg(feature = "pkg_scroll")]
pub mod scroll_view;

// ---------------------------------------------------------------------------
// Top-level re-exports
// ---------------------------------------------------------------------------

pub use error::PackageError;
pub use package::{LauncherPackage, MemoryTrimLevel, PackageKind, PackageMeta, WidgetPackage};
pub use registry::PackageRegistry;
