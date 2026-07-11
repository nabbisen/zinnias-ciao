use super::*;

#[test]
fn operator_label_is_required_short_and_plain() {
    assert!(valid_operator_label("INC-1234"));
    assert!(valid_operator_label("operator@example"));
    assert!(!valid_operator_label(""));
    assert!(!valid_operator_label("   "));
    assert!(!valid_operator_label("incident\nwith-newline"));
    assert!(!valid_operator_label(
        &"a".repeat(OPERATOR_LABEL_MAX_CHARS + 1)
    ));
}
