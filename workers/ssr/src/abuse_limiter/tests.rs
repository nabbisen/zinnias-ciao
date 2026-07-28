use super::*;

fn row(policy: &str, window_started_ms: i64, count: i64) -> Row {
    Row {
        policy: policy.to_owned(),
        window_started_ms,
        count,
    }
}

// ── Content-Type media-type matching (I-N4) ─────────────────────────────────

#[test]
fn json_media_type_matches_with_or_without_parameters() {
    assert!(is_json_media_type("application/json"));
    assert!(is_json_media_type("application/json; charset=utf-8"));
    assert!(is_json_media_type(" application/json "));
    assert!(is_json_media_type("APPLICATION/JSON"));
}

#[test]
fn json_media_type_rejects_other_types() {
    assert!(!is_json_media_type("text/plain"));
    assert!(!is_json_media_type("application/json5"));
    assert!(!is_json_media_type(""));
}

// ── Policy mapping ───────────────────────────────────────────────────────────

#[test]
fn known_policies_have_fixed_limits() {
    assert_eq!(policy_limits("invite"), Some((10, 300_000)));
    assert_eq!(policy_limits("relink"), Some((10, 300_000)));
    assert_eq!(policy_limits("community"), Some((3, 86_400_000)));
}

#[test]
fn unknown_policy_has_no_limits() {
    assert_eq!(policy_limits("invite\0"), None);
    assert_eq!(policy_limits(""), None);
    assert_eq!(policy_limits("Invite"), None);
}

// ── Fixed-window boundary ────────────────────────────────────────────────────

#[test]
fn invite_attempts_one_through_ten_are_allowed_then_eleventh_is_blocked() {
    let mut existing = None;
    let now = 1_000_000_i64;
    for attempt in 1..=10 {
        let (next, outcome) = transition(existing.clone(), "invite", now).unwrap();
        assert_eq!(outcome, TransitionOutcome::Allowed, "attempt {attempt}");
        assert_eq!(next.count, attempt);
        existing = Some(next);
    }

    let (next, outcome) = transition(existing, "invite", now).unwrap();
    assert!(
        matches!(outcome, TransitionOutcome::Blocked { .. }),
        "11th attempt must be blocked"
    );
    assert_eq!(next.count, 11); // limit + 1
}

#[test]
fn community_attempts_one_through_three_are_allowed_then_fourth_is_blocked() {
    let mut existing = None;
    let now = 1_000_000_i64;
    for attempt in 1..=3 {
        let (next, outcome) = transition(existing.clone(), "community", now).unwrap();
        assert_eq!(outcome, TransitionOutcome::Allowed, "attempt {attempt}");
        existing = Some(next);
    }

    let (next, outcome) = transition(existing, "community", now).unwrap();
    assert!(matches!(outcome, TransitionOutcome::Blocked { .. }));
    assert_eq!(next.count, 4); // limit + 1
}

#[test]
fn expiry_starts_a_new_window_at_count_one() {
    let saturated = row("invite", 0, 11);
    let now_after_window = 300_000_i64; // exactly at expiry boundary
    let (next, outcome) = transition(Some(saturated), "invite", now_after_window).unwrap();
    assert_eq!(outcome, TransitionOutcome::Allowed);
    assert_eq!(next.count, 1);
    assert_eq!(next.window_started_ms, now_after_window);
}

// ── Logical expiry before policy mismatch ───────────────────────────────────

#[test]
fn unexpired_policy_mismatch_fails_closed() {
    let existing = row("relink", 0, 1);
    let result = transition(Some(existing), "invite", 1_000);
    assert_eq!(result, Err(()));
}

#[test]
fn expired_mismatched_row_starts_a_new_window_under_the_requested_policy() {
    let existing = row("relink", 0, 5);
    let (next, outcome) = transition(Some(existing), "invite", 300_000).unwrap();
    assert_eq!(outcome, TransitionOutcome::Allowed);
    assert_eq!(next.policy, "invite");
    assert_eq!(next.count, 1);
}

#[test]
fn unknown_requested_policy_fails_closed() {
    assert_eq!(transition(None, "not-a-policy", 0), Err(()));
    assert_eq!(
        transition(Some(row("invite", 0, 1)), "not-a-policy", 0),
        Err(())
    );
}

// ── Saturation ────────────────────────────────────────────────────────────

#[test]
fn sustained_blocked_burst_saturates_at_limit_plus_one_without_extending_the_window() {
    let mut existing = row("invite", 0, 11); // already saturated at limit+1
    for _ in 0..20 {
        let (next, outcome) = transition(Some(existing.clone()), "invite", 1_000).unwrap();
        assert!(matches!(outcome, TransitionOutcome::Blocked { .. }));
        assert_eq!(next.count, 11, "count must never exceed limit + 1");
        assert_eq!(
            next.window_started_ms, 0,
            "a blocked burst must not extend/restart the window"
        );
        existing = next;
    }
}

// ── Retry-after bounds ───────────────────────────────────────────────────────

#[test]
fn retry_after_seconds_is_clamped_to_one_through_window() {
    // Just entered the window: nearly the full window remains.
    let existing = row("invite", 0, 10);
    let (_, outcome) = transition(Some(existing), "invite", 1).unwrap();
    let TransitionOutcome::Blocked {
        retry_after_seconds,
    } = outcome
    else {
        panic!("expected Blocked");
    };
    assert!((1..=300).contains(&retry_after_seconds));

    // Nearly at the end of the window: retry-after must still be >= 1.
    let existing = row("invite", 0, 10);
    let (_, outcome) = transition(Some(existing), "invite", 299_999).unwrap();
    let TransitionOutcome::Blocked {
        retry_after_seconds,
    } = outcome
    else {
        panic!("expected Blocked");
    };
    assert!((1..=300).contains(&retry_after_seconds));
}

#[test]
fn community_retry_after_is_clamped_to_one_through_the_community_window() {
    let existing = row("community", 0, 3);
    let (_, outcome) = transition(Some(existing), "community", 1).unwrap();
    let TransitionOutcome::Blocked {
        retry_after_seconds,
    } = outcome
    else {
        panic!("expected Blocked");
    };
    assert!((1..=86_400).contains(&retry_after_seconds));
}

// ── Below-limit increment ───────────────────────────────────────────────────

#[test]
fn below_limit_increments_and_preserves_window_start() {
    let existing = row("relink", 500, 3);
    let (next, outcome) = transition(Some(existing), "relink", 1_000).unwrap();
    assert_eq!(outcome, TransitionOutcome::Allowed);
    assert_eq!(next.count, 4);
    assert_eq!(next.window_started_ms, 500, "window start must not move");
}
