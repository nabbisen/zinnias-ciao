use super::*;

// ── §5.2: the safe-return allowlist ────────────────────────────────────

#[test]
fn allowlisted_destination_is_accepted() {
    assert_eq!(resolve_safe_return(Some("/")), "/");
}

#[test]
fn account_destination_is_accepted_handoff_055() {
    // Handoff 055 §5.4: the allowlist's first growth since §5.2 — no live
    // call site produces this value yet (see the module doc comment on
    // `ALLOWED_RETURN_DESTINATIONS`), but the function's own behavior for
    // it is proven here regardless of what currently reaches it.
    assert_eq!(resolve_safe_return(Some("/account")), "/account");
}

#[test]
fn missing_stored_value_falls_back_to_the_safe_default() {
    assert_eq!(resolve_safe_return(None), "/");
}

#[test]
fn protocol_relative_host_is_rejected() {
    // The classic "relative paths are safe" trap: `//evil.example` is
    // relative by some readings and absolute (protocol-relative) to a
    // browser — it must fall back to the safe default, never pass through.
    assert_eq!(resolve_safe_return(Some("//evil.example")), "/");
}

#[test]
fn absolute_urls_are_rejected() {
    for value in [
        "https://evil.example",
        "http://evil.example/",
        "https://evil.example/path",
    ] {
        assert_eq!(resolve_safe_return(Some(value)), "/", "value={value}");
    }
}

#[test]
fn unlisted_relative_paths_are_rejected() {
    for value in ["/admin", "/c/some-community/home", "/join", ""] {
        assert_eq!(resolve_safe_return(Some(value)), "/", "value={value}");
    }
}

#[test]
fn scheme_relative_and_backslash_variants_are_rejected() {
    // Browsers historically treat backslashes as forward slashes in some
    // contexts — a value that only guards against `//` is not enough.
    for value in ["\\\\evil.example", "/\\evil.example", "\\/evil.example"] {
        assert_eq!(resolve_safe_return(Some(value)), "/", "value={value}");
    }
}

// ── SessionProvenance exhaustiveness (§5.4) ────────────────────────────

#[test]
fn session_provenance_round_trips_all_three_variants() {
    use crate::db::session::SessionProvenance;
    assert_eq!(
        SessionProvenance::InviteRedemption.as_str(),
        "invite_redemption"
    );
    assert_eq!(SessionProvenance::Relink.as_str(), "relink");
    assert_eq!(
        SessionProvenance::ExternalIdentity.as_str(),
        "external_identity"
    );
    // Every variant produces a distinct value — a compile-time-checked
    // enum only closes the typo hazard (Handoff 054 §5.4) if two variants
    // can never collide on the same stored string.
    let all = [
        SessionProvenance::InviteRedemption.as_str(),
        SessionProvenance::Relink.as_str(),
        SessionProvenance::ExternalIdentity.as_str(),
    ];
    let unique: std::collections::HashSet<_> = all.iter().collect();
    assert_eq!(unique.len(), all.len());
}

// ── Invite-reference extraction (reads the existing __join_ticket cookie)

#[test]
fn invite_reference_extracted_from_join_ticket_cookie() {
    // Mirrors legacy_post_profile's own cookie shape exactly:
    // "{ticket}|{invite_id}:{community_id}".
    let cookie_value = "sometoken|inv_abc123:com_xyz789";
    let (_ticket, ticket_value) = cookie_value.split_once('|').unwrap();
    let invite_id = ticket_value.split(':').next().unwrap();
    assert_eq!(invite_id, "inv_abc123");
}

// ── urlencode: only the alphabet actually used (hex digests / random
// tokens) needs to survive unescaped; everything else must be percent-
// encoded so a malicious query-parameter value can never break out of
// the constructed authorize/token request.

#[test]
fn urlencode_passes_through_the_unreserved_alphabet() {
    let input = "abcXYZ019-_.~";
    assert_eq!(urlencode(input), input);
}

#[test]
fn urlencode_escapes_everything_else() {
    assert_eq!(urlencode("a b"), "a%20b");
    assert_eq!(urlencode("a&b=c"), "a%26b%3Dc");
    assert_eq!(urlencode("https://x"), "https%3A%2F%2Fx");
}
