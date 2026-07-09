use super::*;

// ── helpers ───────────────────────────────────────────────────────────

fn day(date: &str, start: &str, end: &str) -> DayInput {
    DayInput {
        day_date: date.into(),
        starts_at: start.into(),
        ends_at: end.into(),
    }
}

fn valid_input() -> EventInput {
    EventInput {
        title: "Saturday Walk".into(),
        location: Some("Station Gate".into()),
        description: None,
        days: vec![day("2026-06-14", "09:00", "10:30")],
    }
}

fn base_day() -> DayInput {
    DayInput {
        day_date: "2026-06-06".into(),
        starts_at: "09:00".into(),
        ends_at: "10:30".into(),
    }
}

// ── validate_event tests ───────────────────────────────────────────────

#[test]
fn valid_single_day() {
    assert!(validate_event(valid_input()).is_ok());
}

#[test]
fn valid_multi_day() {
    let mut inp = valid_input();
    inp.days.push(day("2026-06-15", "09:00", "10:00"));
    assert!(validate_event(inp).is_ok());
}

#[test]
fn empty_title_rejected() {
    let mut inp = valid_input();
    inp.title = "   ".into();
    assert_eq!(validate_event(inp), Err(EventValidationError::TitleEmpty));
}

#[test]
fn title_too_long() {
    let mut inp = valid_input();
    inp.title = "A".repeat(EVENT_TITLE_MAX + 1);
    assert_eq!(validate_event(inp), Err(EventValidationError::TitleTooLong));
}

#[test]
fn end_before_start_rejected() {
    let inp = EventInput {
        title: "Walk".into(),
        location: None,
        description: None,
        days: vec![day("2026-06-14", "10:00", "09:00")],
    };
    assert_eq!(
        validate_event(inp),
        Err(EventValidationError::DayEndBeforeStart(1))
    );
}

#[test]
fn end_equal_start_rejected() {
    let inp = EventInput {
        title: "Walk".into(),
        location: None,
        description: None,
        days: vec![day("2026-06-14", "09:00", "09:00")],
    };
    assert_eq!(
        validate_event(inp),
        Err(EventValidationError::DayEndBeforeStart(1))
    );
}

#[test]
fn no_days_rejected() {
    let mut inp = valid_input();
    inp.days.clear();
    assert_eq!(validate_event(inp), Err(EventValidationError::NoDays));
}

#[test]
fn missing_day_date() {
    let inp = EventInput {
        title: "Walk".into(),
        location: None,
        description: None,
        days: vec![day("", "09:00", "10:00")],
    };
    assert_eq!(
        validate_event(inp),
        Err(EventValidationError::DayDateMissing(1))
    );
}

// ── expand_recurrence tests (RFC-022) ─────────────────────────────────

#[test]
fn none_freq_returns_single_day() {
    let days = expand_recurrence(&base_day(), RecurrenceFreq::None, 4).unwrap();
    assert_eq!(days.len(), 1);
    assert_eq!(days[0].day_date, "2026-06-06");
}

#[test]
fn count_one_returns_single_day() {
    let days = expand_recurrence(&base_day(), RecurrenceFreq::Weekly, 1).unwrap();
    assert_eq!(days.len(), 1);
}

#[test]
fn weekly_four_weeks() {
    let days = expand_recurrence(&base_day(), RecurrenceFreq::Weekly, 4).unwrap();
    assert_eq!(days.len(), 4);
    assert_eq!(days[0].day_date, "2026-06-06");
    assert_eq!(days[1].day_date, "2026-06-13");
    assert_eq!(days[2].day_date, "2026-06-20");
    assert_eq!(days[3].day_date, "2026-06-27");
}

#[test]
fn biweekly_three_occurrences() {
    let days = expand_recurrence(&base_day(), RecurrenceFreq::Biweekly, 3).unwrap();
    assert_eq!(days.len(), 3);
    assert_eq!(days[0].day_date, "2026-06-06");
    assert_eq!(days[1].day_date, "2026-06-20");
    assert_eq!(days[2].day_date, "2026-07-04");
}

#[test]
fn monthly_crosses_year_boundary() {
    let base = DayInput {
        day_date: "2026-11-15".into(),
        starts_at: "10:00".into(),
        ends_at: "11:00".into(),
    };
    let days = expand_recurrence(&base, RecurrenceFreq::Monthly, 3).unwrap();
    assert_eq!(days[0].day_date, "2026-11-15");
    assert_eq!(days[1].day_date, "2026-12-15");
    assert_eq!(days[2].day_date, "2027-01-15");
}

#[test]
fn monthly_clamps_to_end_of_feb() {
    let base = DayInput {
        day_date: "2026-01-31".into(),
        starts_at: "10:00".into(),
        ends_at: "11:00".into(),
    };
    let days = expand_recurrence(&base, RecurrenceFreq::Monthly, 2).unwrap();
    assert_eq!(days[0].day_date, "2026-01-31");
    assert_eq!(days[1].day_date, "2026-02-28"); // Feb 2026 has 28 days
}

