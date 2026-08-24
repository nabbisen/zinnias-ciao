use super::*;
use zinnias_ciao_contracts::Locale;

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
/// class Handoff 073 exists for — plus every body label and the
/// named-substitution summary line, at `Locale::En`, with a `Locale::Ja`
/// discriminating half on the header and nav.
#[test]
fn admin_export_page_renders_with_no_japanese_codepoint_in_english_locale() {
    let header = render::header_with_switcher_next_localized(
        i18n::t(Locale::En, i18n::EXPORT_TITLE),
        "community-a",
        &[("community-a".to_string(), "Community A".to_string())],
        "admin_export",
        Locale::En,
    );
    let nav = render::bottom_nav_localized("community-a", "home", Locale::En);
    let summary_counts = i18n::t(Locale::En, i18n::ADMIN_EXPORT_SUMMARY_COUNTS)
        .replace("{events}", "5")
        .replace("{members}", "3");
    let body = format!(
        "{exp_desc}{privacy_note}{download_btn}{single_use}{summary_counts}",
        exp_desc = i18n::t(Locale::En, i18n::EXPORT_DESCRIPTION),
        privacy_note = i18n::t(Locale::En, i18n::EXPORT_PRIVACY_NOTE),
        download_btn = i18n::t(Locale::En, i18n::EXPORT_DOWNLOAD_BTN),
        single_use = i18n::t(Locale::En, i18n::EXPORT_SINGLE_USE),
        summary_counts = summary_counts,
    );
    let en_page = format!("{header}<main>{body}</main>{nav}");

    assert!(
        !contains_japanese_codepoint(&en_page),
        "English-locale admin export page must contain no Japanese codepoint, found some in: {en_page}"
    );

    // Sanity: the same header/nav composition at Locale::Ja must contain
    // Japanese — proves the assertion above is discriminating.
    let ja_header = render::header_with_switcher_next_localized(
        i18n::t(Locale::Ja, i18n::EXPORT_TITLE),
        "community-a",
        &[("community-a".to_string(), "Community A".to_string())],
        "admin_export",
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

/// Handoff 074 §3.7: named substitution (not positional) means a dropped
/// or misspelled placeholder in the English half silently survives into
/// rendered output as a literal `{events}`/`{members}` instead of a
/// number — this test is the guard.
#[test]
fn admin_export_summary_counts_substitutes_both_placeholders_in_english_locale() {
    let summary_counts = i18n::t(Locale::En, i18n::ADMIN_EXPORT_SUMMARY_COUNTS)
        .replace("{events}", "5")
        .replace("{members}", "3");
    assert!(
        !summary_counts.contains("{events}"),
        "English export summary must not contain a literal {{events}} placeholder: {summary_counts}"
    );
    assert!(
        !summary_counts.contains("{members}"),
        "English export summary must not contain a literal {{members}} placeholder: {summary_counts}"
    );
    assert!(summary_counts.contains('5') && summary_counts.contains('3'));
}
