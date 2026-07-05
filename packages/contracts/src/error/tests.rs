use super::*;

#[test]
fn forbidden_and_not_found_same_message() {
    // Must not leak resource existence (RFC-004 / RFC-012)
    assert_eq!(
        AppError::forbidden().user_message,
        AppError::not_found().user_message
    );
}

#[test]
fn session_expired_message_is_plain_language() {
    let msg = AppError::session_expired().user_message;
    assert!(!msg.contains("JWT"));
    assert!(!msg.contains("token"));
    assert!(!msg.contains("cookie"));
    assert!(!msg.contains("401"));
}
