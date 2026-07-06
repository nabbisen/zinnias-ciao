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

#[test]
fn admin_events_new_destination_preserves_day() {
    assert_eq!(
        admin_events_new_destination("community-a", "admin_events_new:2026-07-05").as_deref(),
        Some("/c/community-a/admin/events/new?day=2026-07-05")
    );
}

#[test]
fn admin_events_new_destination_rejects_bad_dates() {
    assert_eq!(
        admin_events_new_destination("community-a", "admin_events_new:2026-07-32"),
        None
    );
    assert_eq!(
        admin_events_new_destination("community-a", "admin_events_new:2026/07/05"),
        None
    );
}

#[test]
fn admin_switch_target_requires_admin_role() {
    let memberships = vec![
        membership_db::CommunitySummary {
            community_id: "community-a".to_string(),
            community_name: "A".to_string(),
            timezone: "Asia/Tokyo".to_string(),
            role: "admin".to_string(),
        },
        membership_db::CommunitySummary {
            community_id: "community-b".to_string(),
            community_name: "B".to_string(),
            timezone: "Asia/Tokyo".to_string(),
            role: "member".to_string(),
        },
    ];

    assert!(is_admin_target(&memberships, "community-a"));
    assert!(!is_admin_target(&memberships, "community-b"));
    assert!(!is_admin_target(&memberships, "community-c"));
}
