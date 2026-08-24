use crate::db::event as event_db;
use crate::db::event_series as series_db;
use zinnias_ciao_contracts::Locale;
use zinnias_ciao_contracts::i18n;
use zinnias_ciao_domain::EventValidationError;

use super::copy::{build_event_copy_prefill, render_copy_event_create_fields};
use super::create::{CreateEventProvenance, create_event_audit_metadata};
use super::forms::{
    RepeatFieldPrefill, render_details_only_event_edit_fields, render_event_create_fields,
    render_event_create_fields_with_repeat, render_recreate_event_create_fields,
};
use super::policy::{
    admin_events_new_next, event_can_seed_copy, event_can_seed_recreate, event_schedule_editable,
    valid_prefill_day, validate_event_details,
};
use super::support::query_escape;

#[test]
fn calendar_prefill_day_accepts_valid_dates_only() {
    assert!(valid_prefill_day("2026-07-05"));
    assert!(valid_prefill_day("2024-02-29"));
    assert!(!valid_prefill_day("2026-02-29"));
    assert!(!valid_prefill_day("2026-07-05x"));
    assert!(!valid_prefill_day("2026/07/05"));
}

#[test]
fn admin_events_new_next_preserves_calendar_day() {
    assert_eq!(admin_events_new_next(None), "admin_events_new");
    assert_eq!(
        admin_events_new_next(Some("2026-07-05")),
        "admin_events_new:2026-07-05"
    );
}

#[test]
fn query_escape_encodes_spaces_and_japanese() {
    assert_eq!(query_escape("Title is required."), "Title+is+required.");
    assert_eq!(
        query_escape(i18n::JA_ADMIN_EDIT_SCHEDULE_NOT_EDITABLE),
        "%E3%81%93%E3%81%AE%E3%82%A4%E3%83%99%E3%83%B3%E3%83%88%E3%81%A7%E3%81%AF%E6%97%A5%E6%99%82%E3%82%92%E5%A4%89%E6%9B%B4%E3%81%A7%E3%81%8D%E3%81%BE%E3%81%9B%E3%82%93%E3%80%82"
    );
}

fn event_row(repeat_rule: &str, repeat_count: Option<u32>) -> event_db::EventRow {
    event_db::EventRow {
        id: "evt".to_string(),
        title: "Title".to_string(),
        description: Some("Description".to_string()),
        location: Some("Room".to_string()),
        status: "scheduled".to_string(),
        repeat_rule: repeat_rule.to_string(),
        repeat_count,
    }
}

fn cancelled_event_row() -> event_db::EventRow {
    let mut event = event_row("weekly", Some(4));
    event.status = "cancelled".to_string();
    event.title = "Cancelled title".to_string();
    event.location = Some("Cancelled room".to_string());
    event.description = Some("Cancelled description".to_string());
    event
}

fn day(seq: u32, date: &str) -> event_db::EventDayRow {
    event_db::EventDayRow {
        id: format!("day-{seq}"),
        seq,
        day_date: date.to_string(),
        starts_at_utc: format!("{date}T01:00:00.000Z"),
        ends_at_utc: format!("{date}T02:00:00.000Z"),
        occurrence_status: "scheduled".to_string(),
        series_id: None,
        series_occurrence_date: None,
    }
}

fn series(
    start_day_date: &str,
    end_mode: &str,
    occurrence_count: Option<u32>,
    until_day_date: Option<&str>,
) -> series_db::EventSeriesRow {
    series_db::EventSeriesRow {
        id: "ser".to_string(),
        event_id: "evt".to_string(),
        community_id: "com".to_string(),
        frequency: "weekly".to_string(),
        start_day_date: start_day_date.to_string(),
        starts_at_local: Some("10:00".to_string()),
        ends_at_local: Some("11:00".to_string()),
        timezone: "Asia/Tokyo".to_string(),
        end_mode: end_mode.to_string(),
        occurrence_count,
        until_day_date: until_day_date.map(str::to_string),
        materialized_through_day_date: Some(start_day_date.to_string()),
    }
}

#[test]
fn schedule_editable_only_for_single_non_recurring_event() {
    assert!(event_schedule_editable(
        &event_row("none", None),
        &[day(1, "2026-07-05")]
    ));
    assert!(!event_schedule_editable(
        &event_row("none", None),
        &[day(1, "2026-07-05"), day(2, "2026-07-06")]
    ));
    assert!(!event_schedule_editable(
        &event_row("weekly", Some(1)),
        &[day(1, "2026-07-05")]
    ));
}

