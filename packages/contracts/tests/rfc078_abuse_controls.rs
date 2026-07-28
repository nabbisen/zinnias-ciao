//! RFC-078 fail-closed abuse-control source/config contract gates.
//!
//! Dedicated module per the architecture-reviewed implementation handoff:
//! these checks do not belong in the monolithic `release_gates.rs` file.
//! Focused unit coverage of the pure header/transition logic itself lives
//! natively in `workers/ssr/src/abuse_control/tests.rs` and
//! `workers/ssr/src/abuse_limiter/tests.rs`; this file checks source and
//! configuration invariants that only make sense read across files.

const ABUSE_CONTROL_SRC: &str = include_str!("../../../workers/ssr/src/abuse_control.rs");
const ABUSE_LIMITER_SRC: &str = include_str!("../../../workers/ssr/src/abuse_limiter.rs");
const LIB_SRC: &str = include_str!("../../../workers/ssr/src/lib.rs");
const JOIN_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/join.rs");
const RELINK_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/relink.rs");
const COMMUNITY_CREATE_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/community_create.rs");
const WRANGLER_TOML_SRC: &str = include_str!("../../../wrangler.toml");
const PACKAGE_JSON_SRC: &str = include_str!("../../../package.json");
const CI_WORKFLOW_SRC: &str = include_str!("../../../.github/workflows/ci.yml");
const ABUSE_CONTROLS_SMOKE_SRC: &str = include_str!("../../../scripts/smoke/abuse-controls.mjs");
const SRC_DIR_LISTING: &[&str] = &[
    "abuse_control.rs",
    "abuse_limiter.rs",
    "audit.rs",
    "authz.rs",
    "codlet.rs",
    "crypto.rs",
    "db.rs",
    "errors.rs",
    "form_token.rs",
    "lib.rs",
    "render.rs",
    "session.rs",
];

#[test]
fn rate_limit_module_is_fully_retired() {
    assert!(
        !std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../workers/ssr/src/rate_limit.rs"
        ))
        .exists(),
        "workers/ssr/src/rate_limit.rs must be deleted, not retained as a compatibility shim"
    );
    assert!(
        !LIB_SRC.contains("mod rate_limit"),
        "lib.rs must not register the retired rate_limit module"
    );
    for (name, source) in [
        ("join", JOIN_HANDLER_SRC),
        ("relink", RELINK_HANDLER_SRC),
        ("community_create", COMMUNITY_CREATE_HANDLER_SRC),
    ] {
        assert!(
            !source.contains("rate_limit::"),
            "{name} handler must not reference the retired rate_limit module"
        );
    }
}

