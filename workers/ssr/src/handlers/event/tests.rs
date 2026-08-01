use super::*;

#[test]
fn format_day_label_date_segment_follows_locale() {
    use zinnias_ciao_contracts::Locale;

    let ja = format_day_label(
        "2026-08-03",
        "2026-08-03T00:00:00.000Z",
        "2026-08-03T01:00:00.000Z",
        false,
        1,
        "Asia/Tokyo",
        Locale::Ja,
    );
    assert!(ja.starts_with("8月3日（月）"));

    let en = format_day_label(
        "2026-08-03",
        "2026-08-03T00:00:00.000Z",
        "2026-08-03T01:00:00.000Z",
        false,
        1,
        "Asia/Tokyo",
        Locale::En,
    );
    assert!(en.starts_with("Mon, 3 Aug"));
    assert!(!en.contains('年'));
}

#[test]
fn note_flash_message_follows_locale() {
    use zinnias_ciao_contracts::Locale;

    assert_eq!(
        note_flash_message(Locale::Ja, Some("note_saved")),
        Some(i18n::JA_NOTE_SAVED_FLASH)
    );
    assert_eq!(
        note_flash_message(Locale::En, Some("note_saved")),
        Some(i18n::EN_NOTE_SAVED_FLASH)
    );
    assert_eq!(
        note_flash_message(Locale::Ja, Some("note_hidden")),
        Some(i18n::JA_NOTE_HIDDEN_FLASH)
    );
    assert_eq!(
        note_flash_message(Locale::En, Some("note_hidden")),
        Some(i18n::EN_NOTE_HIDDEN_FLASH)
    );
}

#[test]
fn note_flash_message_ignores_unknown_query_text() {
    use zinnias_ciao_contracts::Locale;

    assert_eq!(note_flash_message(Locale::Ja, Some("saved")), None);
    assert_eq!(
        note_flash_message(Locale::Ja, Some("<script>alert(1)</script>")),
        None
    );
    assert_eq!(note_flash_message(Locale::Ja, None), None);
}
