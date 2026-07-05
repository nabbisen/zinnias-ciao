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
mod tests;
