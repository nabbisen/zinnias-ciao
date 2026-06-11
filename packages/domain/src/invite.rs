//! Invite-code domain rules (RFC-003).
//!
//! Validation logic is pure — no I/O. The Worker layer does the DB lookup and
//! HMAC check; this module owns the business rules about what makes a code valid.

use thiserror::Error;

/// Maximum length of a raw (un-normalized) invite code input.
pub const INVITE_CODE_MAX_RAW_LEN: usize = 16;

/// Length of a generated invite code (before normalization).
/// 6 upper-case alphanumeric chars (ambiguous chars excluded).
pub const INVITE_CODE_LEN: usize = 6;

/// Characters used when generating invite codes.
/// Visually ambiguous characters (0/O, 1/I/L) are excluded (RFC-003 §5).
pub const INVITE_CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InviteValidationError {
    #[error("Invalid or expired code.")]
    InvalidOrExpired,
    #[error("Please wait a little and try again.")]
    RateLimited,
}

/// Validate raw user input before any DB lookup.
/// Returns `Err` with the generic message if the format is obviously wrong
/// so we never hit the DB with garbage.
pub fn validate_invite_input(raw: &str) -> Result<(), InviteValidationError> {
    if raw.is_empty() || raw.len() > INVITE_CODE_MAX_RAW_LEN {
        return Err(InviteValidationError::InvalidOrExpired);
    }
    // After normalization (strip spaces/hyphens, uppercase) must be exactly
    // INVITE_CODE_LEN alphanumeric chars.
    let normalized: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if normalized.len() != INVITE_CODE_LEN {
        return Err(InviteValidationError::InvalidOrExpired);
    }
    if !normalized.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(InviteValidationError::InvalidOrExpired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_code_accepted() {
        assert!(validate_invite_input("X7Y9Z2").is_ok());
        assert!(validate_invite_input("x7y9z2").is_ok()); // lowercase ok pre-norm
        assert!(validate_invite_input("X7-Y9 Z2").is_ok()); // separators stripped
    }

    #[test]
    fn empty_rejected() {
        assert_eq!(
            validate_invite_input(""),
            Err(InviteValidationError::InvalidOrExpired)
        );
    }

    #[test]
    fn too_short_rejected() {
        assert_eq!(
            validate_invite_input("X7Y9"),
            Err(InviteValidationError::InvalidOrExpired)
        );
    }

    #[test]
    fn too_long_rejected() {
        assert_eq!(
            validate_invite_input("X7Y9Z2AAAAAAAAAA"),
            Err(InviteValidationError::InvalidOrExpired)
        );
    }

    #[test]
    fn special_chars_rejected() {
        assert_eq!(
            validate_invite_input("X7Y9Z!"),
            Err(InviteValidationError::InvalidOrExpired)
        );
    }
}
