use super::*;

#[test]
fn attendance_flash_message_matches_known_code() {
    assert_eq!(
        attendance_flash_message(Locale::Ja, Some("attendance_saved")),
        Some(i18n::JA_ADMIN_ATTENDANCE_SAVED_FLASH)
    );
    assert_eq!(
        attendance_flash_message(Locale::En, Some("attendance_saved")),
        Some(i18n::EN_ADMIN_ATTENDANCE_SAVED_FLASH)
    );
}

#[test]
fn attendance_flash_message_ignores_unknown_query_text() {
    assert_eq!(attendance_flash_message(Locale::Ja, Some("Saved")), None);
    assert_eq!(
        attendance_flash_message(Locale::Ja, Some("<script>alert(1)</script>")),
        None
    );
    assert_eq!(attendance_flash_message(Locale::Ja, None), None);
}
