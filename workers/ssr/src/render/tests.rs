use super::event_card::CardDay;
use super::participants::initials;
use super::shell::{escape_html, shell_with_lang};
use super::status::status_display;
use super::time::{format_day_time_tz_localized, parse_utc_display, parse_utc_time};

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

    let en = super::bottom_nav_localized("community-a", "me", Locale::En);
    assert!(en.contains(i18n::EN_NAV_HOME));
    assert!(en.contains(i18n::EN_NAV_ME));
    assert!(!en.contains(i18n::JA_NAV_HOME));
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
fn parse_utc_time_basic() {
    assert_eq!(parse_utc_time("2026-06-14T10:30:00.000Z"), "10:30");
}

#[test]
fn parse_utc_display_uses_ja_format() {
    // Home card date display must use Japanese convention, not "Jun 14".
    let out = parse_utc_display("2026-06-14T09:00:00.000Z");
    assert!(
        !out.contains("Jun"),
        "must not contain English month: {out}"
    );
    assert!(out.contains("月"), "must contain 月: {out}");
    assert!(out.contains("日"), "must contain 日: {out}");
    assert!(out.contains("09:00"), "must contain time: {out}");
}

#[test]
fn status_display_going() {
    let (_, _, label) = status_display(Some("going"));
    assert!(!label.is_empty());
    assert!(
        !label.contains("Going"),
        "label must be Japanese, got: {label}"
    );
}

#[test]
fn status_display_not_going() {
    let (_, _, label) = status_display(Some("not_going"));
    assert!(!label.is_empty());
    assert!(
        !label.contains("No Go"),
        "label must be Japanese, got: {label}"
    );
}

#[test]
fn status_display_no_answer_is_default() {
    let (_, _, label_none) = status_display(None);
    let (_, _, label_unknown) = status_display(Some("unknown_value"));
    assert_eq!(
        label_none, label_unknown,
        "unknown status must use same label as None"
    );
    assert!(!label_none.is_empty());
}
