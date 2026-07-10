//! UI string constants (i18n scaffold — RFC-026).
//!
//! All user-visible strings are collected here so they can be translated
//! without touching handler logic.  Currently English only; Japanese parity
//! is enforced by the i18n lint test below.
//!
//! Naming: `<LANG>_<CONTEXT>_<KEY>` in SCREAMING_SNAKE_CASE.

mod access;
mod admin;
mod calendar;
mod community;
mod events;
mod export;
mod general;
mod home;
mod me;
mod notes;
mod templates;

pub use access::*;
pub use admin::*;
pub use calendar::*;
pub use community::*;
pub use events::*;
pub use export::*;
pub use general::*;
pub use home::*;
pub use me::*;
pub use notes::*;
pub use templates::*;

#[cfg(test)]
mod tests;
