use super::*;

// ── local_to_utc tests (RFC-018 write path) ───────────────────────────

#[test]
fn tokyo_local_to_utc_subtracts_nine_hours() {
    // Architect acceptance case: 09:00 Asia/Tokyo -> 00:00Z same day.
    let off = offset_minutes("Asia/Tokyo").unwrap();
    assert_eq!(
        local_to_utc("2026-06-14", "09:00", off),
        "2026-06-14T00:00:00.000Z"
    );
}

#[test]
fn tokyo_local_to_utc_wraps_to_previous_day() {
    // 06:00 JST -> 21:00Z the previous day.
    let off = offset_minutes("Asia/Tokyo").unwrap();
    assert_eq!(
        local_to_utc("2026-06-14", "06:00", off),
        "2026-06-13T21:00:00.000Z"
    );
}

#[test]
fn new_york_local_to_utc_adds_five_hours() {
    // -5h zone: 20:00 local -> 01:00Z next day.
    let off = offset_minutes("America/New_York").unwrap();
    assert_eq!(
        local_to_utc("2026-06-14", "20:00", off),
        "2026-06-15T01:00:00.000Z"
    );
}

#[test]
fn utc_local_to_utc_is_identity() {
    assert_eq!(
        local_to_utc("2026-06-14", "09:00", 0),
        "2026-06-14T09:00:00.000Z"
    );
}

#[test]
fn local_to_utc_round_trips_with_to_local_parts() {
    let off = offset_minutes("Asia/Tokyo").unwrap();
    let utc = local_to_utc("2026-06-14", "09:00", off);
    let (d, t) = to_local_parts(&utc, off);
    assert_eq!((d.as_str(), t.as_str()), ("2026-06-14", "09:00"));
}

#[test]
fn local_to_utc_month_boundary_backward() {
    // 00:30 JST on the 1st -> 15:30Z on the last day of previous month.
    let off = offset_minutes("Asia/Tokyo").unwrap();
    assert_eq!(
        local_to_utc("2026-07-01", "00:30", off),
        "2026-06-30T15:30:00.000Z"
    );
}

#[test]
fn local_to_utc_bad_input_falls_back() {
    assert_eq!(local_to_utc("bad", "09:00", 540), "badT09:00:00.000Z");
}

#[test]
fn utc_offset_is_zero() {
    assert_eq!(offset_minutes("UTC"), Some(0));
}

#[test]
fn tokyo_is_plus_nine_hours() {
    assert_eq!(offset_minutes("Asia/Tokyo"), Some(9 * 60));
}

#[test]
fn new_york_is_minus_five() {
    assert_eq!(offset_minutes("America/New_York"), Some(-5 * 60));
}

#[test]
fn kolkata_half_hour_offset() {
    assert_eq!(offset_minutes("Asia/Kolkata"), Some(5 * 60 + 30));
}

#[test]
fn unknown_tz_returns_none() {
    assert_eq!(
        offset_minutes("Atlantis/Underwater"),
        None,
        "unknown timezone must return None, not a silent UTC fallback"
    );
}

#[test]
fn offset_minutes_or_utc_falls_back_to_utc() {
    assert_eq!(
        offset_minutes_or_utc("Atlantis/Underwater"),
        0,
        "display-path helper must fall back to UTC (0) for unknown zones"
    );
}

#[test]
fn tokyo_no_day_wrap() {
    // UTC 01:00 + 9h = JST 10:00, same date
    let (d, t) = to_local_parts("2026-06-14T01:00:00.000Z", 9 * 60);
    assert_eq!(d, "2026-06-14");
    assert_eq!(t, "10:00");
}

#[test]
fn tokyo_forward_day() {
    // UTC 16:00 + 9h = JST 01:00 next day
    let (d, t) = to_local_parts("2026-06-14T16:00:00.000Z", 9 * 60);
    assert_eq!(d, "2026-06-15");
    assert_eq!(t, "01:00");
}

#[test]
fn new_york_backward_day() {
    // UTC 03:00 − 5h = Eastern 22:00 previous day
    let (d, t) = to_local_parts("2026-06-14T03:00:00.000Z", -5 * 60);
    assert_eq!(d, "2026-06-13");
    assert_eq!(t, "22:00");
}

#[test]
fn month_boundary_forward() {
    // UTC 2026-06-30 16:00 + 9h = JST 2026-07-01 01:00
    let (d, t) = to_local_parts("2026-06-30T16:00:00.000Z", 9 * 60);
    assert_eq!(d, "2026-07-01");
    assert_eq!(t, "01:00");
}

#[test]
fn month_boundary_backward() {
    // UTC 2026-07-01 02:00 − 5h = Eastern 2026-06-30 21:00
    let (d, t) = to_local_parts("2026-07-01T02:00:00.000Z", -5 * 60);
    assert_eq!(d, "2026-06-30");
    assert_eq!(t, "21:00");
}

#[test]
fn leap_day_boundary_forward() {
    // 2028 is a leap year: UTC 2028-02-28 16:00 + 9h = JST 2028-02-29 01:00
    let (d, t) = to_local_parts("2028-02-28T16:00:00.000Z", 9 * 60);
    assert_eq!(d, "2028-02-29");
    assert_eq!(t, "01:00");
}

#[test]
fn non_leap_feb_boundary() {
    // 2026 is not a leap year: UTC 2026-02-28 16:00 + 9h = JST 2026-03-01 01:00
    let (d, t) = to_local_parts("2026-02-28T16:00:00.000Z", 9 * 60);
    assert_eq!(d, "2026-03-01");
    assert_eq!(t, "01:00");
}

#[test]
fn year_boundary_forward() {
    // UTC 2026-12-31 16:00 + 9h = JST 2027-01-01 01:00
    let (d, t) = to_local_parts("2026-12-31T16:00:00.000Z", 9 * 60);
    assert_eq!(d, "2027-01-01");
    assert_eq!(t, "01:00");
}

#[test]
fn year_boundary_backward() {
    // UTC 2027-01-01 02:00 − 5h = Eastern 2026-12-31 21:00
    let (d, t) = to_local_parts("2027-01-01T02:00:00.000Z", -5 * 60);
    assert_eq!(d, "2026-12-31");
    assert_eq!(t, "21:00");
}

#[test]
fn days_in_month_leap_feb() {
    assert_eq!(days_in_month(2028, 2), 29);
    assert_eq!(days_in_month(2026, 2), 28);
    assert_eq!(days_in_month(2000, 2), 29); // divisible by 400
    assert_eq!(days_in_month(1900, 2), 28); // divisible by 100 but not 400
}

#[test]
fn exact_midnight_utc_no_change() {
    // UTC midnight stays midnight in UTC
    let (d, t) = to_local_parts("2026-06-14T00:00:00.000Z", 0);
    assert_eq!(d, "2026-06-14");
    assert_eq!(t, "00:00");
}

#[test]
fn kolkata_half_hour_arithmetic() {
    // UTC 05:30 + 5:30 = IST 11:00
    let (d, t) = to_local_parts("2026-06-14T05:30:00.000Z", 5 * 60 + 30);
    assert_eq!(d, "2026-06-14");
    assert_eq!(t, "11:00");
}
