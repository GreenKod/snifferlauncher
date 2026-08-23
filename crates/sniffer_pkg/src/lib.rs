//! # `sniffer_pkg` — Hybrid Package & Component Layer Framework
//!
//! This crate provides the **Rust-native package framework** that sits between the
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
//! │   PackageDiscovery (manifest search) │
//! │   PackageLoader    (dyn loader)      │
//! ├──────────────────────────────────────┤
//! │  Core Engine  (sniffer_core/render)  │
//! └──────────────────────────────────────┘
//! ```
//!
//! ## Loading Modes
//!
//! | Mode    | Mechanism                        | Feature flag        |
//! |---------|----------------------------------|---------------------|
//! | Static  | `registry.register_service(pkg)` | (direct crate dep)  |
//! | Dynamic | `registry.load_dynamic(path)`    | `dynamic`           |
//!
//! Dynamic packages must export `pkg_create` / `pkg_destroy` or `pkg_create_widget` with C ABI.
//! See [`loader`] for the full ABI contract and safety requirements.

pub mod discovery;
pub mod error;
pub mod loader;
pub mod manifest;
pub mod package;
pub mod registry;

// ---------------------------------------------------------------------------
// Top-level re-exports
// ---------------------------------------------------------------------------

pub use discovery::PackageDiscovery;
pub use error::PackageError;
pub use manifest::{ManifestPackageKind, PackageManifest};
pub use package::{LauncherPackage, MemoryTrimLevel, PackageKind, PackageMeta, WidgetPackage};
pub use registry::PackageRegistry;
