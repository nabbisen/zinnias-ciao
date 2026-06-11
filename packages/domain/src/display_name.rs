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
mod tests {
    use super::*;

    #[test]
    fn valid_name() {
        assert_eq!(validate_display_name("Aya"), Ok("Aya".to_string()));
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(validate_display_name("  Aya  "), Ok("Aya".to_string()));
    }

    #[test]
    fn empty_rejected() {
        assert_eq!(validate_display_name(""), Err(DisplayNameError::Empty));
        assert_eq!(validate_display_name("   "), Err(DisplayNameError::Empty));
    }

    #[test]
    fn too_long_rejected() {
        let name = "A".repeat(41);
        assert_eq!(validate_display_name(&name), Err(DisplayNameError::TooLong));
    }

    #[test]
    fn exactly_max_len_ok() {
        let name = "A".repeat(40);
        assert!(validate_display_name(&name).is_ok());
    }

    #[test]
    fn control_char_rejected() {
        assert_eq!(
            validate_display_name("Aya\x01"),
            Err(DisplayNameError::InvalidChars)
        );
    }

    #[test]
    fn unicode_name_ok() {
        assert!(validate_display_name("田中さくら").is_ok());
    }

    #[test]
    fn unicode_length_counted_by_char() {
        // 40 CJK chars must be accepted (char count, not byte count)
        let name = "亜".repeat(40);
        assert!(validate_display_name(&name).is_ok());
        let too_long = "亜".repeat(41);
        assert_eq!(
            validate_display_name(&too_long),
            Err(DisplayNameError::TooLong)
        );
    }
}
