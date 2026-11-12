//! # opentatu-dashboard — local web dashboard
//!
//! Server-rendered HTML (Axum + Askama), no JavaScript, one embedded stylesheet. Text shown
//! to the owner is pt-BR; code, comments and docs are English.
//!
//! - Routes are registered ONLY in [`routes::router`].
//! - Pages call `opentatu-services` (never the database) and render through
//!   [`render::render`].
//! - UI components are Askama macros in `templates/components/ui.html`; Rust-side view
//!   data (labels, CSS modifiers) lives in [`view`]. Reuse them; never restyle inline.
//! - Security defaults: loopback-only bind, `Host` allowlist (anti DNS rebinding),
//!   strict security headers, `Cache-Control: no-store` on pages (docs/SECURITY.md).

mod assets;
mod config;
mod error;
mod pages;
mod security;
mod server;

pub mod render;
pub mod routes;
pub mod view;

pub use config::DashboardConfig;
pub use error::{Error, PageError};
pub use routes::{DashboardState, router};
pub use server::serve;
