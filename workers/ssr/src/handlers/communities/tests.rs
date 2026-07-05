use super::*;

#[test]
fn month_parse_rejects_bad_values() {
    assert_eq!(parse_month("2026-07"), Some((2026, 7)));
    assert_eq!(parse_month("2026-13"), None);
    assert_eq!(parse_month("202607"), None);
    assert_eq!(parse_month("2026/07"), None);
}

#[test]
fn ymd_parse_rejects_bad_values() {
    assert_eq!(parse_ymd("2026-07-05"), Some((2026, 7, 5)));
    assert_eq!(parse_ymd("2026-07-05x"), None);
    assert_eq!(parse_ymd("2026-07-32"), None);
    assert_eq!(parse_ymd("2026/07/05"), None);
}

#[test]
fn add_months_crosses_years() {
    assert_eq!(add_months(2026, 1, -1), (2025, 12));
    assert_eq!(add_months(2026, 12, 1), (2027, 1));
}

#[test]
fn switcher_next_preserves_month_and_day() {
    assert_eq!(switcher_next(2026, 7, None), "communities:2026-07");
    assert_eq!(
        switcher_next(2026, 7, Some("2026-07-05")),
        "communities:2026-07:2026-07-05"
    );
}
