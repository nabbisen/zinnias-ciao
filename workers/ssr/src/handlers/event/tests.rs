use super::*;

#[test]
fn format_day_label_date_segment_follows_locale() {
    use zinnias_ciao_contracts::Locale;

    let ja = format_day_label(
        "2026-08-03",
        "2026-08-03T00:00:00.000Z",
        "2026-08-03T01:00:00.000Z",
        false,
        1,
        "Asia/Tokyo",
        Locale::Ja,
    );
    assert!(ja.starts_with("8月3日（月）"));

    let en = format_day_label(
        "2026-08-03",
        "2026-08-03T00:00:00.000Z",
        "2026-08-03T01:00:00.000Z",
        false,
        1,
        "Asia/Tokyo",
        Locale::En,
    );
    assert!(en.starts_with("Mon, 3 Aug"));
    assert!(!en.contains('年'));
}
