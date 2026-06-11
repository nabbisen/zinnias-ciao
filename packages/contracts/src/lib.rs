// contracts — DTOs, error types, and constants shared between
// the domain layer and the SSR worker renderer.  No Worker/WASM deps.

pub mod auth;
pub mod error;
pub mod views;

pub use auth::{SESSION_TTL_SECONDS, FORM_TOKEN_TTL_SECONDS, SESSION_COOKIE_NAME};
pub use error::{AppError, ErrorCode};
