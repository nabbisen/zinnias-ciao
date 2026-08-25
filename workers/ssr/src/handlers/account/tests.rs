use super::*;
use crate::db::identity::LinkedIdentitySummary;
use crate::db::membership::CommunitySummary;
use zinnias_ciao_contracts::Locale;

fn identity(namespace: &str, linked_at: &str) -> LinkedIdentitySummary {
    LinkedIdentitySummary {
        id: "idty_test".to_owned(),
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

/// Duplicated locally per `handlers/relink.rs`'s own precedent (Handoff
/// 075) rather than shared/exported.
fn contains_japanese_codepoint(s: &str) -> bool {
    s.chars().any(|c| {
        let cp = c as u32;
        (0x3040..=0x30FF).contains(&cp)
            || (0x4E00..=0x9FFF).contains(&cp)
            || (0x3000..=0x303F).contains(&cp)
            || (0xFF00..=0xFFEF).contains(&cp)
    })
}

// ── §5.4: no linked identities / no communities ──────────────────────────

#[test]
fn zero_linked_identities_shows_the_none_yet_message() {
    let html = render_identities(&[], Locale::Ja);
    assert!(html.contains(i18n::JA_ACCOUNT_NO_LINKED_IDENTITIES));
}

#[test]
fn zero_communities_shows_the_no_communities_message() {
    // RFC-081 §6: the state a no-membership principal actually sees.
    let html = render_communities(&[], Locale::Ja);
    assert!(html.contains(i18n::JA_ACCOUNT_NO_COMMUNITIES));
}

// ── §5.4: identities render namespace and linked-at, and only those ──────

#[test]
fn linked_identity_shows_namespace_and_linked_at() {
    let html = render_identities(
        &[identity("idns_local_fake", "2026-08-11T00:00:00.000Z")],
        Locale::Ja,
    );
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
    let html = render_identities(
        &[identity(
            "<script>evil</script>",
            "2026-08-11T00:00:00.000Z",
        )],
        Locale::Ja,
    );
    assert!(!html.contains("<script>evil</script>"));
    assert!(html.contains("&lt;script&gt;"));
}

#[test]
fn multiple_linked_identities_all_render() {
    let html = render_identities(
        &[
            identity("idns_local_fake", "2026-08-11T00:00:00.000Z"),
            identity("idns_other", "2026-08-12T00:00:00.000Z"),
        ],
        Locale::Ja,
    );
    assert!(html.contains("idns_local_fake"));
    assert!(html.contains("idns_other"));
}

// ── §5.4: communities render name, escaped ───────────────────────────────

#[test]
fn community_name_is_escaped() {
    let html = render_communities(&[community("<script>evil</script>")], Locale::Ja);
    assert!(!html.contains("<script>evil</script>"));
    assert!(html.contains("&lt;script&gt;"));
}

#[test]
fn multiple_communities_all_render() {
    let html = render_communities(
        &[community("Community A"), community("Community B")],
        Locale::Ja,
    );
    assert!(html.contains("Community A"));
    assert!(html.contains("Community B"));
}

// ── §5.2/§5.4: freshness display, not an operation gate ──────────────────

#[test]
fn fresh_session_shows_the_can_manage_message() {
    let html = render_freshness(true, Locale::Ja);
    assert!(html.contains(i18n::JA_ACCOUNT_FRESH_CAN_MANAGE));
    assert!(!html.contains(i18n::JA_ACCOUNT_STALE_SIGN_IN_AGAIN));
}

#[test]
fn stale_session_shows_the_sign_in_again_message_and_link() {
    let html = render_freshness(false, Locale::Ja);
    assert!(html.contains(i18n::JA_ACCOUNT_STALE_SIGN_IN_AGAIN));
    assert!(html.contains("/identity/start?action=sign_in"));
    assert!(!html.contains(i18n::JA_ACCOUNT_FRESH_CAN_MANAGE));
}

// ── The full body: recovery credential existence (Handoff 057 §5.1) ──────

#[test]
fn recovery_credential_section_shows_none_yet_when_absent() {
    let html = render_body(&[], &[], true, false, "tok", None, Locale::Ja);
    assert!(html.contains(i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_NONE));
    assert!(!html.contains(i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_EXISTS));
}

#[test]
fn recovery_credential_section_shows_exists_when_present() {
    let html = render_body(&[], &[], true, true, "tok", None, Locale::Ja);
    assert!(html.contains(i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_EXISTS));
    assert!(!html.contains(i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_NONE));
}

#[test]
fn recovery_regenerate_form_always_present_and_carries_the_token() {
    // Reachable whether or not a credential already exists — for a
    // pre-057 linked member with no credential yet, this doubles as
    // "generate for the first time."
    for has_credential in [false, true] {
        let html = render_body(
            &[],
            &[],
            true,
            has_credential,
            "regen-token-value",
            None,
            Locale::Ja,
        );
        assert!(html.contains("action=\"/account/recovery/regenerate\""));
        assert!(html.contains("regen-token-value"));
    }
}

#[test]
fn reveal_renders_the_code_and_warning_only_when_present() {
    let without = render_body(&[], &[], true, true, "tok", None, Locale::Ja);
    assert!(!without.contains("id=\"recovery-code-reveal\""));

    let with = render_body(
        &[],
        &[],
        true,
        true,
        "tok",
        Some("ABCD-EFGH-JKMN"),
        Locale::Ja,
    );
    assert!(with.contains("id=\"recovery-code-reveal\""));
    assert!(with.contains("ABCD-EFGH-JKMN"));
    assert!(with.contains(i18n::JA_ACCOUNT_RECOVERY_REVEAL_WARNING));
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
        true,
        "tok",
        None,
        Locale::Ja,
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
    let html = render_body(&[], &[], true, false, "tok", None, Locale::Ja);
    assert!(html.contains(i18n::JA_ACCOUNT_LINKED_IDENTITIES_HEADING));
    assert!(html.contains(i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_HEADING));
    assert!(html.contains(i18n::JA_ACCOUNT_COMMUNITIES_HEADING));
}

#[test]
fn each_linked_identity_carries_its_own_unlink_link() {
    let html = render_identities(
        &[identity("idns_local_fake", "2026-08-11T00:00:00.000Z")],
        Locale::Ja,
    );
    assert!(html.contains("/account/unlink/idty_test"));
    assert!(html.contains(i18n::JA_ACCOUNT_UNLINK_LABEL));
}

// ── RFC-084 §8 (Handoff 084): English-locale render assertion ────────────

#[test]
fn account_page_renders_with_no_japanese_codepoint_in_english_locale() {
    let html = render_body(
        &[identity("idns_local_fake", "2026-08-11T00:00:00.000Z")],
        &[community("Test Community")],
        true,
        true,
        "tok",
        Some("ABCD-EFGH-JKMN"),
        Locale::En,
    );
    assert!(
        !contains_japanese_codepoint(&html),
        "English-locale account page must contain no Japanese codepoint, found some in: {html}"
    );

    let ja_html = render_body(
        &[identity("idns_local_fake", "2026-08-11T00:00:00.000Z")],
        &[community("Test Community")],
        true,
        true,
        "tok",
        Some("ABCD-EFGH-JKMN"),
        Locale::Ja,
    );
    assert!(
        contains_japanese_codepoint(&ja_html),
        "Japanese-locale account page render must contain Japanese text"
    );
}
