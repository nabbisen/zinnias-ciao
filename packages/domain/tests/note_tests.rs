//! Note validation tests (RFC-007).

use zinnias_ciao_domain::{validate_note, NOTE_MAX_CHARS};

#[test]
fn valid_note() {
    assert!(validate_note("I will be 10 minutes late.").is_ok());
}

#[test]
fn trim_whitespace() {
    assert_eq!(validate_note("  hello  "), Ok("hello".to_string()));
}

#[test]
fn empty_after_trim_is_ok() {
    assert_eq!(validate_note("   "), Ok("".to_string()));
}

#[test]
fn exactly_200_chars() {
    assert!(validate_note(&"A".repeat(NOTE_MAX_CHARS)).is_ok());
}

#[test]
fn over_200_chars_rejected() {
    use zinnias_ciao_domain::NoteError;
    assert_eq!(validate_note(&"A".repeat(NOTE_MAX_CHARS + 1)), Err(NoteError::TooLong));
}

#[test]
fn unicode_length_by_char_not_bytes() {
    assert!(validate_note(&"亜".repeat(NOTE_MAX_CHARS)).is_ok());
    use zinnias_ciao_domain::NoteError;
    assert_eq!(validate_note(&"亜".repeat(NOTE_MAX_CHARS + 1)), Err(NoteError::TooLong));
}

#[test]
fn control_char_rejected() {
    use zinnias_ciao_domain::NoteError;
    assert_eq!(validate_note("hello\x01world"), Err(NoteError::InvalidChars));
}

#[test]
fn newline_and_tab_allowed() {
    assert!(validate_note("line1\nline2").is_ok());
    assert!(validate_note("a\tb").is_ok());
}

#[test]
fn xss_payload_not_rejected_by_validator() {
    // Validation does NOT escape — the renderer must escape (RFC-007 §7).
    assert!(validate_note("<script>alert('x')</script>").is_ok());
}

#[test]
fn error_message_plain_language() {
    use zinnias_ciao_domain::NoteError;
    let msg = NoteError::TooLong.to_string();
    assert!(msg.contains("200"));
    assert!(!msg.to_lowercase().contains("error"));
    assert!(!msg.to_lowercase().contains("invalid"));
}
