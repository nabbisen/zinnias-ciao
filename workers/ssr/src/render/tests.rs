use super::event_card::CardDay;
use super::participants::initials;
use super::shell::{escape_html, shell_with_lang};
use super::status::status_display;
use super::time::format_day_time_tz_localized;

// RFC-072 Slice C: the date label inside a formatted day/time range must
// follow `locale`, not always Japanese.
#[test]
fn format_day_time_tz_localized_date_label_follows_locale() {
    use zinnias_ciao_contracts::Locale;

    let day = CardDay {
        starts_at_utc: "2026-08-03T00:00:00.000Z",
        ends_at_utc: "2026-08-03T01:00:00.000Z",
        day_date: "2026-08-03",
    };

    let ja = format_day_time_tz_localized(&day, "Asia/Tokyo", Locale::Ja);
    assert!(ja.starts_with("8月3日（月）"));

    let en = format_day_time_tz_localized(&day, "Asia/Tokyo", Locale::En);
    assert!(en.starts_with("Mon, 3 Aug"));
    assert!(
        !en.chars().take(3).all(|c| c.is_ascii_digit() || c == '/'),
        "EN date label must not be all-numeric: {en}"
    );
}

// RFC-072: `html lang` must derive from the same locale passed to the
// shell — tested directly here for the same reason `title_escaped_in_shell`
// tests `escape_html` directly: `page`/`page_localized` wrap a
// `worker::Response` and cannot be constructed in a native test
// environment.
#[test]
fn shell_lang_matches_the_locale_code_passed_in() {
    use zinnias_ciao_contracts::Locale;

    let ja = shell_with_lang(Locale::Ja.code(), "Title", "<p>Body</p>");
    assert!(ja.contains("<html lang=\"ja\">"));
    assert!(!ja.contains("<html lang=\"en\">"));

    let en = shell_with_lang(Locale::En.code(), "Title", "<p>Body</p>");
    assert!(en.contains("<html lang=\"en\">"));
    assert!(!en.contains("<html lang=\"ja\">"));
}

#[test]
fn bottom_nav_localized_labels_switch_with_locale() {
    use zinnias_ciao_contracts::{Locale, i18n};

    let ja = super::bottom_nav_localized("community-a", "me", Locale::Ja);
    assert!(ja.contains(i18n::JA_NAV_HOME));
    assert!(ja.contains(i18n::JA_NAV_ME));
    assert!(!ja.contains(i18n::EN_NAV_HOME));
    // Handoff 036: was a bare `aria-label="Main"` on every page's bottom
    // nav — a screen-reader leak, checked as the exact attribute value.
    assert!(ja.contains(&format!("aria-label=\"{}\"", i18n::JA_NAV_MAIN_ARIA_LABEL)));
    assert!(!ja.contains(&format!("aria-label=\"{}\"", i18n::EN_NAV_MAIN_ARIA_LABEL)));

    let en = super::bottom_nav_localized("community-a", "me", Locale::En);
    assert!(en.contains(i18n::EN_NAV_HOME));
    assert!(en.contains(i18n::EN_NAV_ME));
    assert!(!en.contains(i18n::JA_NAV_HOME));
    assert!(en.contains(&format!("aria-label=\"{}\"", i18n::EN_NAV_MAIN_ARIA_LABEL)));
    assert!(!en.contains(&format!("aria-label=\"{}\"", i18n::JA_NAV_MAIN_ARIA_LABEL)));
}

