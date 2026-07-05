use super::*;

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
        community_id: "com".to_string(),
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
        event_id: "evt".to_string(),
        seq,
        day_date: date.to_string(),
        starts_at_utc: format!("{date}T01:00:00.000Z"),
        ends_at_utc: format!("{date}T02:00:00.000Z"),
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
    let html = render_recreate_event_create_fields(&cancelled_event_row(), None);
    assert!(html.contains("name=\"copy_source_event_id\""));
    assert!(html.contains("Cancelled title"));
    assert!(html.contains("Cancelled room"));
    assert!(html.contains("Cancelled description"));
    assert!(html.contains(i18n::JA_ADMIN_RECREATE_EVENT_HELPER));
    assert!(html.contains("メモ"));
    assert!(html.contains("name=\"day_date\" value=\"\""));
    assert!(html.contains("name=\"starts_at\" value=\"\""));
    assert!(html.contains("name=\"ends_at\" value=\"\""));
    assert!(html.contains("<option value=\"none\">"));
    assert!(!html.contains("value=\"weekly\" selected"));
    assert!(!html.contains("name=\"repeat_count\" value=\"4\""));
}

#[test]
fn recreate_source_requires_cancelled_event() {
    assert!(event_can_seed_recreate(&cancelled_event_row()));
    assert!(!event_can_seed_recreate(&event_row("none", None)));
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