#[test]
fn count_capped_at_max() {
    let days = expand_recurrence(&base_day(), RecurrenceFreq::Weekly, 200).unwrap();
    assert_eq!(days.len(), RECURRENCE_MAX_COUNT as usize);
}

#[test]
fn times_preserved_across_occurrences() {
    let days = expand_recurrence(&base_day(), RecurrenceFreq::Weekly, 3).unwrap();
    for d in &days {
        assert_eq!(d.starts_at, "09:00");
        assert_eq!(d.ends_at, "10:30");
    }
}

#[test]
fn freq_round_trip() {
    assert_eq!(
        RecurrenceFreq::parse_form_value("weekly").as_str(),
        "weekly"
    );
    assert_eq!(
        RecurrenceFreq::parse_form_value("biweekly").as_str(),
        "biweekly"
    );
    assert_eq!(
        RecurrenceFreq::parse_form_value("monthly").as_str(),
        "monthly"
    );
    assert_eq!(RecurrenceFreq::parse_form_value("none").as_str(), "none");
    assert_eq!(RecurrenceFreq::parse_form_value("unknown").as_str(), "none");
}

#[test]
fn materialization_window_ends_six_months_ahead() {
    let window = recurrence_materialization_window("2026-07-09").unwrap();
    assert_eq!(window.from_day_date, "2026-07-09");
    assert_eq!(window.through_day_date, "2027-01-31");
}

#[test]
fn month_intersection_rejects_far_future() {
    let window = recurrence_materialization_window("2026-07-09").unwrap();
    assert!(month_intersects_materialization_window(
        "2027-01-01",
        "2027-02-01",
        &window
    ));
    assert!(!month_intersects_materialization_window(
        "2027-02-01",
        "2027-03-01",
        &window
    ));
}

#[test]
fn open_ended_occurrences_stop_at_window() {
    let days = generate_recurrence_occurrences(
        &base_day(),
        RecurrenceFreq::Weekly,
        &RecurrenceEnd::OpenEnded,
        "2026-06-27",
        &[],
    )
    .unwrap();
    assert_eq!(days.len(), 4);
    assert_eq!(days[3].ordinal, 4);
    assert_eq!(days[3].day.day_date, "2026-06-27");
}

#[test]
fn skip_exception_creates_ordinal_gap() {
    let days = generate_recurrence_occurrences(
        &base_day(),
        RecurrenceFreq::Weekly,
        &RecurrenceEnd::AfterCount(4),
        "2026-06-27",
        &[String::from("2026-06-13")],
    )
    .unwrap();
    assert_eq!(days.len(), 3);
    assert_eq!(days[0].ordinal, 1);
    assert_eq!(days[1].ordinal, 3);
    assert_eq!(days[1].day.day_date, "2026-06-20");
}

#[test]
fn generated_occurrences_are_capped_per_operation() {
    let days = generate_recurrence_occurrences(
        &base_day(),
        RecurrenceFreq::Weekly,
        &RecurrenceEnd::OpenEnded,
        "2030-01-01",
        &[],
    )
    .unwrap();
    assert_eq!(days.len(), RECURRENCE_MATERIALIZATION_INSERT_CAP);
}

#[test]
fn materialization_generation_continues_after_existing_window() {
    let days = generate_recurrence_occurrences_after(
        &base_day(),
        RecurrenceFreq::Weekly,
        &RecurrenceEnd::OpenEnded,
        Some("2027-08-21"),
        "2027-09-18",
        &[],
        RECURRENCE_MATERIALIZATION_INSERT_CAP,
    )
    .unwrap();
    assert_eq!(days[0].ordinal, 65);
    assert_eq!(days[0].day.day_date, "2027-08-28");
    assert_eq!(days[3].ordinal, 68);
    assert_eq!(days[3].day.day_date, "2027-09-18");
}

#[test]
fn materialization_generation_caps_current_run_after_existing_window() {
    let days = generate_recurrence_occurrences_after(
        &base_day(),
        RecurrenceFreq::Weekly,
        &RecurrenceEnd::OpenEnded,
        Some("2027-08-21"),
        "2027-12-31",
        &[],
        2,
    )
    .unwrap();
    assert_eq!(
        days.iter()
            .map(|occurrence| occurrence.day.day_date.as_str())
            .collect::<Vec<_>>(),
        vec!["2027-08-28", "2027-09-04"]
    );
}

#[test]
fn materialization_generation_preserves_skip_after_existing_window() {
    let days = generate_recurrence_occurrences_after(
        &base_day(),
        RecurrenceFreq::Weekly,
        &RecurrenceEnd::OpenEnded,
        Some("2027-08-21"),
        "2027-09-11",
        &[String::from("2027-08-28")],
        RECURRENCE_MATERIALIZATION_INSERT_CAP,
    )
    .unwrap();
    assert_eq!(days.len(), 2);
    assert_eq!(days[0].ordinal, 66);
    assert_eq!(days[0].day.day_date, "2027-09-04");
}

#[test]
fn validate_recurrence_end_open_ended() {
    let end = validate_recurrence_end(RecurrenceFreq::Weekly, "open_ended", None, None).unwrap();
    assert_eq!(end, Some(RecurrenceEnd::OpenEnded));
}