#[test]
fn header_with_switcher_next_localized_switch_button_follows_locale() {
    use zinnias_ciao_contracts::{Locale, i18n};

    let communities: Vec<(String, String)> = vec![("community-a".to_string(), "A".to_string())];

    // Checked as the exact button text (`>label</button>`), not a bare
    // substring: `EN_NAV_SWITCH_GO` ("Switch") is itself a substring of the
    // `aria-label` value ("Switch community"), which would false-positive a
    // bare `.contains` check on the Japanese render too. Same reasoning
    // applies to the aria-label assertions below, checked as the exact
    // attribute value.
    let ja = super::header_with_switcher_next_localized(
        "Title",
        "community-a",
        &communities,
        "me",
        Locale::Ja,
    );
    assert!(ja.contains(&format!(">{}</button>", i18n::JA_NAV_SWITCH_GO)));
    assert!(!ja.contains(&format!(">{}</button>", i18n::EN_NAV_SWITCH_GO)));
    assert!(ja.contains(&format!("aria-label='{}'", i18n::JA_NAV_SWITCH_ARIA_LABEL)));
    assert!(!ja.contains(&format!("aria-label='{}'", i18n::EN_NAV_SWITCH_ARIA_LABEL)));

    let en = super::header_with_switcher_next_localized(
        "Title",
        "community-a",
        &communities,
        "me",
        Locale::En,
    );
    assert!(en.contains(&format!(">{}</button>", i18n::EN_NAV_SWITCH_GO)));
    assert!(!en.contains(&format!(">{}</button>", i18n::JA_NAV_SWITCH_GO)));
    assert!(en.contains(&format!("aria-label='{}'", i18n::EN_NAV_SWITCH_ARIA_LABEL)));
    assert!(!en.contains(&format!("aria-label='{}'", i18n::JA_NAV_SWITCH_ARIA_LABEL)));
}

#[test]
fn escape_script_tag() {
    let out = escape_html("<script>alert(\"xss\")</script>");
    assert!(!out.contains('<') && !out.contains('>'));
    assert!(out.contains("&lt;script&gt;"));
}

#[test]
fn escape_ampersand() {
    assert_eq!(escape_html("a&b"), "a&amp;b");
}

#[test]
fn escape_clean_string() {
    assert_eq!(escape_html("hello world"), "hello world");
}

#[test]
fn title_escaped_in_shell() {
    // Verify the title is properly escaped when inserted into the page shell.
    // We test escape_html directly here because page() wraps a worker::Response
    // and cannot be constructed in a native test environment.
    let escaped = escape_html("<bad>&title");
    assert!(escaped.contains("&lt;bad&gt;"));
    assert!(escaped.contains("&amp;"));
    assert!(!escaped.contains('<'));
    assert!(!escaped.contains('>'));
}

#[test]
fn initials_two_words() {
    assert_eq!(initials("Aya Tanaka"), "AT");
}

#[test]
fn initials_one_word() {
    assert_eq!(initials("Aya"), "A");
}

#[test]
fn initials_japanese_name() {
    // Each kanji is one Unicode char; we take the first two.
    assert_eq!(initials("田中 花子"), "田花");
}

#[test]
fn status_display_going() {
    use zinnias_ciao_contracts::Locale;

    let (_, _, label) = status_display(Locale::Ja, Some("going"));
    assert!(!label.is_empty());
    assert!(
        !label.contains("Going"),
        "label must be Japanese, got: {label}"
    );
}

#[test]
fn status_display_not_going() {
    use zinnias_ciao_contracts::Locale;

    let (_, _, label) = status_display(Locale::Ja, Some("not_going"));
    assert!(!label.is_empty());
    assert!(
        !label.contains("No Go"),
        "label must be Japanese, got: {label}"
    );
}

#[test]
fn status_display_no_answer_is_default() {
    use zinnias_ciao_contracts::Locale;

    let (_, _, label_none) = status_display(Locale::Ja, None);
    let (_, _, label_unknown) = status_display(Locale::Ja, Some("unknown_value"));
    assert_eq!(
        label_none, label_unknown,
        "unknown status must use same label as None"
    );
    assert!(!label_none.is_empty());
}