#[test]
fn wrangler_toml_declares_the_durable_object_and_has_no_runtime_rate_limit_kv() {
    assert!(
        WRANGLER_TOML_SRC.contains("[exports.AbuseLimiter]")
            && WRANGLER_TOML_SRC.contains(r#"type    = "durable-object""#)
            && WRANGLER_TOML_SRC.contains(r#"storage = "sqlite""#),
        "wrangler.toml must declare exactly one top-level SQLite-backed AbuseLimiter export"
    );
    assert_eq!(
        WRANGLER_TOML_SRC.matches("[exports.AbuseLimiter]").count(),
        1,
        "exports must not be repeated per named environment (RFC-078 §Platform and dependency boundary)"
    );
    assert_eq!(
        WRANGLER_TOML_SRC
            .matches(r#"class_name = "AbuseLimiter""#)
            .count(),
        4,
        "the ABUSE_LIMITER binding must be repeated for root, dev, staging, and production"
    );
    assert!(
        !WRANGLER_TOML_SRC.contains("RATE_LIMIT"),
        "tracked wrangler.toml must no longer declare the retired RATE_LIMIT KV binding"
    );
    assert!(
        !WRANGLER_TOML_SRC.contains("new_sqlite_classes")
            && !WRANGLER_TOML_SRC.contains("[[migrations]]"),
        "must use declarative exports, never the legacy Durable Object migrations array"
    );
}

#[test]
fn abuse_limiter_is_exported_at_crate_root_for_wasm32_only_and_never_routed_publicly() {
    assert!(
        LIB_SRC.contains("mod abuse_limiter;") && LIB_SRC.contains("mod abuse_control;"),
        "lib.rs must register both RFC-078 modules"
    );
    assert!(
        LIB_SRC.contains("pub use abuse_limiter::AbuseLimiter;")
            && LIB_SRC.contains(r#"#[cfg(target_arch = "wasm32")]"#),
        "AbuseLimiter must be re-exported at crate root, gated to wasm32 (its wasm-bindgen glue does not compile natively)"
    );
    assert!(
        !LIB_SRC.contains("/v1/reserve") && !LIB_SRC.contains("/v1/reset"),
        "the private Durable Object protocol paths must never be reachable through the public HTTP router"
    );
}

#[test]
fn ingress_validator_uses_only_headers_get_never_get_all() {
    assert!(
        ABUSE_CONTROL_SRC.contains("headers.get(\"CF-Worker\")")
            && ABUSE_CONTROL_SRC.contains("headers.get(\"CF-Connecting-IP\")"),
        "ingress validation must read the trusted headers with Headers::get"
    );
    assert!(
        !ABUSE_CONTROL_SRC.contains("get_all("),
        "H-B1: get_all's worker-sys binding is declared without `catch` and aborts rather than \
         returning Unavailable; the ingress path must never call it"
    );
    assert!(
        !ABUSE_LIMITER_SRC.contains("get_all("),
        "H-B1: the Durable Object side must also never call the non-catching get_all binding"
    );
    assert!(
        !ABUSE_CONTROL_SRC.contains("X-Forwarded-For")
            && !ABUSE_CONTROL_SRC.contains("\"unknown\""),
        "the fail-open X-Forwarded-For fallback and the shared 'unknown' subject literal must not return"
    );
}

#[test]
fn ingress_rejects_upstream_worker_and_ipv6_header_and_class_e() {
    assert!(
        ABUSE_CONTROL_SRC.contains("IngressRejection::UpstreamWorker")
            && ABUSE_CONTROL_SRC.contains("IngressRejection::Ipv6HeaderPresent")
            && ABUSE_CONTROL_SRC.contains("IngressRejection::ClassEAddress"),
        "the closed ingress rejection categories must cover CF-Worker, CF-Connecting-IPv6, and Class E"
    );
    assert!(
        ABUSE_CONTROL_SRC.contains("to_ipv4_mapped()"),
        "IPv4-mapped IPv6 must be normalized to IPv4 before the Class E test (H-B1 / design-review N1)"
    );
    let canonicalize_pos = ABUSE_CONTROL_SRC
        .find("fn canonicalize")
        .expect("canonicalize routine must exist");
    let mapped_pos = ABUSE_CONTROL_SRC
        .find("to_ipv4_mapped()")
        .expect("normalization call must exist");
    let reject_fn_pos = ABUSE_CONTROL_SRC
        .find("fn reject_class_e")
        .expect("class E rejection helper must exist");
    assert!(
        canonicalize_pos < mapped_pos && mapped_pos < reject_fn_pos,
        "IPv4-mapped normalization must live inside canonicalize, ahead of the class E helper it calls"
    );
}

#[test]
fn scope_labels_are_the_five_closed_stable_strings() {
    for label in [
        "\"invite\"",
        "\"relink\"",
        "\"community-user\"",
        "\"community-session\"",
        "\"community-network\"",
    ] {
        assert!(
            ABUSE_CONTROL_SRC.contains(label),
            "Scope::label must produce the stable identifier {label}"
        );
    }
    assert!(
        ABUSE_CONTROL_SRC.contains("abuse-control:v1:"),
        "HMAC domain separation must use the fixed v1 prefix"
    );
}

#[test]
fn protocol_is_versioned_closed_and_bounded() {
    assert!(
        ABUSE_CONTROL_SRC.contains("/v1/reserve") && ABUSE_CONTROL_SRC.contains("/v1/reset"),
        "the private protocol must use fixed versioned paths"
    );
    assert!(
        ABUSE_CONTROL_SRC.contains("deny_unknown_fields")
            && ABUSE_LIMITER_SRC.contains("deny_unknown_fields"),
        "both caller and Durable Object protocol bodies must reject unknown fields"
    );
    assert!(
        ABUSE_CONTROL_SRC.contains("PROTOCOL_MAX_BODY_BYTES")
            && ABUSE_LIMITER_SRC.contains("PROTOCOL_MAX_BODY_BYTES"),
        "the request/response body bound must be shared and named, not duplicated as a magic number"
    );
}

#[test]
fn transition_and_alarm_are_fail_closed_by_construction() {
    assert!(
        ABUSE_LIMITER_SRC.contains("fn alarm(&self)"),
        "alarm() must be implemented explicitly; the DurableObject trait default is unimplemented!() and panics"
    );
    assert!(
        ABUSE_LIMITER_SRC.contains("#[durable_object(alarm)]"),
        "the class attribute must request alarm bindgen, matching the hand-written alarm() override"
    );
    assert!(
        !ABUSE_CONTROL_SRC.contains("Outcome::Unavailable => Outcome::Allowed")
            && !ABUSE_LIMITER_SRC.contains("unwrap_or(TransitionOutcome::Allowed)"),
        "no code path may substitute Allowed for a coordinator or storage failure"
    );
}

#[test]
fn handler_ordering_is_ingress_then_token_then_reserve_then_credential_lookup() {
    for (name, source, credential_marker) in [
        ("join", JOIN_HANDLER_SRC, "invite::find_valid"),
        (
            "relink",
            RELINK_HANDLER_SRC,
            "relink_db::find_valid_by_hmac",
        ),
    ] {
        let ingress_pos = source
            .find("canonical_client_network")
            .unwrap_or_else(|| panic!("{name} must call abuse_control::canonical_client_network"));
        let consume_pos = source.find("consume_detailed").unwrap_or_else(|| {
            panic!(
                "{name} must use form_token::consume_detailed, not the Option-collapsing consume"
            )
        });
        let reserve_pos = source
            .find("abuse_control::reserve")
            .unwrap_or_else(|| panic!("{name} must call abuse_control::reserve"));
        let credential_pos = source
            .find(credential_marker)
            .unwrap_or_else(|| panic!("{name} must perform its credential lookup"));

        assert!(
            ingress_pos < consume_pos,
            "{name}: ingress validation must precede form-token D1 work"
        );
        assert!(
            consume_pos < reserve_pos,
            "{name}: form-token consumption must precede the reservation"
        );
        assert!(
            reserve_pos < credential_pos,
            "{name}: credential HMAC/D1 lookup must occur only after a reservation attempt"
        );
        assert!(
            source.contains("ConsumeResult::Replay(_)"),
            "{name}: replayed tokens must perform neither reservation nor credential lookup (H-N1)"
        );
        assert!(
            source.contains("abuse_control::reset"),
            "{name}: a valid credential result must attempt a limiter reset"
        );
    }
}

#[test]
fn join_replay_no_longer_discards_the_consume_result() {
    // H-N1 regression: `join.rs` previously called `form_token::consume` and
    // discarded the outcome with `let _ = ...`, so a replayed token still
    // proceeded to credential lookup. That pattern must not return.
    assert!(
        !JOIN_HANDLER_SRC.contains("let _ = crate::form_token::consume("),
        "join.rs must not silently discard the form-token consume outcome"
    );
    assert!(
        JOIN_HANDLER_SRC.contains("crate::form_token::consume_detailed")
            && JOIN_HANDLER_SRC.contains("ConsumeResult::Replay(_)"),
        "join.rs must classify Proceed vs Replay explicitly before reserving or looking up a credential"
    );
}

#[test]
fn community_creation_reserves_user_then_session_then_network_in_order() {
    let user_pos = COMMUNITY_CREATE_HANDLER_SRC
        .find("Scope::CommunityUser")
        .expect("community creation must reserve the user dimension");
    let session_pos = COMMUNITY_CREATE_HANDLER_SRC
        .find("Scope::CommunitySession")
        .expect("community creation must reserve the session dimension");
    let network_pos = COMMUNITY_CREATE_HANDLER_SRC
        .find("Scope::CommunityNetwork")
        .expect("community creation must reserve the network dimension");
    assert!(
        user_pos < session_pos && session_pos < network_pos,
        "community creation must reserve user, then session, then network, in that exact order"
    );
    assert!(
        !COMMUNITY_CREATE_HANDLER_SRC.contains("abuse_control::reset"),
        "community creation must not reset capacity on success (RFC-078 §Community-creation reservations)"
    );
    // `require_auth_or!` also appears in `get_new_community` above
    // `post_new_community` in this file; scope the ordering check to the
    // POST handler specifically rather than the first occurrence anywhere.
    let post_handler_start = COMMUNITY_CREATE_HANDLER_SRC
        .find("pub async fn post_new_community")
        .expect("post_new_community must exist");
    let post_handler_src = &COMMUNITY_CREATE_HANDLER_SRC[post_handler_start..];
    let ingress_pos = post_handler_src
        .find("canonical_client_network")
        .expect("community creation must validate ingress");
    let auth_pos = post_handler_src
        .find("require_auth_or!")
        .expect("community creation must authenticate");
    assert!(
        ingress_pos < auth_pos,
        "ingress validation must precede authentication, per RFC-078's community-creation ordering"
    );
}

#[test]
fn ci_and_lint_cover_the_ssr_workspace_and_all_targets() {
    assert!(
        PACKAGE_JSON_SRC.contains("cargo clippy --workspace --all-targets"),
        "package.json lint must match the all-target workspace Clippy policy, including workers/ssr"
    );
    assert!(
        PACKAGE_JSON_SRC.contains("\"test:abuse-controls\""),
        "package.json must expose a domain-named RFC-078 abuse-controls test command"
    );
    assert!(
        CI_WORKFLOW_SRC.contains("--workspace") && CI_WORKFLOW_SRC.contains("--all-targets"),
        "CI must run native SSR tests and all-target Clippy, not only domain/contracts"
    );
    assert!(
        CI_WORKFLOW_SRC.contains("wasm32-unknown-unknown"),
        "CI must retain the SSR wasm32 type-check"
    );
}

#[test]
fn abuse_controls_smoke_proves_no_lost_increments_under_concurrency() {
    // I-B1: sequential requests alone cannot detect a lost-increment race —
    // the original B3 defect. The local runtime evidence must fire a genuine
    // concurrent burst and assert an *exact* admitted count, for both the
    // invite/relink policy and the community-creation policy, so this pins
    // that the evidence exists and cannot silently regress to
    // sequential-only coverage.
    assert!(
        ABUSE_CONTROLS_SMOKE_SRC.contains("Promise.all"),
        "the smoke evidence must fire a genuine concurrent burst, not only sequential requests"
    );
    assert!(
        ABUSE_CONTROLS_SMOKE_SRC.contains("inviteConcurrencyBurst")
            && ABUSE_CONTROLS_SMOKE_SRC.contains("communityCreationConcurrencyBurst"),
        "both the invite/relink and community-creation policies need concurrent-burst coverage"
    );
    assert!(
        ABUSE_CONTROLS_SMOKE_SRC.contains("counts[200] ?? 0,\n      10,")
            || ABUSE_CONTROLS_SMOKE_SRC.contains("counts[200] ?? 0, 10"),
        "the invite burst must assert an exact admitted count of 10, not a range"
    );
    assert!(
        ABUSE_CONTROLS_SMOKE_SRC.contains("counts[303] ?? 0,\n      3,")
            || ABUSE_CONTROLS_SMOKE_SRC.contains("counts[303] ?? 0, 3"),
        "the community-creation burst must assert an exact admitted count of 3, not a range"
    );
}

#[test]
fn release_gates_has_no_stale_rate_limit_include() {
    // release_gates.rs's pre-existing RFC-024/RFC-057 tests were updated in
    // place to reference the new abuse_control/Scope calls (a minimal,
    // legitimate cross-reference) rather than moved here wholesale; this
    // guards only against the specific staleness the handoff called out —
    // an include_str! of the file that RFC-078 deletes.
    let release_gates_src = include_str!("release_gates.rs");
    assert!(
        !release_gates_src.contains("rate_limit.rs"),
        "release_gates.rs must not retain an include_str! of the deleted rate_limit.rs"
    );
}

#[test]
fn ssr_module_inventory_documents_the_two_new_rfc078_modules() {
    // A light documentation-truth check: the architecture doc's workspace
    // layout listing should be kept in sync when new top-level src modules
    // are added. This does not enumerate every module — only guards the two
    // RFC-078 additions against silent drift from the doc.
    let architecture_doc = include_str!("../../../docs/src/developer/architecture.md");
    for module in SRC_DIR_LISTING {
        if *module == "abuse_control.rs" || *module == "abuse_limiter.rs" {
            assert!(
                architecture_doc.contains(module),
                "docs/src/developer/architecture.md must list {module} in the workspace layout"
            );
        }
    }
}
