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
    assert_eq!(date_label_en("2026-06-14"), "14 Jun");
    assert_eq!(date_label_en("2026-12-01"), "1 Dec");
}

#[test]
fn malformed_date_falls_back() {
    assert_eq!(date_label_ja("not-a-date-x"), "not-a-date-x");
    assert_eq!(date_label_en("garbage"), "garbage");
}
