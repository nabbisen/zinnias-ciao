use super::*;

#[test]
fn ics_text_escapes_backslash() {
    assert_eq!(ics_text("a\\b"), "a\\\\b");
}

#[test]
fn ics_text_escapes_semicolon_and_comma() {
    assert_eq!(ics_text("a;b,c"), "a\\;b\\,c");
}

#[test]
fn ics_text_escapes_newline() {
    assert_eq!(ics_text("a\nb"), "a\\nb");
}

#[test]
fn ics_text_strips_cr() {
    assert_eq!(ics_text("a\rb"), "ab");
}

#[test]
fn utc_to_ics_dt_with_millis() {
    assert_eq!(
        utc_to_ics_dt("2026-06-14T09:00:00.000Z"),
        "20260614T090000Z"
    );
}

#[test]
fn utc_to_ics_dt_without_millis() {
    assert_eq!(utc_to_ics_dt("2026-06-14T09:00:00Z"), "20260614T090000Z");
}

#[test]
fn utc_to_ics_dt_midnight() {
    assert_eq!(utc_to_ics_dt("2026-01-01T00:00:00Z"), "20260101T000000Z");
}

#[test]
fn fold_short_line_unchanged() {
    let folded = fold_line("SUMMARY:Hello");
    assert_eq!(folded, "SUMMARY:Hello\r\n");
}

#[test]
fn fold_exact_75_unchanged() {
    let line = "A".repeat(75);
    let folded = fold_line(&line);
    assert_eq!(folded, format!("{line}\r\n"));
}

#[test]
fn fold_76_chars_breaks() {
    let line = "A".repeat(76);
    let folded = fold_line(&line);
    let lines: Vec<&str> = folded.split("\r\n").collect();
    assert_eq!(lines[0].len(), 75);
    assert!(lines[1].starts_with(' '));
}

#[test]
fn fold_long_line_continuation_starts_with_space() {
    let line = format!("SUMMARY:{}", "A".repeat(80));
    let folded = fold_line(&line);
    let second = folded.split("\r\n").nth(1).unwrap_or("");
    assert!(
        second.starts_with(' '),
        "continuation must start with space"
    );
}

#[test]
fn sanitize_filename_strips_special_chars() {
    assert_eq!(sanitize_filename("My Community!"), "MyCommunity");
    assert_eq!(sanitize_filename("zinnia-club_2026"), "zinnia-club_2026");
}

#[test]
fn sanitize_filename_truncates_at_40() {
    let long = "A".repeat(60);
    assert_eq!(sanitize_filename(&long).len(), 40);
}

#[test]
fn build_vcalendar_scheduled_event() {
    let days = vec![IcsDay {
        uid: "day1@ciao.zinnias",
        title: "Saturday Walk",
        location: Some("Station Gate"),
        status: "scheduled",
        starts_at_utc: "2026-06-14T01:00:00.000Z",
        ends_at_utc: "2026-06-14T02:30:00.000Z",
    }];
    let ics = build_vcalendar("Zinnia Club", &days);
    assert!(ics.contains("BEGIN:VCALENDAR"));
    assert!(ics.contains("BEGIN:VEVENT"));
    assert!(ics.contains("SUMMARY:Saturday Walk"));
    assert!(ics.contains("DTSTART:20260614T010000Z"));
    assert!(ics.contains("DTEND:20260614T023000Z"));
    assert!(ics.contains("LOCATION:Station Gate"));
    assert!(ics.contains("STATUS:CONFIRMED"));
    assert!(ics.contains("UID:day1@ciao.zinnias"));
    assert!(ics.contains("END:VEVENT"));
    assert!(ics.contains("END:VCALENDAR"));
}

#[test]
fn build_vcalendar_cancelled_event() {
    let days = vec![IcsDay {
        uid: "day2@ciao.zinnias",
        title: "Sunday Walk",
        location: None,
        status: "cancelled",
        starts_at_utc: "2026-06-21T01:00:00.000Z",
        ends_at_utc: "2026-06-21T02:00:00.000Z",
    }];
    let ics = build_vcalendar("Zinnia Club", &days);
    assert!(ics.contains("SUMMARY:[Cancelled] Sunday Walk"));
    assert!(ics.contains("STATUS:CANCELLED"));
    assert!(!ics.contains("LOCATION:"));
}

#[test]
fn build_vcalendar_empty() {
    let ics = build_vcalendar("Test", &[]);
    assert!(ics.contains("BEGIN:VCALENDAR"));
    assert!(ics.contains("END:VCALENDAR"));
    assert!(!ics.contains("BEGIN:VEVENT"));
}

#[test]
fn build_vcalendar_special_chars_in_title() {
    let days = vec![IcsDay {
        uid: "day3@ciao.zinnias",
        title: "Walk; Bring gear, optional",
        location: None,
        status: "scheduled",
        starts_at_utc: "2026-06-14T01:00:00Z",
        ends_at_utc: "2026-06-14T02:00:00Z",
    }];
    let ics = build_vcalendar("Test", &days);
    assert!(ics.contains("SUMMARY:Walk\\; Bring gear\\, optional"));
}
