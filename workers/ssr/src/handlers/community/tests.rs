use super::*;

#[test]
fn calendar_next_destination_preserves_month_and_day() {
    assert_eq!(
        calendar_next_destination("community-a", "communities:2026-07").as_deref(),
        Some("/c/community-a/communities?month=2026-07")
    );
    assert_eq!(
        calendar_next_destination("community-a", "communities:2026-07:2026-07-05").as_deref(),
        Some("/c/community-a/communities?month=2026-07&day=2026-07-05")
    );
}

#[test]
fn calendar_next_destination_rejects_bad_dates() {
    assert_eq!(
        calendar_next_destination("community-a", "communities:2026-13"),
        None
    );
    assert_eq!(
        calendar_next_destination("community-a", "communities:2026-07:2026-08-01"),
        None
    );
    assert_eq!(
        calendar_next_destination("community-a", "communities:2026-07:2026-07-32"),
        None
    );
}
