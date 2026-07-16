use super::*;

#[test]
fn operator_label_matches_closed_audit_field() {
    assert!(valid_operator_label("inc-1234"));
    assert!(valid_operator_label("operator.example_1"));
    assert!(!valid_operator_label(""));
    assert!(!valid_operator_label("   "));
    assert!(!valid_operator_label("INC-1234"));
    assert!(!valid_operator_label("operator@example"));
    assert!(!valid_operator_label("incident\nwith-newline"));
    assert!(!valid_operator_label(
        &"a".repeat(OPERATOR_LABEL_MAX_BYTES + 1)
    ));
}
