//! Generates self-contained HTML files and ES module Web Components that run
//! compiled GAME cinematics using the WebGPU API. Zero dependencies.
//!
//! This module is split into:
//! - `html` — full-page HTML runtime (wrap_html, wrap_html_full)
//! - `component` — Web Component ES module (wrap_web_component)
//! - `arc` — arc timeline JS generation (easing, timeline data, update functions)
//! - `helpers` — shared JS helpers (param init, param update, audio setup)

mod arc;
mod component;
mod helpers;
mod html;

pub use component::wrap_web_component;
pub use html::{wrap_html, wrap_html_embed, wrap_html_full};
