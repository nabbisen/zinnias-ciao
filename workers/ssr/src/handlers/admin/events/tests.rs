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
