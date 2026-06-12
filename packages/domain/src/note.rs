//! Event note validation (RFC-007).

use thiserror::Error;

pub const NOTE_MAX_CHARS: usize = 200;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NoteError {
    #[error("Your note is too long. Please keep it under 200 characters.")]
    TooLong,
    #[error("Your note contains invalid characters.")]
    InvalidChars,
}

/// Validate and normalise a note body.
/// Trims leading/trailing whitespace, rejects control characters (except
/// newline/tab), and enforces the 200-char Unicode limit.
pub fn validate_note(raw: &str) -> Result<String, NoteError> {
    let trimmed = raw.trim();
    // Reject control characters other than newline (\n) and tab (\t)
    if trimmed
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(NoteError::InvalidChars);
    }
    if trimmed.chars().count() > NOTE_MAX_CHARS {
        return Err(NoteError::TooLong);
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_short_note() {
        assert!(validate_note("I will be 10 minutes late.").is_ok());
    }

    #[test]
    fn trim_whitespace() {
        assert_eq!(
            validate_note("  hello  "),
            Ok("hello".to_string())
        );
    }

    #[test]
    fn empty_after_trim_ok() {
        // Empty string is valid — means "no note" / delete
        assert_eq!(validate_note("   "), Ok("".to_string()));
    }

    #[test]
    fn exactly_max_ok() {
        let n = "A".repeat(NOTE_MAX_CHARS);
        assert!(validate_note(&n).is_ok());
    }

    #[test]
    fn over_max_rejected() {
        let n = "A".repeat(NOTE_MAX_CHARS + 1);
        assert_eq!(validate_note(&n), Err(NoteError::TooLong));
    }

    #[test]
    fn unicode_char_count_not_bytes() {
        let n = "亜".repeat(NOTE_MAX_CHARS);
        assert!(validate_note(&n).is_ok());
        let too_long = "亜".repeat(NOTE_MAX_CHARS + 1);
        assert_eq!(validate_note(&too_long), Err(NoteError::TooLong));
    }

    #[test]
    fn control_char_rejected() {
        assert_eq!(validate_note("hello\x01"), Err(NoteError::InvalidChars));
    }

    #[test]
    fn newline_and_tab_allowed() {
        assert!(validate_note("line1\nline2").is_ok());
        assert!(validate_note("col1\tcol2").is_ok());
    }

    #[test]
    fn xss_payload_passes_through_unmodified() {
        // Validation does NOT escape — that is the renderer's job (RFC-007 §7).
        // It only rejects length and control chars.
        let xss = "<script>alert('x')</script>";
        assert!(validate_note(xss).is_ok());
    }
}
