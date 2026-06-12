// contracts — DTOs, error types, and constants shared between
// the domain layer and the SSR worker renderer.  No Worker/WASM deps.

pub mod auth;
pub mod error;
pub mod html;
pub mod i18n;
pub mod ics;
pub mod tz;
pub mod views;

pub use auth::{FORM_TOKEN_TTL_SECONDS, SESSION_COOKIE_NAME, SESSION_TTL_SECONDS};
pub use error::{AppError, ErrorCode};
pub use html::escape_html;


/// Build a comma-separated list of positional D1 placeholders for IN clauses.
/// `build_in_placeholders(3, 0)` → `"?1, ?2, ?3"`.
/// `offset` shifts the numbering: `build_in_placeholders(2, 3)` → `"?4, ?5"`.
pub fn build_in_placeholders(count: usize, offset: usize) -> String {
    (1..=count)
        .map(|i| format!("?{}", i + offset))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod placeholder_tests {
    use super::*;

    #[test]
    fn single_placeholder() {
        assert_eq!(build_in_placeholders(1, 0), "?1");
    }

    #[test]
    fn three_placeholders_no_offset() {
        assert_eq!(build_in_placeholders(3, 0), "?1, ?2, ?3");
    }

    #[test]
    fn placeholders_with_offset() {
        // Used when appending a membership_id after day_ids
        assert_eq!(build_in_placeholders(3, 3), "?4, ?5, ?6");
    }

    #[test]
    fn empty_returns_empty_string() {
        assert_eq!(build_in_placeholders(0, 0), "");
    }
}
