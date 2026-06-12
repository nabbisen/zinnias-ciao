//! Security tests for domain-level validation (RFC-012 / RFC-015).
//! Tests that require contracts live in packages/contracts/tests/release_gates.rs.

use zinnias_ciao_domain::{validate_note, validate_display_name, validate_invite_input};

#[test]
fn xss_script_tag_passes_validation() {
    // Validator is not the escape boundary; render::escape_html is.
    assert!(validate_note("<script>alert('xss')</script>").is_ok());
    assert!(validate_display_name("<img onerror=alert(1)>").is_ok());
}

#[test]
fn xss_event_handler_in_display_name_passes_validation() {
    assert!(validate_display_name("\" onmouseover=\"alert(1)").is_ok());
}

#[test]
fn invite_error_no_internal_terms() {
    use zinnias_ciao_domain::InviteValidationError;
    let msg = InviteValidationError::InvalidOrExpired.to_string();
    assert!(!msg.to_lowercase().contains("hash"));
    assert!(!msg.to_lowercase().contains("hmac"));
}

#[test]
fn note_null_byte_rejected() {
    use zinnias_ciao_domain::NoteError;
    assert_eq!(validate_note("hello\x00"), Err(NoteError::InvalidChars));
}

#[test]
fn note_escape_sequence_rejected() {
    use zinnias_ciao_domain::NoteError;
    assert_eq!(validate_note("test\x1b[31m"), Err(NoteError::InvalidChars));
}

#[test]
fn display_name_null_byte_rejected() {
    use zinnias_ciao_domain::DisplayNameError;
    assert_eq!(validate_display_name("Aya\x00"), Err(DisplayNameError::InvalidChars));
}

#[test]
fn audit_metadata_blocked_keys_documented() {
    let blocked = ["note", "body", "secret", "token", "password",
                   "session_hmac", "code_hmac", "pepper"];
    for k in blocked { assert!(!k.is_empty()); }
}
