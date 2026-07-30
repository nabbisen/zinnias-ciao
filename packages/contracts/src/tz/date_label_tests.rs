use super::*;

#[test]
fn weekday_known_dates() {
    // 2026-06-14 is a Sunday.
    assert_eq!(weekday_index(2026, 6, 14), 0);
    // 2026-06-13 is a Saturday.
    assert_eq!(weekday_index(2026, 6, 13), 6);
    // 2000-01-01 was a Saturday.
    assert_eq!(weekday_index(2000, 1, 1), 6);
    // 2026-01-01 is a Thursday.
    assert_eq!(weekday_index(2026, 1, 1), 4);
}

#[test]
fn ja_label_has_month_day_weekday() {
    // 2026-06-13 is Saturday → 土
    assert_eq!(date_label_ja("2026-06-13"), "6月13日（土）");
    // 2026-06-14 is Sunday → 日
    assert_eq!(date_label_ja("2026-06-14"), "6月14日（日）");
}

#[test]
fn ja_label_no_english_month() {
    let label = date_label_ja("2026-06-14");
    assert!(
        !label.contains("Jun"),
        "JA label must not contain English month: {label}"
    );
    assert!(label.contains("月"), "JA label must use 月");
}

#[test]
fn en_label_format() {
    // 2026-06-14 is a Sunday; 2026-12-01 is a Tuesday.
    assert_eq!(date_label_en("2026-06-14"), "Sun, 14 Jun");
    assert_eq!(date_label_en("2026-12-01"), "Tue, 1 Dec");
}

#[test]
fn en_label_never_all_numeric() {
    // RFC-072 Slice C date-format decision: the month is always spelled or
    // abbreviated, never numeric — an all-numeric date is ambiguous between
    // month-first and day-first readers.
    let label = date_label_en("2026-08-03");
    assert!(
        !label
            .chars()
            .all(|c| c.is_ascii_digit() || c == '/' || c == '-'),
        "EN date label must not be all-numeric: {label}"
    );
    assert_eq!(label, "Mon, 3 Aug");
}

#[test]
fn weekday_en_matches_known_dates() {
    // 2026-06-14 is a Sunday.
    assert_eq!(weekday_en(weekday_index(2026, 6, 14)), "Sun");
    // 2026-06-13 is a Saturday.
    assert_eq!(weekday_en(weekday_index(2026, 6, 13)), "Sat");
}

#[test]
fn weekday_en_out_of_range_index_does_not_panic() {
    // rem_euclid handles negative/oversized indices safely, mirroring
    // weekday_ja's existing behaviour.
    assert_eq!(weekday_en(-1), "Sat");
    assert_eq!(weekday_en(7), "Sun");
}

#[test]
fn month_name_en_full_names() {
    assert_eq!(month_name_en(1), "January");
    assert_eq!(month_name_en(8), "August");
    assert_eq!(month_name_en(12), "December");
}

#[test]
fn month_name_en_out_of_range_does_not_panic() {
    assert_eq!(month_name_en(0), "");
    assert_eq!(month_name_en(13), "");
}

#[test]
fn malformed_date_falls_back() {
    assert_eq!(date_label_ja("not-a-date-x"), "not-a-date-x");
    assert_eq!(date_label_en("garbage"), "garbage");
}
