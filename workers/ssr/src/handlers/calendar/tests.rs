use super::*;

#[test]
fn calendar_flash_message_uses_fixed_japanese_copy() {
    assert_eq!(
        calendar_flash_message(Some("generated")),
        Some(i18n::JA_CALENDAR_GENERATED_FLASH)
    );
    assert_eq!(
        calendar_flash_message(Some("disabled")),
        Some(i18n::JA_CALENDAR_REVOKED_FLASH)
    );
}

#[test]
fn calendar_flash_message_ignores_unknown_query_text() {
    assert_eq!(calendar_flash_message(Some("Feed URL generated")), None);
    assert_eq!(
        calendar_flash_message(Some("<script>alert(1)</script>")),
        None
    );
    assert_eq!(calendar_flash_message(None), None);
}
