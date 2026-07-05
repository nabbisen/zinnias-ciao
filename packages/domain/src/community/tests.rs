use super::*;

#[test]
fn community_name_is_trimmed() {
    assert_eq!(
        validate_community_name("  Lunch group  ").unwrap(),
        "Lunch group"
    );
}

#[test]
fn community_name_rejects_empty() {
    assert_eq!(
        validate_community_name(" \n "),
        Err(CommunityNameError::Empty)
    );
}

#[test]
fn community_name_rejects_long_values() {
    let too_long = "a".repeat(COMMUNITY_NAME_MAX + 1);
    assert_eq!(
        validate_community_name(&too_long),
        Err(CommunityNameError::TooLong)
    );
}

#[test]
fn community_name_rejects_control_chars() {
    assert_eq!(
        validate_community_name("Lunch\u{0000}group"),
        Err(CommunityNameError::InvalidCharacter)
    );
}
