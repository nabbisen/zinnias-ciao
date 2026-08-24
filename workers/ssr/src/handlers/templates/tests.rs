use super::*;

/// Duplicated locally per `admin/members.rs`'s own precedent (Handoff 072)
/// rather than shared/exported.
fn contains_japanese_codepoint(s: &str) -> bool {
    s.chars().any(|c| {
        let cp = c as u32;
        (0x3040..=0x30FF).contains(&cp)
            || (0x4E00..=0x9FFF).contains(&cp)
            || (0x3000..=0x303F).contains(&cp)
            || (0xFF00..=0xFFEF).contains(&cp)
    })
}

/// Handoff 074 §3.7: covers the header and nav specifically — the leak
/// class Handoff 073 exists for — plus every body label, at `Locale::En`,
/// with a `Locale::Ja` discriminating half on the header and nav.
#[test]
fn admin_templates_page_renders_with_no_japanese_codepoint_in_english_locale() {
    let header = render::header_with_switcher_next_localized(
        i18n::t(Locale::En, i18n::TEMPLATES_TITLE),
        "community-a",
        &[("community-a".to_string(), "Community A".to_string())],
        "admin_templates",
        Locale::En,
    );
    let nav = render::bottom_nav_localized("community-a", "home", Locale::En);
    let body = format!(
        "{desc}{save_section}{lbl_title}{lbl_loc}{lbl_dur}{btn_save}{use_btn}{del_btn}{empty}",
        desc = i18n::t(Locale::En, i18n::TEMPLATES_DESCRIPTION),
        save_section = i18n::t(Locale::En, i18n::TEMPLATES_SAVE_SECTION),
        lbl_title = i18n::t(Locale::En, i18n::TEMPLATES_TITLE_LABEL),
        lbl_loc = i18n::t(Locale::En, i18n::TEMPLATES_LOC_LABEL),
        lbl_dur = i18n::t(Locale::En, i18n::TEMPLATES_DUR_LABEL),
        btn_save = i18n::t(Locale::En, i18n::TEMPLATES_SAVE_BTN),
        use_btn = i18n::t(Locale::En, i18n::TEMPLATES_USE_BTN),
        del_btn = i18n::t(Locale::En, i18n::TEMPLATES_DELETE_BTN),
        empty = i18n::t(Locale::En, i18n::TEMPLATES_EMPTY),
    );
    let en_page = format!("{header}<main>{body}</main>{nav}");

    assert!(
        !contains_japanese_codepoint(&en_page),
        "English-locale admin templates page must contain no Japanese codepoint, found some in: {en_page}"
    );

    // Sanity: the same header/nav composition at Locale::Ja must contain
    // Japanese — proves the assertion above is discriminating.
    let ja_header = render::header_with_switcher_next_localized(
        i18n::t(Locale::Ja, i18n::TEMPLATES_TITLE),
        "community-a",
        &[("community-a".to_string(), "Community A".to_string())],
        "admin_templates",
        Locale::Ja,
    );
    let ja_nav = render::bottom_nav_localized("community-a", "home", Locale::Ja);
    assert!(
        contains_japanese_codepoint(&ja_header),
        "Japanese-locale header render must contain Japanese text"
    );
    assert!(
        contains_japanese_codepoint(&ja_nav),
        "Japanese-locale nav render must contain Japanese text"
    );
}

#[test]
fn templates_flash_message_matches_known_codes() {
    assert_eq!(
        templates_flash_message(Some("title_required"), Locale::Ja),
        Some(i18n::JA_ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH)
    );
    assert_eq!(
        templates_flash_message(Some("title_required"), Locale::En),
        Some(i18n::EN_ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH)
    );
    assert_eq!(
        templates_flash_message(Some("template_saved"), Locale::Ja),
        Some(i18n::JA_ADMIN_TEMPLATE_SAVED_FLASH)
    );
    assert_eq!(
        templates_flash_message(Some("template_saved"), Locale::En),
        Some(i18n::EN_ADMIN_TEMPLATE_SAVED_FLASH)
    );
    assert_eq!(
        templates_flash_message(Some("template_deleted"), Locale::Ja),
        Some(i18n::JA_ADMIN_TEMPLATE_DELETED_FLASH)
    );
    assert_eq!(
        templates_flash_message(Some("template_deleted"), Locale::En),
        Some(i18n::EN_ADMIN_TEMPLATE_DELETED_FLASH)
    );
}

#[test]
fn templates_flash_message_ignores_unknown_query_text() {
    assert_eq!(
        templates_flash_message(Some("Title required"), Locale::Ja),
        None
    );
    assert_eq!(
        templates_flash_message(Some("<script>alert(1)</script>"), Locale::Ja),
        None
    );
    assert_eq!(templates_flash_message(None, Locale::Ja), None);
}