#[test]
fn details_only_edit_form_hides_schedule_controls() {
    let html = render_details_only_event_edit_fields(
        Locale::Ja,
        &event_row("weekly", Some(4)),
        &[day(1, "2026-07-05"), day(2, "2026-07-12")],
        "Asia/Tokyo",
        None,
    );
    assert!(html.contains(i18n::JA_ADMIN_EDIT_RECURRING_HELPER));
    assert!(html.contains(i18n::JA_ADMIN_EDIT_SCHEDULE_HEADING));
    assert!(html.contains("name=\"title\""));
    assert!(!html.contains("name=\"day_date\""));
    assert!(!html.contains("name=\"starts_at\""));
    assert!(!html.contains("name=\"ends_at\""));
    assert!(!html.contains("name=\"repeat_rule\""));
}

#[test]
fn recreate_form_prefills_details_only_and_warns_about_memos() {
    let html = render_recreate_event_create_fields(Locale::Ja, &cancelled_event_row(), None);
    assert!(html.contains("name=\"copy_source_event_id\""));
    assert!(html.contains("name=\"copy_mode\" value=\"cancelled_recreate\""));
    assert!(html.contains("Cancelled title"));
    assert!(html.contains("Cancelled room"));
    assert!(html.contains("Cancelled description"));
    assert!(html.contains(i18n::JA_ADMIN_RECREATE_EVENT_HELPER));
    assert!(html.contains("メモ"));
    assert!(html.contains("name=\"day_date\" value=\"\""));
    assert!(html.contains("name=\"starts_at\" value=\"\""));
    assert!(html.contains("name=\"ends_at\" value=\"\""));
    assert!(html.contains("<option value=\"none\" selected>"));
    assert!(!html.contains("value=\"weekly\" selected"));
    assert!(!html.contains("name=\"repeat_count\" value=\"4\""));
}

#[test]
fn recreate_source_requires_cancelled_event_but_copy_accepts_scheduled() {
    assert!(event_can_seed_recreate(&cancelled_event_row()));
    assert!(!event_can_seed_recreate(&event_row("none", None)));
    assert!(event_can_seed_copy(&cancelled_event_row()));
    assert!(event_can_seed_copy(&event_row("none", None)));
}

#[test]
fn copy_form_prefills_single_day_source_and_uses_event_copy_mode() {
    let prefill = build_event_copy_prefill(
        Locale::Ja,
        &event_row("none", None),
        &[day(1, "2026-07-05")],
        None,
        "Asia/Tokyo",
        "2026-07-01",
        "2027-01-01",
    );
    assert_eq!(prefill.day_date.as_deref(), Some("2026-07-05"));
    assert_eq!(prefill.starts_at.as_deref(), Some("10:00"));
    assert_eq!(prefill.ends_at.as_deref(), Some("11:00"));
    assert_eq!(prefill.repeat, RepeatFieldPrefill::normal_create_default());
    assert!(prefill.helpers.contains(&i18n::JA_ADMIN_COPY_EVENT_HELPER));
    assert!(
        prefill
            .helpers
            .contains(&i18n::JA_ADMIN_COPY_EVENT_DATE_WARNING)
    );

    let html = render_copy_event_create_fields(Locale::Ja, "evt", &prefill, None);
    assert!(html.contains("name=\"copy_source_event_id\" value=\"evt\""));
    assert!(html.contains("name=\"copy_mode\" value=\"event_copy\""));
    assert!(html.contains("name=\"day_date\" value=\"2026-07-05\""));
    assert!(html.contains("name=\"starts_at\" value=\"10:00\""));
    assert!(html.contains("name=\"ends_at\" value=\"11:00\""));
    assert!(html.contains("<option value=\"none\" selected>"));
}

#[test]
fn copy_prefill_for_multi_day_source_keeps_schedule_blank() {
    let prefill = build_event_copy_prefill(
        Locale::Ja,
        &event_row("none", None),
        &[day(1, "2026-07-05"), day(2, "2026-07-06")],
        None,
        "Asia/Tokyo",
        "2026-07-01",
        "2027-01-01",
    );
    assert_eq!(prefill.day_date, None);
    assert_eq!(prefill.starts_at, None);
    assert_eq!(prefill.ends_at, None);
    assert!(
        prefill
            .helpers
            .contains(&i18n::JA_ADMIN_COPY_EVENT_MULTI_DAY_HELPER)
    );
}

