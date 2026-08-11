use super::*;

// ── Ingress header-shape classification (H-B1) ──────────────────────────────

#[test]
fn accepts_a_single_valid_ipv4_address() {
    assert_eq!(
        classify_ingress(None, None, Some("203.0.113.7")),
        Ok("203.0.113.7".to_string())
    );
}

#[test]
fn rejects_when_cf_worker_header_present() {
    assert_eq!(
        classify_ingress(Some("anything"), None, Some("203.0.113.7")),
        Err(IngressRejection::UpstreamWorker)
    );
}

#[test]
fn rejects_when_cf_connecting_ipv6_header_present() {
    assert_eq!(
        classify_ingress(None, Some("2001:db8::1"), Some("203.0.113.7")),
        Err(IngressRejection::Ipv6HeaderPresent)
    );
}

#[test]
fn rejects_missing_cf_connecting_ip() {
    assert_eq!(
        classify_ingress(None, None, None),
        Err(IngressRejection::InvalidAddress)
    );
}

#[test]
fn rejects_empty_cf_connecting_ip() {
    assert_eq!(
        classify_ingress(None, None, Some("")),
        Err(IngressRejection::InvalidAddress)
    );
}

#[test]
fn rejects_comma_joined_repeated_header_value() {
    // The Fetch standard combines a genuinely repeated header into one
    // comma-joined value before this code ever sees it (H-B1) — a single
    // `Headers::get` plus this rule is sufficient to detect multiplicity.
    assert_eq!(
        classify_ingress(None, None, Some("203.0.113.7, 198.51.100.9")),
        Err(IngressRejection::InvalidAddress)
    );
}

#[test]
fn rejects_whitespace_containing_value() {
    assert_eq!(
        classify_ingress(None, None, Some("203.0.113.7 198.51.100.9")),
        Err(IngressRejection::InvalidAddress)
    );
    assert_eq!(
        classify_ingress(None, None, Some("203.0.113.7\t")),
        Err(IngressRejection::InvalidAddress)
    );
}

#[test]
fn rejects_malformed_address() {
    assert_eq!(
        classify_ingress(None, None, Some("not-an-ip")),
        Err(IngressRejection::InvalidAddress)
    );
}

#[test]
fn rejects_zone_qualified_address() {
    // std's IpAddr parser does not accept a `%zone` scope-id suffix, so a
    // zone-qualified literal fails to parse and is rejected as malformed.
    assert_eq!(
        classify_ingress(None, None, Some("fe80::1%eth0")),
        Err(IngressRejection::InvalidAddress)
    );
}

#[test]
fn rejects_raw_ipv4_class_e_address() {
    assert_eq!(
        classify_ingress(None, None, Some("240.1.2.3")),
        Err(IngressRejection::ClassEAddress)
    );
    assert_eq!(
        classify_ingress(None, None, Some("255.255.255.255")),
        Err(IngressRejection::ClassEAddress)
    );
}

#[test]
fn accepts_ipv4_just_below_class_e() {
    assert_eq!(
        classify_ingress(None, None, Some("239.255.255.255")),
        Ok("239.255.255.255".to_string())
    );
}

#[test]
fn normalizes_ipv4_mapped_ipv6_to_ipv4() {
    assert_eq!(
        classify_ingress(None, None, Some("::ffff:203.0.113.5")),
        Ok("203.0.113.5".to_string())
    );
}

#[test]
fn rejects_ipv4_mapped_ipv6_class_e_address() {
    // H-B1 dedicated regression: normalize-then-reject order must not let a
    // Class E address slip through disguised as an IPv6 literal.
    assert_eq!(
        classify_ingress(None, None, Some("::ffff:240.0.0.1")),
        Err(IngressRejection::ClassEAddress)
    );
}

#[test]
fn canonicalizes_native_ipv6_to_first_64_bits() {
    let result = classify_ingress(None, None, Some("2001:db8:1234:5678:aaaa:bbbb:cccc:dddd"));
    assert_eq!(result, Ok("2001:db8:1234:5678::".to_string()));
}

#[test]
fn same_64_prefix_yields_the_same_canonical_subject() {
    let a = classify_ingress(None, None, Some("2001:db8:1234:5678:1111:2222:3333:4444"));
    let b = classify_ingress(None, None, Some("2001:db8:1234:5678:9999:8888:7777:6666"));
    assert_eq!(a, b);
}

// ── HMAC domain separation ──────────────────────────────────────────────────

#[test]
fn subject_digest_differs_across_all_six_scopes() {
    let scopes = [
        Scope::Invite,
        Scope::Relink,
        Scope::CommunityUser,
        Scope::CommunitySession,
        Scope::CommunityNetwork,
        Scope::Recovery,
    ];
    let mut digests = Vec::new();
    for scope in scopes {
        digests.push(subject_digest("pepper", scope, "same-subject"));
    }
    for i in 0..digests.len() {
        for j in (i + 1)..digests.len() {
            assert_ne!(
                digests[i], digests[j],
                "scopes must produce distinct digests"
            );
        }
    }
}

#[test]
fn subject_digest_differs_across_peppers() {
    assert_ne!(
        subject_digest("pepper-a", Scope::Invite, "subject"),
        subject_digest("pepper-b", Scope::Invite, "subject")
    );
}

#[test]
fn subject_digest_differs_across_subjects() {
    assert_ne!(
        subject_digest("pepper", Scope::Invite, "203.0.113.1"),
        subject_digest("pepper", Scope::Invite, "203.0.113.2")
    );
}

