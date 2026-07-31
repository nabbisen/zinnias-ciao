use super::*;

#[test]
fn calendar_flash_message_follows_locale() {
    use zinnias_ciao_contracts::Locale;

    assert_eq!(
        calendar_flash_message(Locale::Ja, Some("generated")),
        Some(i18n::JA_CALENDAR_GENERATED_FLASH)
    );
    assert_eq!(
        calendar_flash_message(Locale::En, Some("generated")),
        Some(i18n::EN_CALENDAR_GENERATED_FLASH)
    );
    assert_eq!(
        calendar_flash_message(Locale::Ja, Some("disabled")),
        Some(i18n::JA_CALENDAR_REVOKED_FLASH)
    );
    assert_eq!(
        calendar_flash_message(Locale::En, Some("disabled")),
        Some(i18n::EN_CALENDAR_REVOKED_FLASH)
    );
}

#[test]
fn calendar_flash_message_ignores_unknown_query_text() {
    use zinnias_ciao_contracts::Locale;

    assert_eq!(
        calendar_flash_message(Locale::Ja, Some("Feed URL generated")),
        None
    );
    assert_eq!(
        calendar_flash_message(Locale::Ja, Some("<script>alert(1)</script>")),
        None
    );
    assert_eq!(calendar_flash_message(Locale::Ja, None), None);
}