// Handoff 026: the RFC-072 residue fix — status_display must follow
// `locale`, the same property already tested for nav/shell/header above.
#[test]
fn status_display_follows_locale() {
    use zinnias_ciao_contracts::{Locale, i18n};

    for status in [
        Some("going"),
        Some("not_going"),
        Some("attended"),
        None,
        Some("unknown_value"),
    ] {
        let (_, _, ja_label) = status_display(Locale::Ja, status);
        let (_, _, en_label) = status_display(Locale::En, status);
        assert_ne!(
            ja_label, en_label,
            "status {status:?} must render a different label per locale"
        );
    }

    let (_, _, en_going) = status_display(Locale::En, Some("going"));
    assert_eq!(en_going, i18n::EN_STATUS_GOING);
    let (_, _, en_not_going) = status_display(Locale::En, Some("not_going"));
    assert_eq!(en_not_going, i18n::EN_STATUS_NOT_GOING);
    let (_, _, en_attended) = status_display(Locale::En, Some("attended"));
    assert_eq!(en_attended, i18n::EN_STATUS_ATTENDED);
    let (_, _, en_no_answer) = status_display(Locale::En, None);
    assert_eq!(en_no_answer, i18n::EN_STATUS_NO_ANSWER);
}

#[test]
fn status_form_follows_locale() {
    use zinnias_ciao_contracts::{Locale, i18n};

    let ja = super::status::status_form(
        Locale::Ja,
        "community-a",
        "event-a",
        "day-a",
        "token",
        None,
        false,
        "",
    );
    assert!(ja.contains(i18n::JA_STATUS_GOING));
    assert!(ja.contains(i18n::JA_STATUS_NOT_GOING));
    assert!(ja.contains(i18n::JA_STATUS_ATTENDED));
    assert!(!ja.contains(i18n::EN_STATUS_GOING));

    let en = super::status::status_form(
        Locale::En,
        "community-a",
        "event-a",
        "day-a",
        "token",
        None,
        false,
        "",
    );
    assert!(en.contains(i18n::EN_STATUS_GOING));
    assert!(en.contains(i18n::EN_STATUS_NOT_GOING));
    assert!(en.contains(i18n::EN_STATUS_ATTENDED));
    assert!(!en.contains(i18n::JA_STATUS_GOING));

    // Clear button only renders when a status is already set.
    let ja_with_current = super::status::status_form(
        Locale::Ja,
        "community-a",
        "event-a",
        "day-a",
        "token",
        Some("going"),
        false,
        "",
    );
    assert!(ja_with_current.contains(i18n::JA_STATUS_CLEAR));
    assert!(!ja_with_current.contains(i18n::EN_STATUS_CLEAR));

    let en_with_current = super::status::status_form(
        Locale::En,
        "community-a",
        "event-a",
        "day-a",
        "token",
        Some("going"),
        false,
        "",
    );
    assert!(en_with_current.contains(i18n::EN_STATUS_CLEAR));
    assert!(!en_with_current.contains(i18n::JA_STATUS_CLEAR));
}

#[test]
fn note_form_follows_locale() {
    use zinnias_ciao_contracts::{Locale, i18n};

    let ja = super::notes::note_form(Locale::Ja, "community-a", "event-a", "token", None, None);
    assert!(ja.contains(i18n::JA_NOTE_SECTION_LABEL));
    assert!(ja.contains(i18n::JA_NOTE_SAVE));
    assert!(!ja.contains(i18n::EN_NOTE_SECTION_LABEL));

    let en = super::notes::note_form(Locale::En, "community-a", "event-a", "token", None, None);
    assert!(en.contains(i18n::EN_NOTE_SECTION_LABEL));
    assert!(en.contains(i18n::EN_NOTE_SAVE));
    assert!(!en.contains(i18n::JA_NOTE_SECTION_LABEL));

    // Delete link only renders when a note already exists.
    let ja_existing = super::notes::note_form(
        Locale::Ja,
        "community-a",
        "event-a",
        "token",
        Some("existing note"),
        None,
    );
    assert!(ja_existing.contains(i18n::JA_NOTE_DELETE));

    let en_existing = super::notes::note_form(
        Locale::En,
        "community-a",
        "event-a",
        "token",
        Some("existing note"),
        None,
    );
    assert!(en_existing.contains(i18n::EN_NOTE_DELETE));
    assert!(!en_existing.contains(i18n::JA_NOTE_DELETE));
}

