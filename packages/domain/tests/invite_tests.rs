//! Integration tests for invite-code validation (RFC-003 §9).

use zinnias_ciao_domain::{InviteValidationError, validate_invite_input};

#[test]
fn valid_6_char_code() {
    assert!(validate_invite_input("X7Y9Z2").is_ok());
}

#[test]
fn valid_with_hyphen_separator() {
    assert!(validate_invite_input("X7Y-9Z2").is_ok()); // normalizes to 6 chars
}

#[test]
fn valid_lowercase_accepted() {
    assert!(validate_invite_input("x7y9z2").is_ok());
}

#[test]
fn too_short() {
    assert_eq!(
        validate_invite_input("X7Y9"),
        Err(InviteValidationError::InvalidOrExpired)
    );
}

#[test]
fn too_long_raw() {
    assert_eq!(
        validate_invite_input("X7Y9Z2AAAAAAAAAA"),
        Err(InviteValidationError::InvalidOrExpired)
    );
}

#[test]
fn empty_input() {
    assert_eq!(
        validate_invite_input(""),
        Err(InviteValidationError::InvalidOrExpired)
    );
}

#[test]
fn special_character() {
    assert_eq!(
        validate_invite_input("X7Y9Z!"),
        Err(InviteValidationError::InvalidOrExpired)
    );
}

#[test]
fn generic_error_message_has_no_technical_terms() {
    let msg = InviteValidationError::InvalidOrExpired.to_string();
    let lower = msg.to_lowercase();
    assert!(!lower.contains("sql"), "error leaks 'sql'");
    assert!(!lower.contains("hmac"), "error leaks 'hmac'");
    assert!(!lower.contains("hash"), "error leaks 'hash'");
}
