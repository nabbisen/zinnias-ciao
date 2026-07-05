//! Display name validation (RFC-003, requirements §7.1.4).

use thiserror::Error;

pub const DISPLAY_NAME_MIN: usize = 1;
pub const DISPLAY_NAME_MAX: usize = 40;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DisplayNameError {
    #[error("Please enter a display name.")]
    Empty,
    #[error("Display name must be 40 characters or fewer.")]
    TooLong,
    #[error("Display name contains invalid characters.")]
    InvalidChars,
}

/// Validate and normalize a display name.
/// Returns the trimmed name, or an error.
pub fn validate_display_name(raw: &str) -> Result<String, DisplayNameError> {
    let trimmed = raw.trim();

    // Empty or whitespace-only
    if trimmed.is_empty() {
        return Err(DisplayNameError::Empty);
    }

    // Control characters (excluding normal whitespace already trimmed)
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(DisplayNameError::InvalidChars);
    }

    // Unicode-aware length (char count, not byte count)
    if trimmed.chars().count() > DISPLAY_NAME_MAX {
        return Err(DisplayNameError::TooLong);
    }

    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests;