#[test]
fn object_name_contains_scope_label_and_digest_but_never_the_raw_subject() {
    let digest = subject_digest("pepper", Scope::CommunityNetwork, "203.0.113.9");
    let name = object_name(Scope::CommunityNetwork, &digest);
    assert_eq!(name, format!("v1:community-network:{digest}"));
    assert!(!name.contains("203.0.113.9"));
}

// ── Reserve-response parsing (strict; malformed never maps to Allowed) ─────

#[test]
fn parses_allowed_response() {
    assert_eq!(
        parse_reserve_outcome(
            200,
            r#"{"outcome":"allowed","retry_after_seconds":0}"#,
            Scope::Invite
        ),
        Ok(Outcome::Allowed)
    );
}

#[test]
fn parses_blocked_response_within_window() {
    assert_eq!(
        parse_reserve_outcome(
            200,
            r#"{"outcome":"blocked","retry_after_seconds":123}"#,
            Scope::Invite
        ),
        Ok(Outcome::Blocked {
            retry_after_seconds: 123
        })
    );
}

#[test]
fn rejects_wrong_status_code() {
    assert_eq!(
        parse_reserve_outcome(
            400,
            r#"{"outcome":"allowed","retry_after_seconds":0}"#,
            Scope::Invite
        ),
        Err(UnavailableCategory::MalformedResponse)
    );
}

#[test]
fn maps_5xx_status_to_coordinator_error_not_malformed_response() {
    // I-N1: a genuine Durable Object failure (its own `Response::error(..,
    // 500)`) must be distinguishable from a parse/shape failure so incident
    // triage is not misdirected.
    for status in [500, 503, 599] {
        assert_eq!(
            parse_reserve_outcome(status, "", Scope::Invite),
            Err(UnavailableCategory::CoordinatorError),
            "status {status} must map to CoordinatorError"
        );
    }
}

#[test]
fn rejects_allowed_with_nonzero_retry_after() {
    // I-N2: the protocol defines `allowed => retry_after_seconds: 0`; a
    // hybrid/corrupted coordinator response must not be read as Allowed.
    assert_eq!(
        parse_reserve_outcome(
            200,
            r#"{"outcome":"allowed","retry_after_seconds":5}"#,
            Scope::Invite
        ),
        Err(UnavailableCategory::MalformedResponse)
    );
}

#[test]
fn rejects_empty_body() {
    assert_eq!(
        parse_reserve_outcome(200, "", Scope::Invite),
        Err(UnavailableCategory::MalformedResponse)
    );
}

#[test]
fn rejects_oversized_body() {
    let oversized = "x".repeat(PROTOCOL_MAX_BODY_BYTES + 1);
    assert_eq!(
        parse_reserve_outcome(200, &oversized, Scope::Invite),
        Err(UnavailableCategory::MalformedResponse)
    );
}

#[test]
fn rejects_unknown_fields() {
    assert_eq!(
        parse_reserve_outcome(
            200,
            r#"{"outcome":"allowed","retry_after_seconds":0,"extra":"field"}"#,
            Scope::Invite
        ),
        Err(UnavailableCategory::MalformedResponse)
    );
}

#[test]
fn rejects_unknown_outcome_value() {
    assert_eq!(
        parse_reserve_outcome(
            200,
            r#"{"outcome":"maybe","retry_after_seconds":0}"#,
            Scope::Invite
        ),
        Err(UnavailableCategory::MalformedResponse)
    );
}

#[test]
fn rejects_malformed_json() {
    assert_eq!(
        parse_reserve_outcome(200, "not json", Scope::Invite),
        Err(UnavailableCategory::MalformedResponse)
    );
}

#[test]
fn rejects_blocked_with_zero_retry_after() {
    assert_eq!(
        parse_reserve_outcome(
            200,
            r#"{"outcome":"blocked","retry_after_seconds":0}"#,
            Scope::Invite
        ),
        Err(UnavailableCategory::MalformedResponse)
    );
}

#[test]
fn rejects_blocked_retry_after_beyond_the_scope_window() {
    // Invite/relink window is 300s; 301 is out of bounds.
    assert_eq!(
        parse_reserve_outcome(
            200,
            r#"{"outcome":"blocked","retry_after_seconds":301}"#,
            Scope::Invite
        ),
        Err(UnavailableCategory::MalformedResponse)
    );
    // Community window is 86_400s; a value within it must be accepted.
    assert_eq!(
        parse_reserve_outcome(
            200,
            r#"{"outcome":"blocked","retry_after_seconds":86400}"#,
            Scope::CommunityUser
        ),
        Ok(Outcome::Blocked {
            retry_after_seconds: 86_400
        })
    );
}

// ── Policy mapping ───────────────────────────────────────────────────────────

#[test]
fn community_scopes_share_one_policy() {
    assert_eq!(Scope::CommunityUser.policy(), "community");
    assert_eq!(Scope::CommunitySession.policy(), "community");
    assert_eq!(Scope::CommunityNetwork.policy(), "community");
}

#[test]
fn invite_and_relink_have_independent_policies() {
    assert_eq!(Scope::Invite.policy(), "invite");
    assert_eq!(Scope::Relink.policy(), "relink");
    assert_ne!(Scope::Invite.policy(), Scope::Relink.policy());
}

#[test]
fn recovery_has_its_own_policy_distinct_from_relink() {
    // Handoff 057 §5.2: must not share a budget with `/relink` — a
    // distinct policy identifier is what keeps the coordinator from
    // pooling the two flows' capacity together.
    assert_eq!(Scope::Recovery.policy(), "recovery");
    assert_ne!(Scope::Recovery.policy(), Scope::Relink.policy());
    assert_ne!(Scope::Recovery.label(), Scope::Relink.label());
}
