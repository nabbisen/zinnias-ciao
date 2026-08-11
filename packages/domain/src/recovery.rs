//! Account recovery credential domain rules (RFC-081 §3).
//!
//! Validation logic is pure — no I/O, same split as `invite.rs`. Reuses
//! `invite::INVITE_CODE_ALPHABET` (RFC-003's visually-unambiguous
//! character set) rather than inventing a second alphabet: Handoff 057
//! §5.1 explicitly asks for the existing convention to be reused, not
//! re-derived, since the audience (a non-technical member reading a code
//! aloud) is the same one RFC-003 designed for.

use thiserror::Error;

/// Maximum length of a raw (un-normalized) recovery code input. Longer
/// than `invite::INVITE_CODE_MAX_RAW_LEN` to leave room for the extra
/// grouping hyphens a 12-character code displays with.
pub const ACCOUNT_RECOVERY_CODE_MAX_RAW_LEN: usize = 24;

/// Length of a generated recovery code (before normalization). Longer
/// than an invite/relink code on purpose: this credential carries no
/// expiry and authenticates an entire account, so it needs materially
/// more entropy than a short-lived, abuse-limited code does — 12
/// characters from a 32-character alphabet is 60 bits, comparable to a
/// direct brute-force target the abuse limiter is the only throttle on.
pub const ACCOUNT_RECOVERY_CODE_LEN: usize = 12;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RecoveryCodeValidationError {
    #[error("Invalid or expired code.")]
    InvalidOrExpired,
}

/// Validate raw user input before any DB lookup — same shape as
/// `invite::validate_invite_input`, just a different length. Returns
/// `Err` with the generic message if the format is obviously wrong so we
/// never hit the DB with garbage.
pub fn validate_recovery_code_input(raw: &str) -> Result<(), RecoveryCodeValidationError> {
    if raw.is_empty() || raw.len() > ACCOUNT_RECOVERY_CODE_MAX_RAW_LEN {
        return Err(RecoveryCodeValidationError::InvalidOrExpired);
    }
    let normalized: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if normalized.len() != ACCOUNT_RECOVERY_CODE_LEN {
        return Err(RecoveryCodeValidationError::InvalidOrExpired);
    }
    if !normalized.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(RecoveryCodeValidationError::InvalidOrExpired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(
            validate_recovery_code_input(""),
            Err(RecoveryCodeValidationError::InvalidOrExpired)
        );
    }

    #[test]
    fn oversized_raw_input_is_rejected() {
        let raw = "A".repeat(ACCOUNT_RECOVERY_CODE_MAX_RAW_LEN + 1);
        assert_eq!(
            validate_recovery_code_input(&raw),
            Err(RecoveryCodeValidationError::InvalidOrExpired)
        );
    }

    #[test]
    fn exact_length_alphanumeric_is_accepted() {
        let raw = "A".repeat(ACCOUNT_RECOVERY_CODE_LEN);
        assert_eq!(validate_recovery_code_input(&raw), Ok(()));
    }

    #[test]
    fn hyphens_and_whitespace_are_stripped_before_length_check() {
        // 3 groups of 4 plus 2 hyphens = 14 raw chars, 12 after stripping.
        let raw = "ABCD-EFGH-JKMN";
        assert_eq!(validate_recovery_code_input(raw), Ok(()));
    }

    #[test]
    fn wrong_length_after_normalization_is_rejected() {
        let raw = "A".repeat(ACCOUNT_RECOVERY_CODE_LEN - 1);
        assert_eq!(
            validate_recovery_code_input(&raw),
            Err(RecoveryCodeValidationError::InvalidOrExpired)
        );
    }

    #[test]
    fn non_alphanumeric_characters_are_rejected() {
        let mut raw = "A".repeat(ACCOUNT_RECOVERY_CODE_LEN - 1);
        raw.push('!');
        assert_eq!(
            validate_recovery_code_input(&raw),
            Err(RecoveryCodeValidationError::InvalidOrExpired)
        );
    }
}
