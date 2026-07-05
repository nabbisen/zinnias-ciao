use super::*;

#[test]
fn single_placeholder() {
    assert_eq!(build_in_placeholders(1, 0), "?1");
}

#[test]
fn three_placeholders_no_offset() {
    assert_eq!(build_in_placeholders(3, 0), "?1, ?2, ?3");
}

#[test]
fn placeholders_with_offset() {
    // Used when appending a membership_id after day_ids
    assert_eq!(build_in_placeholders(3, 3), "?4, ?5, ?6");
}

#[test]
fn empty_returns_empty_string() {
    assert_eq!(build_in_placeholders(0, 0), "");
}
