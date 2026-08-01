use super::*;

#[test]
fn templates_flash_message_matches_known_codes() {
    assert_eq!(
        templates_flash_message(Some("title_required")),
        Some(i18n::JA_ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH)
    );
    assert_eq!(
        templates_flash_message(Some("template_saved")),
        Some(i18n::JA_ADMIN_TEMPLATE_SAVED_FLASH)
    );
    assert_eq!(
        templates_flash_message(Some("template_deleted")),
        Some(i18n::JA_ADMIN_TEMPLATE_DELETED_FLASH)
    );
}

#[test]
fn templates_flash_message_ignores_unknown_query_text() {
    assert_eq!(templates_flash_message(Some("Title required")), None);
    assert_eq!(
        templates_flash_message(Some("<script>alert(1)</script>")),
        None
    );
    assert_eq!(templates_flash_message(None), None);
}