#[test]
fn past_recurring_copy_resets_date_and_end_controls_but_keeps_template_time() {
    let source_series = series("2026-06-01", "after_count", Some(6), None);
    let prefill = build_event_copy_prefill(
        Locale::Ja,
        &event_row("weekly", Some(6)),
        &[day(1, "2026-06-01")],
        Some(&source_series),
        "Asia/Tokyo",
        "2026-07-01",
        "2027-01-01",
    );
    assert_eq!(prefill.day_date, None);
    assert_eq!(prefill.starts_at.as_deref(), Some("10:00"));
    assert_eq!(prefill.ends_at.as_deref(), Some("11:00"));
    assert_eq!(prefill.repeat.repeat_rule, "weekly");
    assert_eq!(prefill.repeat.repeat_end_mode, "open_ended");
    assert_eq!(prefill.repeat.repeat_count, None);
    assert_eq!(prefill.repeat.repeat_until, None);
    assert!(
        prefill
            .helpers
            .contains(&i18n::JA_ADMIN_COPY_EVENT_RECURRING_PAST)
    );
}

#[test]
fn valid_recurring_copy_preserves_after_count_end_controls() {
    let source_series = series("2026-07-05", "after_count", Some(6), None);
    let prefill = build_event_copy_prefill(
        Locale::Ja,
        &event_row("weekly", Some(6)),
        &[day(1, "2026-07-05")],
        Some(&source_series),
        "Asia/Tokyo",
        "2026-07-01",
        "2027-01-01",
    );
    assert_eq!(prefill.day_date.as_deref(), Some("2026-07-05"));
    assert_eq!(prefill.repeat.repeat_rule, "weekly");
    assert_eq!(prefill.repeat.repeat_end_mode, "after_count");
    assert_eq!(prefill.repeat.repeat_count, Some(6));
    assert!(
        prefill
            .helpers
            .contains(&i18n::JA_ADMIN_COPY_EVENT_DATE_WARNING)
    );
}

#[test]
fn recurring_until_before_base_resets_only_end_controls() {
    let source_series = series("2026-07-05", "until_date", None, Some("2026-07-01"));
    let prefill = build_event_copy_prefill(
        Locale::Ja,
        &event_row("weekly", None),
        &[day(1, "2026-07-05")],
        Some(&source_series),
        "Asia/Tokyo",
        "2026-07-01",
        "2027-01-01",
    );
    assert_eq!(prefill.day_date.as_deref(), Some("2026-07-05"));
    assert_eq!(prefill.repeat.repeat_rule, "weekly");
    assert_eq!(prefill.repeat.repeat_end_mode, "open_ended");
    assert_eq!(prefill.repeat.repeat_until, None);
    assert!(
        prefill
            .helpers
            .contains(&i18n::JA_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE)
    );
}

#[test]
fn audit_metadata_separates_cancelled_recreate_from_event_copy() {
    let recreate = create_event_audit_metadata(Some(&CreateEventProvenance::CancelledRecreate(
        "evt-old".to_string(),
    )));
    assert!(matches!(
        recreate,
        crate::audit::AuditMetadata::EventCreated {
            creation_mode: crate::audit::EventCreationMode::CancelledRecreate,
            source_event_id: Some(source),
        } if source == "evt-old"
    ));

    let copied = create_event_audit_metadata(Some(&CreateEventProvenance::EventCopy(
        "evt-src".to_string(),
    )));
    assert!(matches!(
        copied,
        crate::audit::AuditMetadata::EventCreated {
            creation_mode: crate::audit::EventCreationMode::EventCopy,
            source_event_id: Some(source),
        } if source == "evt-src"
    ));
}

/// RFC-083 §6.3: a rendered-output assertion, not a source scan. Scans the
/// *whole* composed string for any codepoint in the ranges Japanese text in
/// this codebase actually uses (Hiragana, Katakana, CJK Unified Ideographs,
/// CJK punctuation, fullwidth forms) — not a check against a fixed list of
/// known constants, which would miss exactly the class of leak RFC-072
/// caught twice: a page half-migrated in a way nobody wrote a specific
/// assertion for.
fn contains_japanese_codepoint(s: &str) -> bool {
    s.chars().any(|c| {
        let cp = c as u32;
        (0x3040..=0x30FF).contains(&cp) // Hiragana + Katakana
            || (0x4E00..=0x9FFF).contains(&cp) // CJK Unified Ideographs
            || (0x3000..=0x303F).contains(&cp) // CJK punctuation (、。「」etc.)
            || (0xFF00..=0xFFEF).contains(&cp) // Fullwidth forms (？ etc.)
    })
}