#[test]
fn admin_note_hide_form_follows_locale() {
    use zinnias_ciao_contracts::{Locale, i18n};

    let ja = super::notes::admin_note_hide_form(Locale::Ja, "community-a", "event-a", "mem-a", "");
    assert!(ja.contains(i18n::JA_NOTE_DELETE));
    assert!(!ja.contains(i18n::EN_NOTE_DELETE));

    let en = super::notes::admin_note_hide_form(Locale::En, "community-a", "event-a", "mem-a", "");
    assert!(en.contains(i18n::EN_NOTE_DELETE));
    assert!(!en.contains(i18n::JA_NOTE_DELETE));
}

#[test]
fn participant_list_follows_locale() {
    use super::participants::ParticipantEntry;
    use zinnias_ciao_contracts::{Locale, i18n};

    let participants = [ParticipantEntry {
        display_name: "Taro",
        status: Some("going"),
    }];

    let ja = super::participants::participant_list(Locale::Ja, &participants);
    assert!(ja.contains(i18n::JA_STATUS_GOING));
    assert!(!ja.contains(i18n::EN_STATUS_GOING));

    let en = super::participants::participant_list(Locale::En, &participants);
    assert!(en.contains(i18n::EN_STATUS_GOING));
    assert!(!en.contains(i18n::JA_STATUS_GOING));

    // Empty-list fallback must also follow locale.
    let ja_empty = super::participants::participant_list(Locale::Ja, &[]);
    assert!(ja_empty.contains(i18n::JA_EVENT_MEMBER_FALLBACK));

    let en_empty = super::participants::participant_list(Locale::En, &[]);
    assert!(en_empty.contains(i18n::EN_EVENT_MEMBER_FALLBACK));
    assert!(!en_empty.contains(i18n::JA_EVENT_MEMBER_FALLBACK));
}

// Handoff 026 — the specific defect this package fixes: Event Detail's
// attendance buttons (rendered by `status_form`) and its cancelled-day
// badge (rendered inline in `event.rs` via `i18n::t(locale,
// i18n::OCCURRENCE_CANCELLED_BADGE)`) must render in the SAME language for
// the same member. Before this package, `status_form` had no `locale`
// parameter at all and always rendered Japanese, while the cancelled badge
// already resolved through `i18n::t` — so an English-preference member on a
// cancelled day saw an English badge above Japanese attendance buttons.
// `get_event_detail` itself cannot be unit-tested directly (async, D1-bound,
// same constraint as `rfc072_communities_and_event_pages_resolve_locale_and_html_lang_together`
// in release_gates.rs) so this proves the underlying render calls agree,
// using the exact same locale value a real request would thread through.
#[test]
fn event_detail_attendance_buttons_and_cancelled_badge_render_in_the_same_language() {
    use zinnias_ciao_contracts::{Locale, i18n};

    for locale in [Locale::Ja, Locale::En] {
        let cancelled_badge_text = i18n::t(locale, i18n::OCCURRENCE_CANCELLED_BADGE);
        let attendance_buttons = super::status::status_form(
            locale,
            "community-a",
            "event-a",
            "day-a",
            "token",
            None,
            false,
            "",
        );

        match locale {
            Locale::Ja => {
                assert_eq!(cancelled_badge_text, i18n::JA_OCCURRENCE_CANCELLED_BADGE);
                assert!(attendance_buttons.contains(i18n::JA_STATUS_GOING));
                assert!(
                    !attendance_buttons.contains(i18n::EN_STATUS_GOING),
                    "Ja-locale attendance buttons must not leak an English label \
                     alongside a Japanese cancelled badge"
                );
            }
            Locale::En => {
                assert_eq!(cancelled_badge_text, i18n::EN_OCCURRENCE_CANCELLED_BADGE);
                assert!(attendance_buttons.contains(i18n::EN_STATUS_GOING));
                assert!(
                    !attendance_buttons.contains(i18n::JA_STATUS_GOING),
                    "En-locale attendance buttons must not render Japanese \
                     alongside an English cancelled badge — this was exactly \
                     the defect Handoff 026 fixes"
                );
            }
        }
    }
}
