//! Display name validation tests (RFC-003 / requirements §7.1.4).

use zinnias_ciao_domain::{DISPLAY_NAME_MAX, DisplayNameError, validate_display_name};

#[test]
fn simple_name_ok() {
    assert_eq!(validate_display_name("Aya"), Ok("Aya".to_string()));
}

#[test]
fn leading_trailing_whitespace_trimmed() {
    assert_eq!(validate_display_name("  Ken  "), Ok("Ken".to_string()));
}

#[test]
fn empty_rejected() {
    assert_eq!(validate_display_name(""), Err(DisplayNameError::Empty));
}

#[test]
fn whitespace_only_rejected() {
    assert_eq!(validate_display_name("   "), Err(DisplayNameError::Empty));
}

#[test]
fn max_length_accepted() {
    let n = "A".repeat(DISPLAY_NAME_MAX);
    assert!(validate_display_name(&n).is_ok());
}

#[test]
fn over_max_length_rejected() {
    let n = "A".repeat(DISPLAY_NAME_MAX + 1);
    assert_eq!(validate_display_name(&n), Err(DisplayNameError::TooLong));
}

#[test]
fn unicode_char_count_not_byte_count() {
    // 40 Japanese chars = 120 bytes — must still be accepted
    let n = "亜".repeat(DISPLAY_NAME_MAX);
    assert!(validate_display_name(&n).is_ok());
    let too_long = "亜".repeat(DISPLAY_NAME_MAX + 1);
    assert_eq!(
        validate_display_name(&too_long),
        Err(DisplayNameError::TooLong)
    );
}

#[test]
fn control_character_rejected() {
    assert_eq!(
        validate_display_name("Aya\x00"),
        Err(DisplayNameError::InvalidChars)
    );
    assert_eq!(
        validate_display_name("Aya\x1b"),
        Err(DisplayNameError::InvalidChars)
    );
}
