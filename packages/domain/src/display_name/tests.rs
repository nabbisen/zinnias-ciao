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
