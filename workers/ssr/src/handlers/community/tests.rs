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
fn calendar_next_destination_preserves_matrix_mode() {
    assert_eq!(
        calendar_next_destination("community-a", "communities:2026-07:matrix").as_deref(),
        Some("/c/community-a/communities?month=2026-07&view=matrix")
    );
    assert_eq!(
        calendar_next_destination("community-a", "communities:2026-07:2026-07-05:matrix")
            .as_deref(),
        Some("/c/community-a/communities?month=2026-07&day=2026-07-05&view=matrix")
    );
}

#[test]
fn calendar_next_destination_preserves_list_mode() {
    assert_eq!(
        calendar_next_destination("community-a", "communities:2026-07:list").as_deref(),
        Some("/c/community-a/communities?month=2026-07&view=list")
    );
    assert_eq!(
        calendar_next_destination("community-a", "communities:2026-07:2026-07-05:list").as_deref(),
        Some("/c/community-a/communities?month=2026-07&day=2026-07-05&view=list")
    );
    assert!(
        !calendar_next_destination("community-a", "communities:2026-07:list")
            .unwrap()
            .contains('#'),
        "the switcher must never emit a fragment"
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
fn calendar_next_destination_rejects_bad_matrix_shapes() {
    for bad in [
        "communities:2026-07:",
        "communities:2026-07:matrix:matrix",
        "communities:2026-07:2026-07-05:agenda",
        "communities:2026-07:agenda",
        "communities::matrix",
        "communities:2026-07:2026-07-05:matrix:extra",
        "communities:2026-07:list:list",
        "communities:2026-07:2026-07-05:list:extra",
    ] {
        assert_eq!(calendar_next_destination("community-a", bad), None, "{bad}");
    }
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
    let memberships = mixed_memberships();

    assert!(is_admin_target(&memberships, "community-a"));
    assert!(!is_admin_target(&memberships, "community-b"));
    assert!(!is_admin_target(&memberships, "community-c"));
}

/// A caller who is admin in `community-a` (the "source") and only a member
/// in `community-b` (the "target"). Every admin-token test below uses this
/// to prove the target-side gate does not inherit admin status from a
/// different community the same caller happens to administer — the
/// privilege-escalation shape RFC-074 exists to prevent.
fn mixed_memberships() -> Vec<membership_db::CommunitySummary> {
    vec![
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
    ]
}

#[test]
fn switch_destination_home_arm_is_explicit() {
    let memberships = mixed_memberships();
    assert_eq!(
        switch_destination("community-b", Some("home"), &memberships),
        "/c/community-b/home"
    );
}

#[test]
fn switch_destination_me_requires_only_membership() {
    let memberships = mixed_memberships();
    assert_eq!(
        switch_destination("community-b", Some("me"), &memberships),
        "/c/community-b/me"
    );
}

#[test]
fn switch_destination_calendar_feed_requires_only_membership() {
    let memberships = mixed_memberships();
    assert_eq!(
        switch_destination("community-b", Some("calendar_feed"), &memberships),
        "/c/community-b/me/calendar"
    );
}

#[test]
fn switch_destination_communities_token_family() {
    let memberships = mixed_memberships();
    assert_eq!(
        switch_destination("community-b", Some("communities"), &memberships),
        "/c/community-b/communities"
    );
    assert_eq!(
        switch_destination(
            "community-b",
            Some("communities:2026-07:2026-07-05:matrix"),
            &memberships
        ),
        "/c/community-b/communities?month=2026-07&day=2026-07-05&view=matrix"
    );
}

#[test]
fn switch_destination_admin_events_new_family() {
    let memberships = mixed_memberships();
    // Admin in the target (community-a): allowed.
    assert_eq!(
        switch_destination("community-a", Some("admin_events_new"), &memberships),
        "/c/community-a/admin/events/new"
    );
    assert_eq!(
        switch_destination(
            "community-a",
            Some("admin_events_new:2026-07-05"),
            &memberships
        ),
        "/c/community-a/admin/events/new?day=2026-07-05"
    );
    // Member-only in the target (community-b): denied, falls back to Home,
    // even though the same caller is admin in community-a — the
    // privilege-escalation shape this RFC exists to prevent.
    assert_eq!(
        switch_destination("community-b", Some("admin_events_new"), &memberships),
        "/c/community-b/home"
    );
    assert_eq!(
        switch_destination(
            "community-b",
            Some("admin_events_new:2026-07-05"),
            &memberships
        ),
        "/c/community-b/home"
    );
}

#[test]
fn switch_destination_admin_members_and_invites_require_target_admin() {
    let memberships = mixed_memberships();
    assert_eq!(
        switch_destination("community-a", Some("admin_members"), &memberships),
        "/c/community-a/admin/members"
    );
    assert_eq!(
        switch_destination("community-b", Some("admin_members"), &memberships),
        "/c/community-b/home"
    );
    assert_eq!(
        switch_destination("community-a", Some("admin_invites"), &memberships),
        "/c/community-a/admin/invites"
    );
    assert_eq!(
        switch_destination("community-b", Some("admin_invites"), &memberships),
        "/c/community-b/home"
    );
}

#[test]
fn switch_destination_admin_export_requires_target_admin() {
    let memberships = mixed_memberships();
    assert_eq!(
        switch_destination("community-a", Some("admin_export"), &memberships),
        "/c/community-a/admin/export"
    );
    assert_eq!(
        switch_destination("community-b", Some("admin_export"), &memberships),
        "/c/community-b/home",
        "admin in community-a must not reach admin_export in community-b where they are only a member"
    );
}

#[test]
fn switch_destination_admin_templates_requires_target_admin() {
    let memberships = mixed_memberships();
    assert_eq!(
        switch_destination("community-a", Some("admin_templates"), &memberships),
        "/c/community-a/admin/templates"
    );
    assert_eq!(
        switch_destination("community-b", Some("admin_templates"), &memberships),
        "/c/community-b/home",
        "admin in community-a must not reach admin_templates in community-b where they are only a member"
    );
}

#[test]
fn switch_destination_rejects_unsafe_and_unknown_next_values() {
    let memberships = mixed_memberships();
    for bad in [
        "bogus",
        "",
        "communities#fragment",
        "admin_export#fragment",
        "/admin/export",
        "https://evil.example/",
        "admin_export%2Fextra",
        "admin_export/../admin_export",
    ] {
        assert_eq!(
            switch_destination("community-b", Some(bad), &memberships),
            "/c/community-b/home",
            "next={bad:?} must fall back to target Home, never be treated as a path/URL/fragment"
        );
    }
    assert_eq!(
        switch_destination("community-b", None, &memberships),
        "/c/community-b/home"
    );
}

#[test]
fn switch_destination_malformed_calendar_token_preserves_rfc073_bare_calendar_fallback() {
    // RFC-074 non-change-scope explicitly preserves `calendar_next_destination`'s
    // existing token shapes unchanged, including its own fallback to the bare,
    // unfiltered Calendar view (not target Home) for a malformed value within
    // the `communities:` family — a discrepancy against Acceptance Criterion
    // 3's literal "falls back to Home" wording, raised explicitly in the
    // Slice 1 review request rather than resolved silently here.
    let memberships = mixed_memberships();
    for bad in [
        "communities:2026-13",
        "communities:2026-07:2026-08-01",
        "communities:2026-07:2026-07-32",
        "communities:2026-07:2026-07-05:agenda",
        "communities:2026-07:2026-07-05:matrix:extra",
    ] {
        assert_eq!(
            switch_destination("community-b", Some(bad), &memberships),
            "/c/community-b/communities",
            "next={bad:?}"
        );
    }
}

#[test]
fn switch_destination_malformed_admin_events_new_token_preserves_bare_create_event_fallback() {
    let memberships = mixed_memberships();
    assert_eq!(
        switch_destination(
            "community-a",
            Some("admin_events_new:2026-07-32"),
            &memberships
        ),
        "/c/community-a/admin/events/new",
        "malformed day within an admin-authorized admin_events_new: token preserves the pre-existing bare create-event fallback"
    );
}

#[test]
fn switch_destination_never_emits_a_fragment_for_any_accepted_token() {
    let memberships = mixed_memberships();
    let candidates = [
        Some("home"),
        Some("me"),
        Some("calendar_feed"),
        Some("communities"),
        Some("communities:2026-07:2026-07-05:matrix"),
        Some("communities:2026-07:2026-07-05:list"),
        Some("admin_events_new"),
        Some("admin_events_new:2026-07-05"),
        Some("admin_members"),
        Some("admin_invites"),
        Some("admin_export"),
        Some("admin_templates"),
        Some("bogus"),
        None,
    ];
    for next in candidates {
        let dest = switch_destination("community-a", next, &memberships);
        assert!(
            !dest.contains('#'),
            "next={next:?} produced a fragment: {dest}"
        );
    }
}
