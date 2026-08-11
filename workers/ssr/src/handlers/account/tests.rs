use super::*;
use crate::db::identity::LinkedIdentitySummary;
use crate::db::membership::CommunitySummary;

fn identity(namespace: &str, linked_at: &str) -> LinkedIdentitySummary {
    LinkedIdentitySummary {
        identity_namespace_id: namespace.to_owned(),
        linked_at: linked_at.to_owned(),
    }
}

fn community(name: &str) -> CommunitySummary {
    CommunitySummary {
        community_id: "com_test".to_owned(),
        community_name: name.to_owned(),
        timezone: "Asia/Tokyo".to_owned(),
        role: "member".to_owned(),
    }
}

// ── §5.4: no linked identities / no communities ──────────────────────────

#[test]
fn zero_linked_identities_shows_the_none_yet_message() {
    let html = render_identities(&[]);
    assert!(html.contains(i18n::JA_ACCOUNT_NO_LINKED_IDENTITIES));
}

#[test]
fn zero_communities_shows_the_no_communities_message() {
    // RFC-081 §6: the state a no-membership principal actually sees.
    let html = render_communities(&[]);
    assert!(html.contains(i18n::JA_ACCOUNT_NO_COMMUNITIES));
}

// ── §5.4: identities render namespace and linked-at, and only those ──────

#[test]
fn linked_identity_shows_namespace_and_linked_at() {
    let html = render_identities(&[identity("idns_local_fake", "2026-08-11T00:00:00.000Z")]);
    assert!(html.contains("idns_local_fake"));
    assert!(html.contains("2026-08-11T00:00:00.000Z"));
}

#[test]
fn linked_identity_output_is_escaped() {
    // `LinkedIdentitySummary` cannot structurally carry a subject or
    // digest (the query that produces it never selects those columns) —
    // this test covers the one field that is still free text at the type
    // level, `identity_namespace_id`, for the ordinary XSS-escaping
    // discipline every other rendered field in this codebase gets.
    let html = render_identities(&[identity(
        "<script>evil</script>",
        "2026-08-11T00:00:00.000Z",
    )]);
    assert!(!html.contains("<script>evil</script>"));
    assert!(html.contains("&lt;script&gt;"));
}

#[test]
fn multiple_linked_identities_all_render() {
    let html = render_identities(&[
        identity("idns_local_fake", "2026-08-11T00:00:00.000Z"),
        identity("idns_other", "2026-08-12T00:00:00.000Z"),
    ]);
    assert!(html.contains("idns_local_fake"));
    assert!(html.contains("idns_other"));
}

// ── §5.4: communities render name, escaped ───────────────────────────────

#[test]
fn community_name_is_escaped() {
    let html = render_communities(&[community("<script>evil</script>")]);
    assert!(!html.contains("<script>evil</script>"));
    assert!(html.contains("&lt;script&gt;"));
}

#[test]
fn multiple_communities_all_render() {
    let html = render_communities(&[community("Community A"), community("Community B")]);
    assert!(html.contains("Community A"));
    assert!(html.contains("Community B"));
}

// ── §5.2/§5.4: freshness display, not an operation gate ──────────────────

#[test]
fn fresh_session_shows_the_can_manage_message() {
    let html = render_freshness(true);
    assert!(html.contains(i18n::JA_ACCOUNT_FRESH_CAN_MANAGE));
    assert!(!html.contains(i18n::JA_ACCOUNT_STALE_SIGN_IN_AGAIN));
}

#[test]
fn stale_session_shows_the_sign_in_again_message_and_link() {
    let html = render_freshness(false);
    assert!(html.contains(i18n::JA_ACCOUNT_STALE_SIGN_IN_AGAIN));
    assert!(html.contains("/identity/start?action=sign_in"));
    assert!(!html.contains(i18n::JA_ACCOUNT_FRESH_CAN_MANAGE));
}

// ── The full body: recovery credential always "none yet" in 5a ───────────

#[test]
fn recovery_credential_section_always_shows_none_yet() {
    // 5a creates no recovery credential (Slice 5b's job) — the account
    // page must not claim one might exist.
    let html = render_body(&[], &[], true);
    assert!(html.contains(i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_NONE));
}

#[test]
fn full_body_never_mentions_subject_digest_or_issuer() {
    // Defence-in-depth beyond the type-level guarantee: even with
    // deliberately suspicious-looking (but type-legal) input, nothing
    // resembling identity internals appears in the rendered page.
    let html = render_body(
        &[identity("idns_local_fake", "2026-08-11T00:00:00.000Z")],
        &[community("Test Community")],
        false,
    );
    for forbidden in ["subject", "digest", "issuer", "sub=", "https://fake-issuer"] {
        assert!(
            !html.to_ascii_lowercase().contains(forbidden),
            "account page must never mention {forbidden}"
        );
    }
}

#[test]
fn full_body_includes_every_section_heading() {
    let html = render_body(&[], &[], true);
    assert!(html.contains(i18n::JA_ACCOUNT_LINKED_IDENTITIES_HEADING));
    assert!(html.contains(i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_HEADING));
    assert!(html.contains(i18n::JA_ACCOUNT_COMMUNITIES_HEADING));
}
