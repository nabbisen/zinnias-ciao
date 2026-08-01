use super::*;

#[test]
fn attendance_flash_message_matches_known_code() {
    assert_eq!(
        attendance_flash_message(Some("attendance_saved")),
        Some(i18n::JA_ADMIN_ATTENDANCE_SAVED_FLASH)
    );
}

#[test]
fn attendance_flash_message_ignores_unknown_query_text() {
    assert_eq!(attendance_flash_message(Some("Saved")), None);
    assert_eq!(
        attendance_flash_message(Some("<script>alert(1)</script>")),
        None
    );
    assert_eq!(attendance_flash_message(None), None);
}