#[test]
fn admin_edit_page_renders_with_no_japanese_codepoint_in_english_locale() {
    // Composes the same pieces get_edit_event's body assembles (header +
    // hint + details-only fields, which itself pulls in summary.rs's
    // schedule card) — everything but the outer <html> shell, whose
    // lang-follows-locale behavior is already covered generically by
    // render::tests::shell_lang_matches_the_locale_code_passed_in.
    let event = event_row("weekly", Some(4));
    let days = [day(1, "2026-07-05"), day(2, "2026-07-12")];

    let header = crate::render::header_with_switcher_localized(
        i18n::t(Locale::En, i18n::ADMIN_EDIT_EVENT_TITLE),
        "community-a",
        &[("community-a".to_string(), "Community A".to_string())],
        Locale::En,
    );
    let fields =
        render_details_only_event_edit_fields(Locale::En, &event, &days, "Asia/Tokyo", None);
    let en_page = format!(
        "{header}<main>{hint}{fields}</main>",
        hint = i18n::t(Locale::En, i18n::ADMIN_EDIT_EVENT_HINT)
    );

    assert!(
        !contains_japanese_codepoint(&en_page),
        "English-locale admin edit page must contain no Japanese codepoint, found some in: {en_page}"
    );

    // Sanity: the same composition at Locale::Ja must contain Japanese —
    // proves the assertion above is discriminating, not vacuously true.
    let ja_fields =
        render_details_only_event_edit_fields(Locale::Ja, &event, &days, "Asia/Tokyo", None);
    assert!(
        contains_japanese_codepoint(&ja_fields),
        "Japanese-locale render must contain Japanese text"
    );
}

/// Handoff 071 (RFC-083 F1): closes the gap `admin_create_recurring_form_...`
/// below leaves open — that test only covers the form-fields component, not
/// the template link (`ADMIN_USE_TEMPLATE_LINK`, the site this handoff
/// converted), the header, or the page title. Composes the same pieces
/// `get_create_event`'s body assembles.
#[test]
fn admin_create_page_renders_with_no_japanese_codepoint_in_english_locale() {
    let header = crate::render::header_with_switcher_next_localized(
        i18n::t(Locale::En, i18n::ADMIN_CREATE_EVENT_TITLE),
        "community-a",
        &[("community-a".to_string(), "Community A".to_string())],
        "admin_events_new",
        Locale::En,
    );
    let templates_link = format!(
        "<a href=\"/c/community-a/admin/templates\">{label}</a>",
        label = i18n::t(Locale::En, i18n::ADMIN_USE_TEMPLATE_LINK)
    );
    let fields = render_event_create_fields(
        Locale::En,
        Some("Title"),
        Some("Location"),
        None,
        None,
        Some("2026-07-05"),
        None,
        None,
    );
    let en_page = format!(
        "{header}<main><h1>{cet}</h1>{fields}{templates_link}</main>",
        cet = i18n::t(Locale::En, i18n::ADMIN_CREATE_EVENT_TITLE)
    );

    assert!(
        !contains_japanese_codepoint(&en_page),
        "English-locale admin create page must contain no Japanese codepoint, found some in: {en_page}"
    );

    // Sanity: the same composition at Locale::Ja must contain Japanese —
    // proves the assertion above is discriminating, not vacuously true.
    let ja_templates_link = format!(
        "<a href=\"/c/community-a/admin/templates\">{label}</a>",
        label = i18n::t(Locale::Ja, i18n::ADMIN_USE_TEMPLATE_LINK)
    );
    assert!(
        contains_japanese_codepoint(&ja_templates_link),
        "Japanese-locale render must contain Japanese text"
    );
}

#[test]
fn admin_create_recurring_form_renders_with_no_japanese_codepoint_in_english_locale() {
    let html = render_event_create_fields_with_repeat(
        Locale::En,
        Some("Title"),
        Some("Location"),
        Some("Description"),
        None,
        Some("2026-07-05"),
        Some("10:00"),
        Some("11:00"),
        &RepeatFieldPrefill::normal_create_default(),
    );
    assert!(
        !contains_japanese_codepoint(&html),
        "English-locale recurring create form must contain no Japanese codepoint, found some in: {html}"
    );

    let ja_html = render_event_create_fields_with_repeat(
        Locale::Ja,
        Some("Title"),
        Some("Location"),
        Some("Description"),
        None,
        Some("2026-07-05"),
        Some("10:00"),
        Some("11:00"),
        &RepeatFieldPrefill::normal_create_default(),
    );
    assert!(
        contains_japanese_codepoint(&ja_html),
        "Japanese-locale recurring create form must contain Japanese text"
    );
}

#[test]
fn validate_event_details_uses_event_text_limits_without_days() {
    let details = validate_event_details("  Title  ".into(), "  Room  ".into(), "  Body  ".into())
        .expect("details are valid");
    assert_eq!(details.title, "Title");
    assert_eq!(details.location.as_deref(), Some("Room"));
    assert_eq!(details.description.as_deref(), Some("Body"));
    assert!(matches!(
        validate_event_details(" ".into(), "".into(), "".into()),
        Err(EventValidationError::TitleEmpty)
    ));
}
