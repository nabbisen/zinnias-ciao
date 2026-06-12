// contracts — DTOs, error types, and constants shared between
// the domain layer and the SSR worker renderer.  No Worker/WASM deps.

pub mod auth;
pub mod error;
pub mod i18n;
pub mod tz;
pub mod views;

pub use auth::{FORM_TOKEN_TTL_SECONDS, SESSION_COOKIE_NAME, SESSION_TTL_SECONDS};
pub use error::{AppError, ErrorCode};
