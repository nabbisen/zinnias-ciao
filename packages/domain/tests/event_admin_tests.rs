//! Event admin validation tests (RFC-009 / RFC-015 release gate).

use zinnias_ciao_domain::{DayInput, EventInput, EventValidationError, validate_event};

fn day(date: &str, start: &str, end: &str) -> DayInput {
    DayInput {
        day_date: date.into(),
        starts_at: start.into(),
        ends_at: end.into(),
    }
}
fn valid() -> EventInput {
    EventInput {
        title: "Saturday Walk".into(),
        location: Some("Station Gate".into()),
        description: None,
        days: vec![day("2026-06-14", "09:00", "10:30")],
    }
}

#[test]
fn valid_single_day_ok() {
    assert!(validate_event(valid()).is_ok());
}

#[test]
fn valid_multi_day_ok() {
    let mut i = valid();
    i.days.push(day("2026-06-15", "09:00", "11:00"));
    assert!(validate_event(i).is_ok());
}

#[test]
fn empty_title_rejected() {
    let mut i = valid();
    i.title = "".into();
    assert_eq!(validate_event(i), Err(EventValidationError::TitleEmpty));
}

#[test]
fn whitespace_only_title_rejected() {
    let mut i = valid();
    i.title = "   ".into();
    assert_eq!(validate_event(i), Err(EventValidationError::TitleEmpty));
}

#[test]
fn title_too_long() {
    let mut i = valid();
    i.title = "A".repeat(81);
    assert_eq!(validate_event(i), Err(EventValidationError::TitleTooLong));
}

#[test]
fn no_days() {
    let mut i = valid();
    i.days.clear();
    assert_eq!(validate_event(i), Err(EventValidationError::NoDays));
}

#[test]
fn day_end_before_start() {
    let i = EventInput {
        title: "T".into(),
        location: None,
        description: None,
        days: vec![day("2026-06-14", "10:00", "09:00")],
    };
    assert_eq!(
        validate_event(i),
        Err(EventValidationError::DayEndBeforeStart(1))
    );
}

#[test]
fn day_end_equals_start() {
    let i = EventInput {
        title: "T".into(),
        location: None,
        description: None,
        days: vec![day("2026-06-14", "09:00", "09:00")],
    };
    assert_eq!(
        validate_event(i),
        Err(EventValidationError::DayEndBeforeStart(1))
    );
}

#[test]
fn missing_day_date() {
    let i = EventInput {
        title: "T".into(),
        location: None,
        description: None,
        days: vec![day("", "09:00", "10:00")],
    };
    assert_eq!(
        validate_event(i),
        Err(EventValidationError::DayDateMissing(1))
    );
}

#[test]
fn missing_start_time() {
    let i = EventInput {
        title: "T".into(),
        location: None,
        description: None,
        days: vec![day("2026-06-14", "", "10:00")],
    };
    assert_eq!(
        validate_event(i),
        Err(EventValidationError::DayStartMissing(1))
    );
}

#[test]
fn second_day_invalid() {
    let mut i = valid();
    i.days.push(day("2026-06-15", "10:00", "09:00")); // bad second day
    assert_eq!(
        validate_event(i),
        Err(EventValidationError::DayEndBeforeStart(2))
    );
}

#[test]
fn location_too_long() {
    let mut i = valid();
    i.location = Some("A".repeat(121));
    assert_eq!(
        validate_event(i),
        Err(EventValidationError::LocationTooLong)
    );
}

#[test]
fn description_too_long() {
    let mut i = valid();
    i.description = Some("A".repeat(501));
    assert_eq!(
        validate_event(i),
        Err(EventValidationError::DescriptionTooLong)
    );
}

#[test]
fn error_messages_plain_language() {
    // Must not leak internal terms
    for e in [
        EventValidationError::TitleEmpty,
        EventValidationError::TitleTooLong,
        EventValidationError::NoDays,
        EventValidationError::DayEndBeforeStart(1),
    ] {
        let msg = e.to_string().to_lowercase();
        assert!(!msg.contains("sql"), "leaked sql in {msg}");
        assert!(!msg.contains("panic"), "leaked panic in {msg}");
    }
}
