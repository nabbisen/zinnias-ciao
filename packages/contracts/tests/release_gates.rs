//! Release-gate checks (RFC-015).
//! Every item here maps to a row in the MVP release-gate matrix.

#![allow(clippy::assertions_on_constants)]

use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::{
    AppError, FORM_TOKEN_TTL_SECONDS, RELINK_CODE_TTL_SECONDS, SESSION_TTL_SECONDS,
};

// ── Session / auth gates ──────────────────────────────────────────────────

#[test]
fn session_ttl_positive_and_bounded() {
    assert!(
        SESSION_TTL_SECONDS > 0,
        "session TTL must be positive (Max-Age=0 bug)"
    );
    assert!(SESSION_TTL_SECONDS >= 3600, "session TTL too short");
    assert!(
        SESSION_TTL_SECONDS <= 31 * 86400,
        "session TTL too long for invite-only MVP"
    );
}

#[test]
fn form_token_ttl_shorter_than_session() {
    assert!(
        FORM_TOKEN_TTL_SECONDS < SESSION_TTL_SECONDS,
        "form token must expire before the session"
    );
}

#[test]
fn session_ttl_never_derived_from_token_exp() {
    // Documents the regression: if someone naively computed TTL as
    // token_exp - now and the token was at the JWT leeway edge (~55s past exp),
    // Max-Age would be <= 0 and the browser would discard the cookie immediately.
    let token_exp: i64 = 1_000_000_000;
    let now_at_edge: i64 = 1_000_000_055; // 55 s past exp — within 60 s leeway
    let derived: i64 = token_exp - now_at_edge;
    assert!(
        derived <= 0,
        "derived TTL {} <= 0 demonstrates the bug",
        derived
    );
    // The correct value is always the constant:
    assert!(SESSION_TTL_SECONDS as i64 > 0);
}

// ── Error model gates ─────────────────────────────────────────────────────

#[test]
fn not_found_and_forbidden_same_message() {
    assert_eq!(
        AppError::not_found().user_message,
        AppError::forbidden().user_message
    );
    assert!(
        RENDER_SRC.contains("fn recovery_links()")
            && RENDER_SRC.contains("href=\\\"/\\\"")
            && RENDER_SRC.contains("href=\\\"/join\\\"")
            && LIB_SRC.contains("is_not_found_error(&error)")
            && LIB_SRC.contains("render::not_found()"),
        "Generic not-found/forbidden and error pages must be recoverable and expected authorization denial must not render as internal error"
    );
}

#[test]
fn internal_error_message_generic() {
    let msg = AppError::internal().user_message;
    assert!(!msg.to_lowercase().contains("sql"));
    assert!(!msg.to_lowercase().contains("panic"));
    assert!(!msg.to_lowercase().contains("stack"));
}

#[test]
fn invite_error_message_generic() {
    let msg = AppError::invite_invalid().user_message;
    assert!(!msg.to_lowercase().contains("hmac"));
    assert!(!msg.to_lowercase().contains("hash"));
    assert!(!msg.to_lowercase().contains("database"));
}

#[test]
fn token_invalid_error_is_retryable() {
    assert!(AppError::token_invalid().retryable);
}

// ── Token purpose completeness gate ──────────────────────────────────────

#[test]
fn all_state_changing_routes_have_token_purpose() {
    // Every mutating route needs a purpose string so tokens can be scoped.
    let required = [
        token_purpose::SET_STATUS,
        token_purpose::SAVE_NOTE,
        token_purpose::DELETE_NOTE,
        token_purpose::CREATE_EVENT,
        token_purpose::EDIT_EVENT,
        token_purpose::CANCEL_EVENT,
        token_purpose::ATTENDANCE_OVERRIDE,
        token_purpose::ADMIN_HIDE_NOTE,
        token_purpose::REVOKE_INVITE,
        token_purpose::CALENDAR_REGENERATE,
        token_purpose::CALENDAR_REVOKE,
        token_purpose::CALENDAR_MATRIX_CSV_EXPORT,
        token_purpose::COMMUNITY_EXPORT,
        token_purpose::CREATE_TEMPLATE,
        token_purpose::DELETE_TEMPLATE,
        token_purpose::REMOVE_MEMBER,
        token_purpose::PROMOTE_MEMBER,
        token_purpose::DEMOTE_MEMBER,
        token_purpose::HELP_SIGNIN,
        token_purpose::REDEEM_RELINK,
        token_purpose::GENERATE_INVITE,
        token_purpose::REDEEM_INVITE,
        token_purpose::JOIN_PROFILE,
        token_purpose::LOGOUT,
        token_purpose::CREATE_COMMUNITY,
        token_purpose::CHANGE_DISPLAY_NAME,
    ];
    for p in required {
        assert!(!p.is_empty(), "token purpose must not be empty: {p}");
        assert!(
            !p.contains(' '),
            "token purpose must not contain spaces: {p}"
        );
    }
}

#[test]
fn rfc054_member_facing_japanese_copy_avoids_technical_jargon() {
    use zinnias_ciao_contracts::i18n::*;

    let reviewed = [
        JA_SESSION_EXPIRED,
        JA_STATUS_GOING,
        JA_STATUS_NOT_GOING,
        JA_STATUS_ATTENDED,
        JA_STATUS_NO_ANSWER,
        JA_STATUS_CLEAR,
        JA_STATUS_CLEAR_LABEL,
        JA_CALENDAR_TITLE,
        JA_CALENDAR_DESCRIPTION,
        JA_CALENDAR_GENERATE,
        JA_CALENDAR_DISABLE,
        JA_CALENDAR_REGENERATE,
        JA_CALENDAR_PRIVACY_NOTE,
        JA_CALENDAR_GENERATED_FLASH,
        JA_CALENDAR_REVOKED_FLASH,
        JA_ME_CALENDAR_LABEL,
        JA_EXPORT_TITLE,
        JA_EXPORT_DESCRIPTION,
        JA_EXPORT_PRIVACY_NOTE,
        JA_EXPORT_DOWNLOAD_BTN,
        JA_ME_EXPORT_LINK,
        JA_ME_DATA_EXPORT,
    ];
    let forbidden = [
        "セッション",
        "トークン",
        "HMAC",
        "ICS",
        "iCS",
        "webcal",
        "JSON",
        "エクスポート",
    ];

    for text in reviewed {
        for term in forbidden {
            assert!(
                !text.contains(term),
                "RFC-054 Japanese member-facing copy contains technical jargon {term:?}: {text}"
            );
        }
    }
}

// ── D1 query budget documentation (RFC-029 / RFC-044) ────────────────────
//
// These constants document the approved D1 operation budget per route.
// The values are *code-level* counts (DB calls + form-token issues in the
// hot paths). They serve as a regression guard: if a future change inflates
// the count, the constant must be updated here with a deliberate review.
//
// All loop-based N+1s that existed before v0.24.0 are eliminated:
//   - Event Detail: list_for_day replaced with list_for_event_days (IN batch)
//   - Event Detail: per-note admin token loop replaced with a confirm-page link
//   - Export: per-event days+attendance+notes replaced with 3 IN queries
//
// The remaining per-day SET_STATUS token issue in Event Detail is bounded:
// single-day events = 1 token issue; recurring events bounded by
// After RFC-046 (event-bound SET_STATUS token), Event Detail issues exactly
// one token regardless of day count. The max-recurring budget collapses to
// the same value as single-day: 13 ops for any event.

/// Fixed D1 queries for Home (no loops above 1 per route):
/// memberships, events, member_count, my_statuses (IN), counts (IN),
/// communities_for_switcher + 2 spares = 8 total.
const QUERY_BUDGET_HOME: usize = 8;

/// Fixed D1 ops for Event Detail — any event, any recurrence count (RFC-046):
/// find_event, days, member_count, my_note, all_notes, all_members,
/// community, my_statuses (IN), counts (IN), all_day_attendances (IN),
/// 1 SET_STATUS token issue (event-bound, not per-day), 1 SAVE_NOTE token issue,
/// communities_for_switcher = 13 total.
/// Before RFC-046, max-recurring was 65 (52 per-day token writes). Now flat.
const QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY: usize = 13;
const QUERY_BUDGET_EVENT_DETAIL_MAX_RECURRING: usize = 13; // same: 1 token regardless of days

/// D1 queries for Export (any community size): 5 fixed + 3 IN batches = 8.
/// Was O(events * days) before v0.25.0; now a flat 8 regardless of size.
const QUERY_BUDGET_EXPORT: usize = 8;

#[test]
fn query_budgets_are_positive_and_ordered() {
    assert!(QUERY_BUDGET_HOME > 0);
    assert!(QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY > QUERY_BUDGET_HOME);
    // After RFC-046 both single-day and max-recurring are identical (13).
    assert_eq!(
        QUERY_BUDGET_EVENT_DETAIL_MAX_RECURRING, QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY,
        "RFC-046: event-bound token makes recurring cost identical to single-day"
    );
    assert!(QUERY_BUDGET_EXPORT > 0);
    // Export must be flat (well under the old per-event worst case):
    assert!(
        QUERY_BUDGET_EXPORT < 20,
        "Export budget {QUERY_BUDGET_EXPORT} exceeds expected flat upper bound"
    );
    // Event detail must be well under the old per-day worst case of 65:
    assert!(
        QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY < 20,
        "Event detail budget {QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY} suggests an N+1 regression"
    );
    assert!(
        QUERY_BUDGET_EVENT_DETAIL_MAX_RECURRING < 20,
        "Event detail recurring budget suggests an N+1 regression"
    );
}

// ── Static source query-count gates (RFC-044 §6.1) ───────────────────────
//
// Count `.await` calls on DB functions in the key handler source files and
// assert they don't regress above their declared budgets. Uses include_str! so
// the check fires on every `cargo test` run without a live database.
//
// The counting heuristic: lines containing `.await` in a handler are almost
// always D1 operations; non-DB awaits (form_data(), etc.) are few and counted
// conservatively. The gate fires if the count exceeds 2× the budget — tight
// enough to catch a major N+1 regression but loose enough to survive minor
// refactors without constant adjustment. A count approaching the 2× ceiling
// should trigger manual budget review.

const HOME_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/home.rs");
const EVENT_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/event.rs");
const EXPORT_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/export.rs");
const AUTH_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/auth.rs");
const CALENDAR_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/calendar.rs");
const COMMUNITY_CREATE_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/community_create.rs");
const ME_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/me.rs");
const LIB_SRC: &str = include_str!("../../../workers/ssr/src/lib.rs");
const AUTHZ_SRC: &str = include_str!("../../../workers/ssr/src/authz.rs");
const CALENDAR_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/calendar.rs");
const COMMUNITY_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/community.rs");
const EVENT_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/event.rs");
const ICS_SRC: &str = include_str!("../../../packages/contracts/src/ics.rs");
const WRANGLER_TOML_SRC: &str = include_str!("../../../wrangler.toml");
const GITIGNORE_SRC: &str = include_str!("../../../.gitignore");
const MIGRATION_0009_SRC: &str = include_str!("../../../migrations/0009_recurrence_v2.sql");
const MIGRATION_0011_SRC: &str =
    include_str!("../../../migrations/0011_membership_ui_language.sql");
const MIGRATION_0017_SRC: &str =
    include_str!("../../../migrations/0017_account_recovery_credentials.sql");
const EVENT_SERIES_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/event_series.rs");
const EVENT_ADMIN_DOMAIN_SRC: &str = include_str!("../../../packages/domain/src/event_admin.rs");
const CRYPTO_SRC: &str = include_str!("../../../workers/ssr/src/crypto.rs");
const CRYPTO_TESTS_SRC: &str = include_str!("../../../workers/ssr/src/crypto/tests.rs");
const CODLET_SRC: &str = include_str!("../../../workers/ssr/src/codlet.rs");
const SESSION_SRC: &str = include_str!("../../../workers/ssr/src/session.rs");
const HEALTH_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/health.rs");
const SETUP_SCRIPT_SRC: &str = include_str!("../../../scripts/setup.mjs");
const SETUP_CORE_SRC: &str = include_str!("../../../scripts/lib/setup-core.mjs");
const LOCAL_SECRET_FILE_SRC: &str = include_str!("../../../scripts/lib/local-secret-file.mjs");
const LOCAL_SECRET_TEST_SRC: &str = include_str!("../../../scripts/test-local-secret-setup.mjs");
const ISOLATED_WORKER_TEST_SRC: &str =
    include_str!("../../../scripts/lib/isolated-worker-test.mjs");
const BOOTSTRAP_SCRIPT_SRC: &str = include_str!("../../../scripts/bootstrap-cloudflare.mjs");
const BOOTSTRAP_CORE_SRC: &str = include_str!("../../../scripts/lib/bootstrap-cloudflare-core.mjs");
const BOOTSTRAP_TEST_SRC: &str = include_str!("../../../scripts/test-bootstrap-cloudflare.mjs");
const PEPPER_CONFIGURATION_TEST_SRC: &str =
    include_str!("../../../scripts/test-hmac-pepper-configuration.mjs");
const RFC077_DIRECT_PEPPER_CALLERS: &[(&str, &str, usize)] = &[
    ("codlet", CODLET_SRC, 2),
    ("session", SESSION_SRC, 1),
    ("health", HEALTH_HANDLER_SRC, 1),
    (
        "join",
        include_str!("../../../workers/ssr/src/handlers/join.rs"),
        3,
    ),
    (
        "relink",
        include_str!("../../../workers/ssr/src/handlers/relink.rs"),
        2,
    ),
    ("calendar", CALENDAR_HANDLER_SRC, 2),
    (
        "communities",
        include_str!("../../../workers/ssr/src/handlers/communities.rs"),
        1,
    ),
    ("community-create", COMMUNITY_CREATE_HANDLER_SRC, 1),
    ("me", ME_HANDLER_SRC, 2), // RFC-072 Slice B: post_language's own pepper resolution
    (
        "operator",
        include_str!("../../../workers/ssr/src/handlers/operator.rs"),
        1,
    ),
    (
        "admin-help-signin",
        include_str!("../../../workers/ssr/src/handlers/admin/help_signin.rs"),
        1,
    ),
    (
        "admin-members",
        include_str!("../../../workers/ssr/src/handlers/admin/members.rs"),
        1,
    ),
];
const RFC077_CODLET_ISSUE_CALLERS: &[(&str, &str, usize)] = &[
    (
        "templates",
        include_str!("../../../workers/ssr/src/handlers/templates.rs"),
        2,
    ),
    ("me", ME_HANDLER_SRC, 5), // RFC-072 Slice B: get_language + refresh_language_form
    ("export", EXPORT_HANDLER_SRC, 1),
    ("event", EVENT_HANDLER_SRC, 3),
    ("community-create", COMMUNITY_CREATE_HANDLER_SRC, 2),
    (
        "communities",
        include_str!("../../../workers/ssr/src/handlers/communities.rs"),
        1,
    ),
    ("calendar", CALENDAR_HANDLER_SRC, 2),
    (
        "admin-help-signin",
        include_str!("../../../workers/ssr/src/handlers/admin/help_signin.rs"),
        1,
    ),
    (
        "admin-role-transfer",
        include_str!("../../../workers/ssr/src/handlers/admin/role_transfer.rs"),
        1,
    ),
    (
        "admin-member-remove",
        include_str!("../../../workers/ssr/src/handlers/admin/member_remove.rs"),
        1,
    ),
    (
        "admin-members",
        include_str!("../../../workers/ssr/src/handlers/admin/members.rs"),
        2,
    ),
    (
        "event-attendance",
        include_str!("../../../workers/ssr/src/handlers/admin/events/attendance.rs"),
        1,
    ),
    (
        "event-cancel",
        include_str!("../../../workers/ssr/src/handlers/admin/events/cancel.rs"),
        1,
    ),
    (
        "event-copy",
        include_str!("../../../workers/ssr/src/handlers/admin/events/copy.rs"),
        1,
    ),
    (
        "event-create",
        include_str!("../../../workers/ssr/src/handlers/admin/events/create.rs"),
        1,
    ),
    (
        "event-edit",
        include_str!("../../../workers/ssr/src/handlers/admin/events/edit.rs"),
        1,
    ),
    (
        "event-notes",
        include_str!("../../../workers/ssr/src/handlers/admin/events/notes.rs"),
        1,
    ),
    (
        "event-occurrence",
        include_str!("../../../workers/ssr/src/handlers/admin/events/occurrence.rs"),
        1,
    ),
    (
        "event-recreate",
        include_str!("../../../workers/ssr/src/handlers/admin/events/recreate.rs"),
        1,
    ),
];
const RFC077_AUTH_HANDLER_SOURCES: &[(&str, &str)] = &[
    (
        "templates",
        include_str!("../../../workers/ssr/src/handlers/templates.rs"),
    ),
    ("me", ME_HANDLER_SRC),
    ("export", EXPORT_HANDLER_SRC),
    ("event", EVENT_HANDLER_SRC),
    ("community-create", COMMUNITY_CREATE_HANDLER_SRC),
    (
        "communities",
        include_str!("../../../workers/ssr/src/handlers/communities.rs"),
    ),
    ("calendar", CALENDAR_HANDLER_SRC),
    (
        "community",
        include_str!("../../../workers/ssr/src/handlers/community.rs"),
    ),
    (
        "join",
        include_str!("../../../workers/ssr/src/handlers/join.rs"),
    ),
    (
        "relink",
        include_str!("../../../workers/ssr/src/handlers/relink.rs"),
    ),
    ("auth", AUTH_HANDLER_SRC),
    ("home", HOME_HANDLER_SRC),
    (
        "admin-help-signin",
        include_str!("../../../workers/ssr/src/handlers/admin/help_signin.rs"),
    ),
    (
        "admin-role-transfer",
        include_str!("../../../workers/ssr/src/handlers/admin/role_transfer.rs"),
    ),
    (
        "admin-member-remove",
        include_str!("../../../workers/ssr/src/handlers/admin/member_remove.rs"),
    ),
    (
        "admin-members",
        include_str!("../../../workers/ssr/src/handlers/admin/members.rs"),
    ),
    (
        "event-attendance",
        include_str!("../../../workers/ssr/src/handlers/admin/events/attendance.rs"),
    ),
    (
        "event-cancel",
        include_str!("../../../workers/ssr/src/handlers/admin/events/cancel.rs"),
    ),
    (
        "event-copy",
        include_str!("../../../workers/ssr/src/handlers/admin/events/copy.rs"),
    ),
    (
        "event-create",
        include_str!("../../../workers/ssr/src/handlers/admin/events/create.rs"),
    ),
    (
        "event-edit",
        include_str!("../../../workers/ssr/src/handlers/admin/events/edit.rs"),
    ),
    (
        "event-notes",
        include_str!("../../../workers/ssr/src/handlers/admin/events/notes.rs"),
    ),
    (
        "event-occurrence",
        include_str!("../../../workers/ssr/src/handlers/admin/events/occurrence.rs"),
    ),
    (
        "event-recreate",
        include_str!("../../../workers/ssr/src/handlers/admin/events/recreate.rs"),
    ),
];
const RFC077_EXECUTABLE_GATE_SOURCES: &[(&str, &str)] = &[
    (
        "admin-role-transfer",
        include_str!("../../../scripts/smoke/admin-role-transfer.mjs"),
    ),
    (
        "calendar-matrix-csv-export",
        include_str!("../../../scripts/smoke/calendar-matrix-csv-export.mjs"),
    ),
    (
        "event-copy",
        include_str!("../../../scripts/smoke/event-copy.mjs"),
    ),
    (
        "help-signin",
        include_str!("../../../scripts/smoke/help-signin.mjs"),
    ),
    (
        "invite-redemption",
        include_str!("../../../scripts/smoke/invite-redemption.mjs"),
    ),
    (
        "member-management",
        include_str!("../../../scripts/smoke/member-management.mjs"),
    ),
    (
        "monthly-attendance-matrix",
        include_str!("../../../scripts/smoke/monthly-attendance-matrix.mjs"),
    ),
    (
        "recurrence-v2",
        include_str!("../../../scripts/smoke/recurrence-v2.mjs"),
    ),
    (
        "self-display-name-editing",
        include_str!("../../../scripts/smoke/self-display-name-editing.mjs"),
    ),
    (
        "audit-class-a-failures",
        include_str!("../../../scripts/test-audit-class-a-failures.mjs"),
    ),
];

#[test]
fn rfc077_has_one_secret_only_pepper_resolver() {
    assert_eq!(
        CRYPTO_SRC.matches(".secret(\"HMAC_PEPPER\")").count(),
        1,
        "RFC-077 requires exactly one HMAC_PEPPER secret-binding read"
    );
    assert!(
        !CRYPTO_SRC.contains(".var(\"HMAC_PEPPER\")")
            && !LIB_SRC.contains(".secret(\"HMAC_PEPPER\")")
            && !LIB_SRC.contains(".var(\"HMAC_PEPPER\")"),
        "RFC-077 forbids a plain-var fallback and binding reads outside crypto.rs"
    );
    assert!(
        CRYPTO_SRC.contains("pub struct HmacPepper(String)")
            && CRYPTO_SRC.contains("pub(crate) fn as_str(&self) -> &str")
            && !CRYPTO_SRC.contains("impl fmt::Debug for HmacPepper")
            && !CRYPTO_SRC.contains("impl fmt::Display for HmacPepper"),
        "validated pepper material must retain its narrow opaque interface"
    );
    assert!(
        CRYPTO_TESTS_SRC.contains("pepper_validation_uses_utf8_byte_length_and_preserves_input")
            && CRYPTO_TESTS_SRC.contains("repeat(4097)")
            && CRYPTO_TESTS_SRC.contains("for sentinel in LEGACY_SENTINELS"),
        "pepper validation boundary cases must remain covered"
    );
}

#[test]
fn rfc077_preflight_and_health_remain_fail_closed() {
    let classify = LIB_SRC
        .find("let security_class = request_security_class")
        .expect("request classification must exist");
    let preflight = LIB_SRC
        .find("crypto::pepper(&env)")
        .expect("protected request preflight must exist");
    let dispatch = LIB_SRC
        .find("|| dispatch_request(req")
        .expect("route dispatch must exist");
    assert!(classify < preflight && preflight < dispatch);
    for path in [
        "/manifest.webmanifest",
        "/sw.js",
        "/static/app.css",
        "/static/app.js",
        "/offline",
        "/version",
    ] {
        assert_eq!(
            LIB_SRC.matches(&format!("\"{path}\"")).count(),
            3,
            "{path} must occur only in dispatch, classifier, and classifier tests"
        );
    }
    assert!(
        HEALTH_HANDLER_SRC.contains("crate::crypto::pepper(env).is_ok()")
            && HEALTH_HANDLER_SRC.contains("\"ready\": true")
            && HEALTH_HANDLER_SRC.contains("\"ready\": false")
            && HEALTH_HANDLER_SRC.contains(".with_status(503)"),
        "health must report readiness from the same validated pepper resolver"
    );
    assert!(
        CODLET_SRC.contains("pub async fn issue_token(")
            && CODLET_SRC.contains(") -> Result<String>")
            && !CODLET_SRC.contains("unwrap_or_default")
            && CODLET_SRC.find("crate::crypto::pepper(env)") < CODLET_SRC.find("env.d1(\"DB\")"),
        "codlet issuance must propagate pepper failures without empty substitution"
    );
    assert!(
        LIB_SRC.contains("rejected_configuration_never_invokes_binding_continuation")
            && LIB_SRC.contains("D1 continuation was invoked")
            && LIB_SRC.contains("KV continuation was invoked"),
        "the native pre-binding spy must pin zero D1 and KV continuation access"
    );
}

#[test]
fn rfc077_pepper_codlet_and_auth_caller_inventories_are_closed() {
    assert_eq!(
        LIB_SRC.matches("crypto::pepper(&env)").count(),
        1,
        "main must have exactly one direct protected-request preflight"
    );
    let direct_count: usize = RFC077_DIRECT_PEPPER_CALLERS
        .iter()
        .map(|(name, source, expected)| {
            let actual = source.matches("crate::crypto::pepper(").count();
            assert_eq!(
                actual, *expected,
                "direct pepper caller inventory drifted: {name}"
            );
            actual
        })
        .sum();
    assert_eq!(direct_count, 18, "direct pepper caller total drifted");

    let issue_count: usize = RFC077_CODLET_ISSUE_CALLERS
        .iter()
        .map(|(name, source, expected)| {
            let actual = source.matches("crate::codlet::issue_token(").count();
            assert_eq!(
                actual, *expected,
                "codlet issuance caller inventory drifted: {name}"
            );
            actual
        })
        .sum();
    assert_eq!(issue_count, 29, "codlet issuance caller total drifted");

    assert!(
        SESSION_SRC.contains("pub enum AuthError")
            && SESSION_SRC.contains("AuthError::Unauthenticated")
            && SESSION_SRC.contains("Err(error) => return Err(error.into_worker_error())"),
        "configuration and runtime errors must remain distinguishable from unauthenticated"
    );
    for (name, source) in RFC077_AUTH_HANDLER_SOURCES {
        if source.contains("require_auth_or!") {
            assert!(
                !source.contains("match crate::session::require_auth"),
                "macro-managed handler retained an untyped auth match: {name}"
            );
        } else {
            assert!(
                source.contains("AuthError::Unauthenticated")
                    && source.contains("Err(error) => return Err(error.into_worker_error())")
                    && !source.contains("Err(_) =>"),
                "explicit auth handler is outside the typed propagation inventory: {name}"
            );
        }
    }
}

#[test]
fn rfc077_configuration_and_local_harness_are_pinned() {
    assert_eq!(
        WRANGLER_TOML_SRC
            .matches("required = [\"HMAC_PEPPER\"]")
            .count(),
        4,
        "root, dev, staging, and production must each require HMAC_PEPPER"
    );
    for pattern in [".dev.vars*", ".env*"] {
        assert!(
            GITIGNORE_SRC.lines().any(|line| line.trim() == pattern),
            "local secret files must remain ignored: {pattern}"
        );
    }
    assert!(
        SETUP_SCRIPT_SRC.contains("loadOrCreateLocalPepper")
            && SETUP_SCRIPT_SRC.contains("runDeveloperSetup")
            && SETUP_CORE_SRC.contains("const pepper = await adapter.loadPepper(projectRoot)")
            && LOCAL_SECRET_FILE_SRC.contains("fs.open(path, 'wx', 0o600)")
            && LOCAL_SECRET_FILE_SRC.contains("await handle.chmod(0o600)")
            && !LOCAL_SECRET_FILE_SRC.contains("fs.chmod(path")
            && !LOCAL_SECRET_FILE_SRC.contains("fs.unlink(path")
            && LOCAL_SECRET_FILE_SRC.contains("O_NOFOLLOW")
            && LOCAL_SECRET_FILE_SRC.contains("DOTENV_LINE")
            && LOCAL_SECRET_TEST_SRC.contains("exclusive-create collision was not injected")
            && LOCAL_SECRET_TEST_SRC.contains("replacement path was modified")
            && LOCAL_SECRET_TEST_SRC.contains("injected_close_failure")
            && LOCAL_SECRET_TEST_SRC.contains("injected_close_after_release_failure")
            && LOCAL_SECRET_TEST_SRC.contains("closeAfterReleaseSanitization")
            && LOCAL_SECRET_TEST_SRC.contains("seed HMAC disagreed"),
        "local setup must use handle-safe permissions/cleanup, exact dotenv parsing, and the adversarial matrix"
    );
    assert!(
        LOCAL_SECRET_FILE_SRC.find("if (originalHandle)")
            < LOCAL_SECRET_FILE_SRC.find("reopened = await fs.open")
            && LOCAL_SECRET_FILE_SRC.contains("if (await sanitizeHandle(originalHandle)) return")
            && LOCAL_SECRET_FILE_SRC.contains("await sanitizeHandle(reopened)"),
        "close failure cleanup must fall back from an unusable original handle to an identity-checked non-following reopen"
    );
    assert!(
        ISOLATED_WORKER_TEST_SRC.contains("CLOUDFLARE_LOAD_DEV_VARS_FROM_DOT_ENV: 'false'")
            && ISOLATED_WORKER_TEST_SRC.contains("CLOUDFLARE_INCLUDE_PROCESS_ENV: 'false'")
            && ISOLATED_WORKER_TEST_SRC.contains("const environment = {")
            && ISOLATED_WORKER_TEST_SRC.contains("mkdtemp")
            && ISOLATED_WORKER_TEST_SRC.contains("randomBytes(32)")
            && ISOLATED_WORKER_TEST_SRC
                .contains("cp(join(repositoryRoot, 'workers', 'ssr', 'build')")
            && ISOLATED_WORKER_TEST_SRC
                .contains("const canaryPath = join(container, '.dev.vars.dev')")
            && ISOLATED_WORKER_TEST_SRC.contains("wrangler-child-wrapper.mjs")
            && ISOLATED_WORKER_TEST_SRC.contains("child-environment-keys.json")
            && ISOLATED_WORKER_TEST_SRC.contains("rm(container, { recursive: true"),
        "runtime tests must copy artifacts and use a sanitized, canaried, child-audited fixture"
    );
    assert!(
        PEPPER_CONFIGURATION_TEST_SRC.contains("assert.equal(result.text, unavailableBody)")
            && PEPPER_CONFIGURATION_TEST_SRC.contains("optionalRecoverySecret")
            && PEPPER_CONFIGURATION_TEST_SRC.contains("COMMUNITY_RECOVERY_TOKEN")
            && PEPPER_CONFIGURATION_TEST_SRC.contains("assertChildEnvironmentAudit"),
        "runtime evidence must pin the exact body, optional recovery, and child environment audit"
    );
}

#[test]
fn rfc077_bootstrap_and_executable_gates_remain_safe() {
    assert!(
        BOOTSTRAP_SCRIPT_SRC.contains("runBootstrap")
            && BOOTSTRAP_CORE_SRC.contains("Missing --target; choose staging or production explicitly.")
            && BOOTSTRAP_CORE_SRC.contains("parsed.length !== 1")
            && BOOTSTRAP_CORE_SRC.contains("rejectDuplicateJsonKeys(stdout)")
            && BOOTSTRAP_CORE_SRC.contains("if (keys.has(key)) throw new DuplicateJsonKeyError")
            && BOOTSTRAP_CORE_SRC.contains("--rotate-hmac-pepper")
            && BOOTSTRAP_CORE_SRC.contains("--confirm-rotation")
            && BOOTSTRAP_CORE_SRC.contains("ROTATE ${targetName}")
            && BOOTSTRAP_CORE_SRC.contains("provisioned-not-ready")
            && BOOTSTRAP_CORE_SRC.contains("Keep this Worker dark")
            && BOOTSTRAP_CORE_SRC.contains("sessions, invites, relink/help-signin codes, form tokens, calendar tokens, and recovery codes"),
        "bootstrap must distinguish fresh provisioning from explicit destructive rotation"
    );
    assert!(
        BOOTSTRAP_TEST_SRC.contains("missingTargetStops")
            && BOOTSTRAP_TEST_SRC.contains("strictSingleResultEnvelope")
            && BOOTSTRAP_TEST_SRC.contains("duplicateResultKeysStop")
            && BOOTSTRAP_TEST_SRC.contains("escaped-duplicate-second")
            && BOOTSTRAP_TEST_SRC.contains("escaped-duplicate-first")
            && BOOTSTRAP_TEST_SRC.contains("exactCommandAdapter")
            && BOOTSTRAP_TEST_SRC.contains("fake adapter rejected unexpected SQL"),
        "bootstrap fake adapter must reject unexpected commands and ambiguous result forms"
    );
    assert_eq!(
        BOOTSTRAP_CORE_SRC.matches("'d1_migrations'").count(),
        1,
        "only the D1 migration ledger may be exempt from application freshness checks"
    );
    assert!(
        PEPPER_CONFIGURATION_TEST_SRC.contains("invalidPhase('missing'")
            && PEPPER_CONFIGURATION_TEST_SRC.contains("rotationInvalidates")
            && PEPPER_CONFIGURATION_TEST_SRC.contains("nonMutation"),
        "the isolated runtime gate must cover missing/invalid secrets, mutation safety, and key-change semantics"
    );
    for (name, source) in RFC077_EXECUTABLE_GATE_SOURCES {
        assert!(
            source.contains("prepareIsolatedWorkerTest"),
            "{name} must use the shared isolated Worker fixture"
        );
        assert!(
            !source.contains("dev-pepper-change-in-production") && !source.contains("dev-pepper"),
            "{name} must not embed a legacy pepper sentinel"
        );
    }
    assert!(
        !PACKAGE_JSON_SRC.contains("rfc077") && !PACKAGE_JSON_SRC.contains("rfc-077"),
        "development commands must use domain/role/function names, not RFC numbers"
    );
}

#[test]
fn tracked_wrangler_template_contains_only_placeholder_resource_ids() {
    let mut checked = 0usize;

    for (idx, line) in WRANGLER_TOML_SRC.lines().enumerate() {
        let content = line.split('#').next().unwrap_or("").trim();
        let key = if content.starts_with("database_id") {
            Some("D1 database_id")
        } else if content.starts_with("id") && content.contains('=') {
            Some("KV namespace id")
        } else {
            None
        };

        let Some(key) = key else {
            continue;
        };
        let value = content
            .split_once('=')
            .map(|(_, value)| value.trim().trim_matches('"'))
            .expect("wrangler resource id line must use key = value syntax");
        checked += 1;

        assert!(
            value == "local" || value.starts_with("REPLACE_WITH_"),
            "tracked wrangler.toml line {} contains a real {key} value ({value:?}); \
             keep real hosted D1/KV IDs in ignored wrangler*.local.toml files",
            idx + 1
        );
    }

    assert!(
        checked >= 4,
        "release gate expected to inspect top-level, dev, staging, and production D1 ids \
         (RATE_LIMIT KV was retired by RFC-078; Durable Object bindings have no raw id to check)"
    );
}

#[test]
fn local_wrangler_configs_remain_ignored() {
    let required_patterns = ["wrangler.*.local.toml", "wrangler.local.toml"];

    for pattern in required_patterns {
        assert!(
            GITIGNORE_SRC.lines().any(|line| line.trim() == pattern),
            ".gitignore must keep {pattern:?} so real hosted D1/KV IDs stay out of Git"
        );
    }
}

#[test]
fn rfc065_legacy_migration_does_not_treat_utc_clock_as_local_time() {
    assert!(
        !MIGRATION_0009_SRC.contains("substr(first_day.starts_at_utc")
            && !MIGRATION_0009_SRC.contains("substr(first_day.ends_at_utc"),
        "RFC-065 migration must not backfill local recurrence times from UTC clock text"
    );
    assert!(
        MIGRATION_0009_SRC.contains("future materialization is disabled")
            && MIGRATION_0009_SRC.contains("NULL,\n    NULL,"),
        "RFC-065 legacy series must use explicit null local times when safe local clocks are unavailable"
    );
}

#[test]
fn rfc065_exception_shape_is_checked_by_database() {
    assert!(
        MIGRATION_0009_SRC.contains("action = 'skip' AND event_day_id IS NULL")
            && MIGRATION_0009_SRC.contains("action = 'cancel' AND event_day_id IS NOT NULL"),
        "RFC-065 exception table must enforce skip/cancel event_day_id shape"
    );
}

#[test]
fn rfc065_materialization_uses_after_date_and_shared_request_budget() {
    assert!(
        EVENT_ADMIN_DOMAIN_SRC.contains("generate_recurrence_occurrences_after")
            && EVENT_ADMIN_DOMAIN_SRC.contains("after_day_date"),
        "RFC-065 domain generator must support rolling materialization after existing dates"
    );
    assert!(
        EVENT_SERIES_DB_SRC.contains("let mut remaining = RECURRENCE_MATERIALIZATION_INSERT_CAP")
            && EVENT_SERIES_DB_SRC
                .contains("materialize_series(db, &row, through_day_date, remaining)")
            && EVENT_SERIES_DB_SRC
                .contains("remaining = remaining.saturating_sub(report.inserted)"),
        "RFC-065 community materialization must enforce one shared insert budget per request"
    );
    assert!(
        EVENT_SERIES_DB_SRC.contains("generate_recurrence_occurrences_after")
            && EVENT_SERIES_DB_SRC.contains("previous_materialized"),
        "RFC-065 materializer must generate after materialized_through instead of replaying the first capped batch"
    );
}

/// Count non-comment lines containing `.await` in a source string.
fn count_awaits(src: &str) -> usize {
    src.lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with("//") && t.contains(".await")
        })
        .count()
}

#[test]
fn home_handler_await_count_within_budget() {
    // Home handler awaits: require_auth (session), list_active_for_user (community
    // switcher route), require_membership, home_upcoming, list_active_for_user
    // (switcher), count_active, find_active (community), list_mine_for_days,
    // counts_for_days, list_communities_for_user.  Total ≈ 10-11 DB awaits.
    // Gate: must not exceed 2 × budget.
    let awaits = count_awaits(HOME_HANDLER_SRC);
    assert!(
        awaits <= QUERY_BUDGET_HOME * 2,
        "home.rs has {awaits} .await calls, exceeds 2× budget ({}).\
         Investigate for N+1 regression.",
        QUERY_BUDGET_HOME * 2
    );
}

#[test]
fn event_detail_handler_await_count_within_budget() {
    // Event detail GET awaits: require_auth, require_membership, find_for_community,
    // days_for_event, count_active, find_mine (note), list_for_event (notes),
    // list_all_active (members), find_active (community), list_mine_for_days,
    // counts_for_days (IN), list_for_event_days (IN), issue token (×2 SET_STATUS +
    // SAVE_NOTE), list_communities_for_user.  ~13 DB awaits for the GET handler.
    // The full file also contains POST handlers; total awaits will be higher.
    // Gate: file total must not regress into obviously N+1 territory (> 50).
    let awaits = count_awaits(EVENT_HANDLER_SRC);
    assert!(
        awaits <= 50,
        "event.rs has {awaits} .await calls total across all handlers.\
         Investigate if event detail GET alone exceeds {QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY}."
    );
}

#[test]
fn export_handler_await_count_within_budget() {
    // export.rs contains three handlers (page, JSON download, token/revoke) plus
    // the build_export helper. The per-route budget is 8 flat IN-batched queries.
    // With ~3 handlers + helper, the file-level ceiling is 30 to catch a
    // clear N+1 regression while allowing normal multi-handler structure.
    // The important invariant (batched IN queries, no per-row fetch) is documented
    // in QUERY_BUDGET_EXPORT and enforced via code review; a live harness (RFC-044)
    // will provide the precise per-route assertion when staging is available.
    let awaits = count_awaits(EXPORT_HANDLER_SRC);
    assert!(
        awaits <= 30,
        "export.rs has {awaits} .await calls across all handlers, exceeds ceiling (30).\
         Investigate for N+1 regression — the export route must use batched IN queries."
    );
}

// ── Version-artifact derivation gates (RFC-044 §11 step 1; Handoff 081) ──
//
// Every version-bearing artifact must derive from `[workspace.package]
// version` in Cargo.toml, so a release bumps ONE authority and every check
// below follows automatically — never a hand-edited test literal. Handoff
// 080 (the 0.63.0 release) had to hand-edit a hardcoded cache-buster literal
// to pass; this section is that finding's own package (§4.1 there).
//
// `workspace_version()` is the single parser every gate here calls. Cache
// keys (`sw.js` CACHE_VERSION) carry a `v` prefix; the cache-buster and
// `package.json` do not — that asymmetry is deliberate (§3.4) and each gate
// below adds or omits the prefix explicitly rather than normalising it away.
//
// These tests read source files at test time using include_str! so they
// fire on every `cargo test` run without any external tooling.

const SW_JS_SOURCE: &str = include_str!("../../../workers/ssr/static/sw.js");
const APP_JS_SOURCE: &str = include_str!("../../../workers/ssr/static/app.js");
const APP_CSS_SOURCE: &str = include_str!("../../../workers/ssr/static/app.css");
const SHELL_RS_SOURCE: &str = include_str!("../../../workers/ssr/src/render/shell.rs");
const WORKSPACE_CARGO_TOML: &str = include_str!("../../../Cargo.toml");

/// Extract the version from `[workspace.package]` in the workspace
/// `Cargo.toml`. The single authority every gate in this section derives
/// from. An unparseable authority fails loudly (`.expect`), which is
/// correct in a test — silently defaulting would hide a broken release.
fn workspace_version() -> String {
    let mut in_workspace_pkg = false;
    let mut found = None;
    for line in WORKSPACE_CARGO_TOML.lines() {
        let trimmed = line.trim();
        if trimmed == "[workspace.package]" {
            in_workspace_pkg = true;
            continue;
        }
        if in_workspace_pkg {
            if trimmed.starts_with('[') {
                break; // left the [workspace.package] section
            }
            if trimmed.starts_with("version") {
                // version     = "0.25.0"
                found = trimmed
                    .split_once('=')
                    .map(|(_, v)| v.trim().trim_matches('"').to_owned());
                break;
            }
        }
    }
    found.expect("workspace version not found in Cargo.toml")
}

#[test]
fn sw_cache_version_matches_workspace_version() {
    // Extract CACHE_VERSION from sw.js:  const CACHE_VERSION = 'vX.Y.Z';
    let cache_ver = SW_JS_SOURCE
        .lines()
        .find(|l| l.trim_start().starts_with("const CACHE_VERSION"))
        .and_then(|l| {
            // e.g.  const CACHE_VERSION = 'v0.25.0';
            let after_eq = l.split_once('=')?.1;
            let inner = after_eq
                .trim()
                .trim_start_matches('\'')
                .trim_end_matches(';')
                .trim_end_matches('\'');
            // Strip the leading 'v'
            inner.strip_prefix('v')
        })
        .expect("CACHE_VERSION not found in sw.js");

    let workspace_ver = workspace_version();

    assert_eq!(
        cache_ver, workspace_ver,
        "sw.js CACHE_VERSION 'v{cache_ver}' does not match workspace version '{workspace_ver}'. \
         Update sw.js CACHE_VERSION when bumping the version."
    );
}

/// Handoff 081 §3.2: this assertion used to live inside
/// `rfc056_calendar_page_owns_calendar_and_switcher`, pinned by a hardcoded
/// version literal that had to be hand-edited every release — misfiled next
/// to a gate (above) that solves the same derivation problem correctly.
/// Moved here and derived: a release bumps Cargo.toml and this follows with
/// no test edit.
#[test]
fn cache_buster_matches_workspace_version() {
    let expected = format!("/static/app.js?v={}", workspace_version());
    assert!(
        RENDER_SRC.contains(expected.as_str()) && STATIC_FILES_SRC.contains(expected.as_str()),
        "HTML shell must cache-bust app.js (in both render/shell.rs and \
         handlers/static_files.rs, checked independently) so a same-version switcher fix is \
         not hidden by the service worker. Expected {expected:?} in both files — a release \
         bumps Cargo.toml's workspace version and this follows automatically."
    );
}

/// Handoff 081 §3.3: nothing previously compared `package.json`'s version to
/// the workspace version, so a release could bump one and not the other and
/// every test would still pass. Parses the JSON rather than substring-
/// matching, so a `"version"` key appearing anywhere else in the file could
/// not satisfy this by accident.
#[test]
fn package_json_version_matches_workspace_version() {
    let parsed: serde_json::Value =
        serde_json::from_str(PACKAGE_JSON_SRC).expect("package.json must be valid JSON");
    let package_ver = parsed
        .get("version")
        .and_then(|v| v.as_str())
        .expect("package.json must have a top-level \"version\" string field");
    let workspace_ver = workspace_version();

    assert_eq!(
        package_ver, workspace_ver,
        "package.json \"version\": \"{package_ver}\" does not match workspace version \
         '{workspace_ver}'. Update package.json's version when bumping the version."
    );
}

// ── Cached-asset content-vs-cache-key drift gate (v0.60.0 release) ───────
//
// The check above proves `sw.js` and `Cargo.toml` agree with each other —
// it cannot prove either one is *fresh*, because both can go stale together.
// That is exactly what happened between v0.59.0 and v0.60.0:
// `workers/ssr/static/app.js` gained 48 lines while its cache-buster
// (`render/shell.rs`) and `sw.js`'s `CACHE_VERSION` both stayed at `v0.59.0`.
//
// This gate hashes the concatenated *content* of every cached static asset
// plus the shell template that references it, and pins that hash here. A
// content change with no accompanying update to this pinned hash means the
// version bump (and therefore the cache-buster and `CACHE_VERSION`) was
// forgotten — the gate fails on content drift, not on two numbers merely
// disagreeing with each other.
//
// Updating the pinned hash is a deliberate, one-line acknowledgement that a
// cached asset changed: recompute it (this test's failure message prints
// the actual value) and paste it in, in the same commit that changes the
// asset — mid-cycle, not only at a release. Re-pin whenever content
// changes; the cache key and version move at release, not per package
// (Handoff 040 §7.3 re-pinned this digest with no version bump, and that
// was correct — the prior wording here said otherwise and was wrong).
const RELEASE_CACHE_ASSET_CONTENT_HASH: &str =
    "129a9e88266b4d49146b1ea2ee5e976b3efdd186580f472ca24d1ce86f193f4f";

fn cached_asset_content_hash() -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(APP_JS_SOURCE.as_bytes());
    hasher.update(APP_CSS_SOURCE.as_bytes());
    hasher.update(SHELL_RS_SOURCE.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn cached_asset_content_matches_pinned_hash() {
    let actual = cached_asset_content_hash();
    assert_eq!(
        actual, RELEASE_CACHE_ASSET_CONTENT_HASH,
        "app.js, app.css, or render/shell.rs changed content without updating \
         RELEASE_CACHE_ASSET_CONTENT_HASH. If this is an intentional asset change for \
         this release, update the pinned hash to {actual:?} in the same commit that \
         bumps the version, the app.js cache-buster, and sw.js's CACHE_VERSION — a \
         content change with no version/cache-key move is exactly the v0.59.0 drift \
         this gate exists to catch."
    );
}

// ── Japanese-only rendered-text gate (RFC-049) ───────────────────────────
//
// The pilot ships Japanese UI only. English words leaked into rendered link
// and button text twice in v0.35.x (event-detail "← Home", communities
// "Invite members" / "Manage members"). These were inline literals, not i18n
// constants, so the i18n parity gate did not catch them.
//
// The `_SRC` constants below back several other gates in this file. The
// leak-detection gate itself has moved past them: it used to scan only this
// hand-picked list of eight files for a hand-maintained vocabulary of past
// regressions (`>Word</a>` shapes with a known-bad word list) — which is why
// it never had a chance to catch Handoff 036's six leaks. Five of those six
// were `aria-label` attribute values, a shape the old gate's element-text-only
// vocabulary could not have matched even if it had scanned the right files.
// See `rfc049_no_english_leaks_in_rendered_text_or_attributes` below (near
// `ENGLISH_LEAK_EXCEPTIONS`), which walks every non-test file under
// `handlers/` and `render/` and is default-fail like `LOCALIZATION_EXCEPTIONS`.

const COMMUNITIES_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/communities.rs");
const COMMUNITIES_MATRIX_SRC: &str = concat!(
    include_str!("../../../workers/ssr/src/handlers/communities/matrix.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/matrix/cells.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/matrix/detail.rs")
);
const COMMUNITIES_SRC: &str = concat!(
    include_str!("../../../workers/ssr/src/handlers/communities.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/calendar.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/calendar/events.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/matrix.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/matrix/cells.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/matrix/detail.rs")
);
const COMMUNITY_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/community.rs");
const ADMIN_EVENTS_SRC: &str = concat!(
    include_str!("../../../workers/ssr/src/handlers/admin/events.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/attendance.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/cancel.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/copy.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/create.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/edit.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/forms.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/notes.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/policy.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/recreate.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/summary.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/support.rs"),
);
const ADMIN_EVENTS_COPY_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/events/copy.rs");
const ROLE_TRANSFER_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/role_transfer.rs");
const MEMBER_REMOVE_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/member_remove.rs");
const HELP_SIGNIN_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/help_signin.rs");
const SUSPENSION_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/suspension.rs");
const MIGRATION_0018_SRC: &str = include_str!("../../../migrations/0018_membership_suspension.sql");
const RELINK_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/relink.rs");
const RECOVERY_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/recovery.rs");
const OPERATOR_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/operator.rs");
const MEMBERSHIP_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/membership.rs");
const LOCALE_SRC: &str = include_str!("../src/locale.rs");
const RELINK_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/relink.rs");
const RECOVERY_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/recovery.rs");
const IDENTITY_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/identity.rs");
const RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC: &str =
    include_str!("../../../scripts/recover-community-access.mjs");
const APP_JS_SRC: &str = include_str!("../../../workers/ssr/static/app.js");
const RENDER_SRC: &str = concat!(
    include_str!("../../../workers/ssr/src/render.rs"),
    include_str!("../../../workers/ssr/src/render/errors.rs"),
    include_str!("../../../workers/ssr/src/render/event_card.rs"),
    include_str!("../../../workers/ssr/src/render/nav.rs"),
    include_str!("../../../workers/ssr/src/render/notes.rs"),
    include_str!("../../../workers/ssr/src/render/participants.rs"),
    include_str!("../../../workers/ssr/src/render/shell.rs"),
    include_str!("../../../workers/ssr/src/render/status.rs"),
    include_str!("../../../workers/ssr/src/render/time.rs"),
);
const STATIC_FILES_SRC: &str = include_str!("../../../workers/ssr/src/handlers/static_files.rs");

/// Handoff 036, Handoff 030's `LOCALIZATION_EXCEPTIONS` pattern: a pinned
/// exact leak count and a written reason, asserted exactly (not a ceiling)
/// so a partial edit to an excluded file still fails. `count` is the total
/// number of findings from both `english_leak_element_text_findings` and
/// `english_leak_attr_findings` combined.
struct EnglishLeakException {
    path: &'static str,
    count: usize,
    reason: &'static str,
}

// Pre-seeded with the brand name, the only known-correct English left after
// fixing Handoff 036's six leaks. Anything else this gate finds is a finding
// to report (§5.3/§7.4), not a row to add here.
const ENGLISH_LEAK_EXCEPTIONS: &[EnglishLeakException] = &[
    EnglishLeakException {
        path: "render/shell.rs",
        count: 1,
        reason: "the \"zinnias\" brand name in the <title>, not a translatable UI string",
    },
    EnglishLeakException {
        path: "handlers/static_files.rs",
        count: 1,
        reason: "the \"zinnias\" brand name in the offline page's <title>, not a translatable UI string",
    },
];

/// Rust's `\`-continued string literals join the next line's content (minus
/// leading whitespace) into the same literal — the exact shape that hid
/// Handoff 036's six leaks from a naive single-line search. Collapsing it
/// first lets both finders below treat a wrapped literal as the one string
/// it compiles to.
fn collapse_rust_line_continuations(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'\n') {
            chars.next();
            while matches!(chars.peek(), Some(' ') | Some('\t')) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A contiguous run of 4+ ASCII letters/spaces immediately before `</` —
/// element text sitting right against its closing tag, independent of where
/// the tag it belongs to opened. `{interpolation}` placeholders break the
/// run at `{`, so a bare `{label}</a>` is never flagged; numbers and symbols
/// break it too, so `42</span>` is never flagged.
fn english_leak_element_text_findings(collapsed: &str) -> Vec<String> {
    let chars: Vec<char> = collapsed.chars().collect();
    let mut findings = Vec::new();
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '<' && chars[i + 1] == '/' {
            let mut j = i;
            while j > 0 && (chars[j - 1].is_ascii_alphabetic() || chars[j - 1] == ' ') {
                j -= 1;
            }
            if i - j >= 4 {
                let run: String = chars[j..i].iter().collect();
                if run.chars().any(|c| c.is_ascii_alphabetic()) {
                    findings.push(run.trim().to_string());
                }
            }
        }
        i += 1;
    }
    findings
}

const USER_VISIBLE_ATTRS: &[&str] = &["aria-label", "title", "placeholder", "alt"];

/// Removes every `{...}` span from an attribute value, so a mixed literal
/// like `"Attendance for {name}"` still surfaces its literal half while a
/// pure `"{main_label}"` interpolation does not.
fn strip_placeholders(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            for c2 in chars.by_ref() {
                if c2 == '}' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `after_name` starts right after an attribute-name match, e.g.
/// `="Save"...`. Recognises both the bare `="..."` and `\"`-escaped forms
/// this codebase's `format!` string literals produce. The single-quoted
/// `aria-label='` site in `render/nav.rs::header_with_switcher_next_localized`
/// builds its value across separate `push_str` calls with no literal text
/// between the quotes in the source — it cannot leak a static English string
/// by construction, so single-quoted attributes are intentionally out of
/// scope here.
fn quoted_attr_value(after_name: &str) -> Option<&str> {
    let s = after_name.strip_prefix('=')?;
    let s = s.strip_prefix('\\').unwrap_or(s);
    let s = s.strip_prefix('"')?;
    let end = s.find('"')?;
    let value = &s[..end];
    Some(value.strip_suffix('\\').unwrap_or(value))
}

/// User-visible attribute values (`aria-label`, `title`, `placeholder`,
/// `alt`) whose non-placeholder remainder still contains an ASCII letter
/// run of 3+ characters — the shape of five of Handoff 036's six leaks, and
/// explicitly "not optional" per that handoff (§4/§5.2).
fn english_leak_attr_findings(collapsed: &str) -> Vec<String> {
    let mut findings = Vec::new();
    for attr in USER_VISIBLE_ATTRS {
        for (idx, _) in collapsed.match_indices(attr) {
            let after = &collapsed[idx + attr.len()..];
            let Some(value) = quoted_attr_value(after) else {
                continue;
            };
            let stripped = strip_placeholders(value);
            let max_letter_run = stripped
                .split(|c: char| !c.is_ascii_alphabetic())
                .map(str::len)
                .max()
                .unwrap_or(0);
            if max_letter_run >= 3 {
                findings.push(format!("{attr}=\"{value}\""));
            }
        }
    }
    findings
}

#[test]
fn rfc049_no_english_leaks_in_rendered_text_or_attributes() {
    // Handoff 036: replaces the hand-maintained forbidden-string/file-list
    // gate above with a default-fail walk, the same move Handoff 030 made
    // for LOCALIZATION_EXCEPTIONS — a file this gate doesn't scan, or a leak
    // shape its vocabulary doesn't know about, used to pass silently. Now an
    // unlisted file with a real leak fails, and an unrecognised leak shape
    // (element text or a user-visible attribute) is caught in every file the
    // walk finds, not just eight hand-picked ones.
    let files = handlers_and_render_files();
    let src_dir = workers_ssr_src_dir();
    let mut seen_exception_paths = std::collections::HashSet::new();
    let mut unexpected: Vec<String> = Vec::new();

    for path in &files {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let collapsed = collapse_rust_line_continuations(&content);
        let mut findings = english_leak_element_text_findings(&collapsed);
        findings.extend(english_leak_attr_findings(&collapsed));

        let rel = path
            .strip_prefix(&src_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        match ENGLISH_LEAK_EXCEPTIONS.iter().find(|e| e.path == rel) {
            Some(exc) => {
                seen_exception_paths.insert(exc.path);
                assert_eq!(
                    findings.len(),
                    exc.count,
                    "{rel}: found {} English leak(s) {:?}, pinned exception count is {} \
                     ({}). Re-pin only if this is a deliberate, reviewed change — a partial \
                     edit to an excluded file must not pass silently.",
                    findings.len(),
                    findings,
                    exc.count,
                    exc.reason
                );
            }
            None => {
                if !findings.is_empty() {
                    unexpected.push(format!("{rel}: {findings:?}"));
                }
            }
        }
    }

    assert!(
        unexpected.is_empty(),
        "English text leaked into rendered element text or a user-visible attribute \
         (aria-label/title/placeholder/alt) and is not in ENGLISH_LEAK_EXCEPTIONS:\n{}\n\
         Either localize it with a JA_*/Localized i18n constant, or — if it is a brand \
         name or otherwise correctly not translated — add a table entry with the exact \
         count and a written reason. If this is unexpected, report it — do not silently \
         add a row.",
        unexpected.join("\n")
    );

    for exc in ENGLISH_LEAK_EXCEPTIONS {
        assert!(
            seen_exception_paths.contains(exc.path),
            "ENGLISH_LEAK_EXCEPTIONS names {} but the walk never found or matched it — \
             stale table entry?",
            exc.path
        );
    }
}

/// Handoff 037 §5: same exceptions-table shape as `LOCALIZATION_EXCEPTIONS`/
/// `ENGLISH_LEAK_EXCEPTIONS` (explicit path, stale-entry assertion) — minus
/// a count and a written reason, since a flash code is binary right-or-wrong
/// with no legitimate reason to differ per file (unlike a brand name or an
/// admin-only exception), so there is nothing for a reason to justify. Add
/// both back if a real exception is ever needed. Expected to stay empty.
const FLASH_CODE_EXCEPTIONS: &[&str] = &[];

/// Text before a file's first `#[cfg(test)]` marker — every file in this
/// tree that has an inline test module puts it once, at the end (confirmed
/// by inspection, not assumed), so this reliably excludes test fixture data
/// (e.g. `admin/members.rs`'s own `invite_get_preflight` test, which uses
/// `"flash=Code+revoked"` as unrelated example query data) from production
/// scanning. Files with a separate sibling `tests.rs` never match here since
/// `handlers_and_render_files()` already excludes files named `tests.rs`.
fn production_region(content: &str) -> &str {
    match content.find("#[cfg(test)]") {
        Some(idx) => &content[..idx],
        None => content,
    }
}

/// Drops `//...` line-comment text before scanning — a doc comment
/// mentioning `` `?flash=Code+revoked` `` as an example (exactly what this
/// gate's own source does, a few lines above) is not a redirect literal.
/// No production string literal in this tree contains `//` on the same line
/// as a `flash=` value, so this cannot hide a real site.
fn strip_line_comments(content: &str) -> String {
    content
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A `?flash=`/`&flash=` value that is not a lowercase-snake-case code and
/// not a `{...}` dynamic REF interpolation (the already-correct `me.rs`
/// pattern — dynamic per-request, nothing static to check) is a violation:
/// an uppercase letter, `+`, `%20`, or any other character prose would
/// produce but a code never would.
fn flash_code_violations(production: &str) -> Vec<String> {
    let mut violations = Vec::new();
    for anchor in ["?flash=", "&flash="] {
        for (idx, _) in production.match_indices(anchor) {
            let after = &production[idx + anchor.len()..];
            let end = after.find('"').unwrap_or(after.len());
            let value = &after[..end];
            if value.starts_with('{') {
                continue;
            }
            let is_snake_case = !value.is_empty()
                && value
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if !is_snake_case {
                violations.push(value.to_string());
            }
        }
    }
    violations
}

#[test]
fn rfc072_flash_query_values_are_lowercase_snake_case_codes_not_prose() {
    // Handoff 037 §5: the new English-leak gate scans rendered templates and
    // cannot see this class — the source-level template at each flash site
    // is just `{}`, a bare interpolation placeholder; the English only
    // exists at runtime, in a query string built by the redirecting
    // handler. But the redirect side IS static text
    // (`"...?flash=Note+removed"`), so a gate can catch the whole class at
    // the point a redirect is written, default-fail like its siblings.
    let files = handlers_and_render_files();
    let src_dir = workers_ssr_src_dir();
    let mut seen_exception_paths = std::collections::HashSet::new();
    let mut unexpected: Vec<String> = Vec::new();

    for path in &files {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let stripped = strip_line_comments(&content);
        let violations = flash_code_violations(production_region(&stripped));

        let rel = path
            .strip_prefix(&src_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        match FLASH_CODE_EXCEPTIONS.iter().find(|&&p| p == rel) {
            Some(&path) => {
                seen_exception_paths.insert(path);
            }
            None => {
                if !violations.is_empty() {
                    unexpected.push(format!("{rel}: {violations:?}"));
                }
            }
        }
    }

    assert!(
        unexpected.is_empty(),
        "?flash= query value is not a lowercase snake_case code (or is missing entirely) and \
         is not in FLASH_CODE_EXCEPTIONS:\n{}\n\
         Prose in a flash query string is exactly how English leaked into rendered flash text \
         (Handoff 037) — map it through a per-surface code mapper (see \
         `calendar_flash_message`/`note_flash_message`) instead.",
        unexpected.join("\n")
    );

    for &exc_path in FLASH_CODE_EXCEPTIONS {
        assert!(
            seen_exception_paths.contains(exc_path),
            "FLASH_CODE_EXCEPTIONS names {exc_path} but the walk never found or matched it — \
             stale table entry?"
        );
    }
}

#[test]
fn rfc061_member_management_is_discoverable_from_admin_workflows() {
    assert!(
        HOME_HANDLER_SRC.contains("/c/{cid}/admin/members")
            && HOME_HANDLER_SRC.contains("i18n::HOME_MANAGE_MEMBERS") // RFC-072 locale-aware accessor
            && !HOME_HANDLER_SRC.contains("invite_label = i18n::t(locale, i18n::HOME_INVITE_MEMBERS)")
            && !HOME_HANDLER_SRC.contains("invite_label = i18n::JA_HOME_INVITE_MEMBERS"),
        "RFC-061 Home admin shortcut must lead to member management, not directly to invite codes"
    );
    assert!(
        ME_HANDLER_SRC.contains("i18n::ME_SECTION_ADMIN") // RFC-072 locale-aware accessor
            && ME_HANDLER_SRC.contains("i18n::ME_MANAGE_MEMBERS") // RFC-072 locale-aware accessor
            && ME_HANDLER_SRC.contains("/c/{cid}/admin/members")
            && ME_HANDLER_SRC.contains("/c/{cid}/admin/export"),
        "RFC-061 Me page must expose admin tools with member management and export"
    );
    assert!(
        MEMBERS_HANDLER_SRC.contains("i18n::ADMIN_INVITES_BACK_TO_MEMBERS") // RFC-072/Handoff 072 locale-aware accessor
            && MEMBERS_HANDLER_SRC.contains("i18n::ADMIN_MEMBERS_GENERATE_INVITE") // RFC-072/Handoff 072 locale-aware accessor
            && MEMBERS_HANDLER_SRC.contains("i18n::ADMIN_MEMBERS_CURRENT_USER") // RFC-072/Handoff 072 locale-aware accessor
            && !MEMBERS_HANDLER_SRC.contains("Generate invite code</a>"),
        "RFC-061 members/invites pages must use reviewed copy and link invites back to members"
    );
}

#[test]
fn rfc061_admin_switch_targets_require_admin_role() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("fn is_admin_target")
            && COMMUNITY_HANDLER_SRC.contains("m.role == \"admin\"")
            && COMMUNITY_HANDLER_SRC.contains("Some(\"admin_members\") if is_admin_target")
            && COMMUNITY_HANDLER_SRC.contains("Some(\"admin_invites\") if is_admin_target")
            && COMMUNITY_HANDLER_SRC.contains("Some(\"admin_events_new\") if is_admin_target")
            && MEMBERS_HANDLER_SRC.contains("\"admin_members\"")
            && MEMBERS_HANDLER_SRC.contains("\"admin_invites\""),
        "RFC-061 admin switch targets must preserve admin pages only for target communities where the user is admin"
    );
}

#[test]
fn rfc062_role_transfer_uses_guarded_member_management_flow() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("\"promote\"")
            && COMMUNITY_HANDLER_SRC.contains("get_promote_member")
            && COMMUNITY_HANDLER_SRC.contains("post_promote_member")
            && COMMUNITY_HANDLER_SRC.contains("\"demote\"")
            && COMMUNITY_HANDLER_SRC.contains("get_demote_member")
            && COMMUNITY_HANDLER_SRC.contains("post_demote_member"),
        "RFC-062 promote/demote routes must be registered explicitly"
    );
    assert!(
        ROLE_TRANSFER_HANDLER_SRC.contains("token_purpose::PROMOTE_MEMBER")
            && ROLE_TRANSFER_HANDLER_SRC.contains("token_purpose::DEMOTE_MEMBER")
            && ROLE_TRANSFER_HANDLER_SRC.contains("i18n::ADMIN_PROMOTE_ACTION") // RFC-072/Handoff 072 locale-aware accessor
            && ROLE_TRANSFER_HANDLER_SRC.contains("i18n::ADMIN_DEMOTE_ACTION") // RFC-072/Handoff 072 locale-aware accessor
            && ROLE_TRANSFER_HANDLER_SRC
                .contains("target_membership_id == membership.membership_id"),
        "RFC-062 handlers must use dedicated token purposes, reviewed copy, and server-side self-target denial"
    );
    assert!(
        MEMBERSHIP_DB_SRC.contains("AuditAction::MembershipPromotedToAdmin")
            && MEMBERSHIP_DB_SRC.contains("AuditAction::MembershipDemotedToMember")
            && MEMBERSHIP_DB_SRC.matches("AuditMetadata::None").count() >= 2,
        "RFC-062 role changes must audit direction by action name without extra metadata"
    );
}

#[test]
fn rfc062_role_transfer_writes_are_scoped_and_guarded() {
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn promote_to_admin_required")
            && MEMBERSHIP_DB_SRC.contains("SET role = 'admin'")
            && MEMBERSHIP_DB_SRC.contains("id = ?1")
            && MEMBERSHIP_DB_SRC.contains("community_id = ?2")
            && MEMBERSHIP_DB_SRC.contains("MEMBERSHIP_ACTIVE")
            && MEMBERSHIP_DB_SRC.contains("role = 'member'"),
        "RFC-062 promote update must be scoped by membership id, community id, active membership, and current role"
    );
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn demote_to_member_required")
            && MEMBERSHIP_DB_SRC.contains("SET role = 'member'")
            && MEMBERSHIP_DB_SRC.contains("role = 'admin'")
            && MEMBERSHIP_DB_SRC.contains("SELECT COUNT(*) FROM community_memberships")
            && MEMBERSHIP_DB_SRC.contains("> 1"),
        "RFC-062 demote update must re-check active admin count in the conditional write"
    );
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn soft_remove_guarded_required")
            && MEMBERSHIP_DB_SRC.contains("role != 'admin'")
            && MEMBERSHIP_DB_SRC.contains("SELECT COUNT(*) FROM community_memberships")
            && MEMBER_REMOVE_HANDLER_SRC.contains("soft_remove_guarded")
            && !MEMBER_REMOVE_HANDLER_SRC.contains("soft_remove(&db, target_membership_id"),
        "RFC-062 must retrofit member removal to use the guarded admin-count-preserving update"
    );
}

#[test]
fn rfc062_admin_invites_remain_member_role_only() {
    let insert_start = MEMBERS_HANDLER_SRC
        .find("invite_db::insert_required(")
        .expect("invite insert call should exist");
    let insert_end = MEMBERS_HANDLER_SRC[insert_start..]
        .find(".await?;")
        .map(|offset| insert_start + offset)
        .expect("invite insert await should exist");
    let invite_insert = &MEMBERS_HANDLER_SRC[insert_start..insert_end];
    assert!(
        invite_insert.contains("\"member\",") && !invite_insert.contains("\"admin\","),
        "RFC-062 keeps admin-granting invite codes out of the UI; generated invites must grant member role"
    );
}

// Handoff 082 (F1 of the RFC-054 Slice 2 review): a copy gate matching its
// own prose, the adjacent failure to a gate matching *source* prose (seven
// prior instances). The idiom from here down: **behaviour is pinned
// exactly** (a duration, a count, a route path — `RELINK_CODE_TTL_SECONDS`
// below stays `assert_eq!`); **copy is pinned by property** — what a
// message must convey, or that two places agree with each other, never
// what characters spell it. A copy constant is never `assert_eq!`'d to a
// language literal; a rewording that keeps the property true must keep
// passing, and one that breaks the property it protects must still fail.
// Line 1729's `JA_JOIN_RELINK_LINK == JA_ADMIN_HELP_SIGNIN_RELINK_LINK`
// already demonstrated the consistency form of this: it pins that two
// places agree without caring what they say.
#[test]
fn rfc063_removal_only_policy_is_locked() {
    use zinnias_ciao_contracts::i18n::*;

    // JA_ADMIN_REMOVE_CONFIRM: examined for a property beyond its exact
    // words and none survives — it is a two-word confirm-button label with
    // no separable claim to verify (unlike the consequence copy below,
    // which states specific, checkable facts). Left exact, deliberately.
    assert_eq!(JA_ADMIN_REMOVE_CONFIRM, "メンバーから外す");
    assert!(
        JA_ADMIN_REMOVE_CONSEQUENCE.contains("できなくなります")
            && EN_ADMIN_REMOVE_CONSEQUENCE
                .to_ascii_lowercase()
                .contains("no longer"),
        "RFC-063 removal copy must say access ends, in both locales"
    );
    // The stem 残り (not the fully-conjugated 残ります) tolerates both a
    // terminal sentence ("…残ります。") and a continuative clause
    // ("…残り、…") joining into the next one — the exact form slice 2's
    // first draft used, which the old assert_eq!-adjacent literal check
    // rejected even though it stated the same fact. Guarded against a
    // negated false-positive (残りません, "do NOT remain") explicitly,
    // since the stem alone cannot distinguish affirmation from negation.
    assert!(
        JA_ADMIN_REMOVE_CONSEQUENCE.contains("残り")
            && !JA_ADMIN_REMOVE_CONSEQUENCE.contains("残りません")
            && EN_ADMIN_REMOVE_CONSEQUENCE
                .to_ascii_lowercase()
                .contains("remain"),
        "RFC-063 removal copy must say past records remain, in both locales"
    );
    // Handoff 054 Slice 2 added this: the single most important fact the
    // message states, since silence here is exactly what slice 2's own A1
    // finding was about (an irreversible action reading as reversible).
    // Pinned as its own property rather than folded into the two above, so
    // a future edit cannot drop it without a visible, separately-named
    // failure — but not extended to every sentence slice 2 added (the
    // other-communities and re-invitation clauses stay unpinned, per this
    // handoff's own warning against recreating the problem one sentence at
    // a time).
    assert!(
        JA_ADMIN_REMOVE_CONSEQUENCE.contains("取り消せません")
            && EN_ADMIN_REMOVE_CONSEQUENCE
                .to_ascii_lowercase()
                .contains("cannot be undone"),
        "RFC-063 removal copy must say the action cannot be undone, in both locales \
         (Handoff 054 Slice 2 A1 — removal is one-way, unlike suspension)"
    );

    // RFC-082 (Handoff 058) amends RFC-063: it explicitly adds a reversible
    // `suspend`/`unsuspend` state alongside the still-terminal `removed_at`,
    // so "suspend" is dropped from the forbidden list here — it is no
    // longer an undocumented escape hatch, it is the reviewed feature.
    // `removed_at` itself remains one-way: RFC-082 §1's own transition
    // table refuses every `removed → anything` transition, so "reactivate"
    // and "restore" (un-removing) stay locked out.
    for (label, src) in [
        ("members handler", MEMBERS_HANDLER_SRC),
        ("member remove handler", MEMBER_REMOVE_HANDLER_SRC),
        ("role transfer handler", ROLE_TRANSFER_HANDLER_SRC),
        ("community router", COMMUNITY_HANDLER_SRC),
    ] {
        let lowered = src.to_ascii_lowercase();
        for forbidden in ["reactivate", "restore"] {
            assert!(
                !lowered.contains(forbidden),
                "RFC-063 removal must stay terminal — {forbidden:?} must not appear in {label}"
            );
        }
    }
}

#[test]
fn rfc063_readd_uses_new_identity_without_display_name_merge() {
    assert!(
        JOIN_HANDLER_SRC.contains("let user_id = crate::crypto::random_token();")
            && JOIN_HANDLER_SRC.contains("let membership_id = crate::crypto::random_token();")
            && JOIN_HANDLER_SRC.contains("crate::db::invite::redeem_required(")
            && INVITE_DB_SRC.contains("INSERT INTO users")
            && INVITE_DB_SRC.contains("INSERT INTO community_memberships"),
        "RFC-063 Option A requires invite redemption to create a fresh user and membership"
    );
    assert!(
        !JOIN_HANDLER_SRC.contains("WHERE display_name")
            && !JOIN_HANDLER_SRC.contains("display_name = ?")
            && !JOIN_HANDLER_SRC.contains("find_by_display_name"),
        "RFC-063 must not re-add or merge memberships by display name"
    );
}

#[test]
fn rfc063_active_member_queries_exclude_removed_members() {
    let list_start = MEMBERSHIP_DB_SRC
        .find("pub async fn list_all_active")
        .expect("list_all_active should exist");
    let list_end = MEMBERSHIP_DB_SRC[list_start..]
        .find("pub async fn find_active_summary")
        .map(|offset| list_start + offset)
        .expect("find_active_summary should follow list_all_active");
    let list_all_active = &MEMBERSHIP_DB_SRC[list_start..list_end];
    assert!(
        list_all_active.contains("MEMBERSHIP_ACTIVE"),
        "RFC-063 active member list must exclude removed memberships"
    );

    let find_start = MEMBERSHIP_DB_SRC
        .find("pub async fn find_active(")
        .expect("find_active should exist");
    let find_end = MEMBERSHIP_DB_SRC[find_start..]
        .find("pub async fn find_active_by_id")
        .map(|offset| find_start + offset)
        .expect("find_active_by_id should follow find_active");
    let find_active = &MEMBERSHIP_DB_SRC[find_start..find_end];
    assert!(
        find_active.contains("MEMBERSHIP_ACTIVE"),
        "RFC-063 active authorization lookup must exclude removed memberships"
    );
}

#[test]
fn rfc024_help_signin_copy_and_ttl_are_locked() {
    use zinnias_ciao_contracts::i18n::*;

    // Behaviour stays exact — a duration, not copy.
    assert_eq!(RELINK_CODE_TTL_SECONDS, 15 * 60);

    // JA_ADMIN_HELP_SIGNIN_ACTION / EN_ADMIN_HELP_SIGNIN_ACTION: the label
    // must describe helping a MEMBER sign in, not creating an invite — the
    // exact confusion RFC-024 exists to prevent. Property, not wording: any
    // phrasing keeping "sign in" and dropping "invite" passes.
    assert!(
        JA_ADMIN_HELP_SIGNIN_ACTION.contains("サインイン")
            && !JA_ADMIN_HELP_SIGNIN_ACTION.contains("招待")
            && EN_ADMIN_HELP_SIGNIN_ACTION
                .to_ascii_lowercase()
                .contains("sign in")
            && !EN_ADMIN_HELP_SIGNIN_ACTION
                .to_ascii_lowercase()
                .contains("invite"),
        "RFC-024 help-signin action label must describe helping a member sign in, \
         and must not read as inviting a new one, in both locales"
    );
    assert!(
        JA_ADMIN_HELP_SIGNIN_RELINK_HINT.contains("招待コード欄では使えません。")
            // JA_ADMIN_HELP_SIGNIN_RELINK_LINK: must describe opening the
            // re-sign-in screen — property, not wording. Cross-consistency
            // with /join's copy of this same link is pinned separately
            // below (JA_JOIN_RELINK_LINK == JA_ADMIN_HELP_SIGNIN_RELINK_LINK),
            // wording-agnostic on both sides.
            && JA_ADMIN_HELP_SIGNIN_RELINK_LINK.contains("サインイン")
            && JA_ADMIN_HELP_SIGNIN_RELINK_LINK.contains("開く")
            && HELP_SIGNIN_HANDLER_SRC.contains("href=\\\"/relink\\\"")
            && JOIN_HANDLER_SRC.contains("href=\\\"/relink\\\"")
            && RENDER_SRC.contains("href=\\\"/relink\\\"")
            && HELP_SIGNIN_HANDLER_SRC.contains("data-copy-code-button")
            && HELP_SIGNIN_HANDLER_SRC.contains("data-copy-code-value")
            && APP_JS_SRC.contains("navigator.clipboard.writeText")
            && JA_JOIN_RELINK_LINK == JA_ADMIN_HELP_SIGNIN_RELINK_LINK
            && HELP_SIGNIN_HANDLER_SRC.contains("rel=\\\"noopener\\\""),
        "RFC-024 help-signin result must direct members to /relink and distinguish it from invite-code entry"
    );

    // JA_RELINK_INVALID / EN_RELINK_INVALID: two properties, both from
    // slice 1's A1 review, not the sentence itself.
    //
    // Property 1 (RFC-081 §3.2 genericness): a member — or an attacker
    // holding a guessed code — must not learn which way a code failed.
    // Enforced structurally rather than by content: every relink-failure
    // branch (form replay, abuse-blocked, abuse-unavailable, no valid code,
    // and the redemption-race branch) must route through this SAME
    // constant, never a cause-specific one. A wording change that keeps
    // every branch sharing one constant still passes; a branch given its
    // own message would shrink this count and must fail.
    let relink_invalid_call_sites = RELINK_HANDLER_SRC.matches("i18n::RELINK_INVALID").count();
    assert!(
        relink_invalid_call_sites >= 5,
        "RFC-081 §3.2: expected at least 5 relink failure branches (form replay, \
         abuse-blocked, abuse-unavailable, no valid code, redemption-race) sharing \
         i18n::RELINK_INVALID; found {relink_invalid_call_sites} — a shrinking count \
         likely means a branch was given its own, more specific message, which would \
         leak which failure occurred"
    );
    // Property 2 (why it is allowed to mention expiry): relink codes
    // genuinely expire — RELINK_CODE_TTL_SECONDS, asserted exact above, is
    // a real positive duration — unlike recovery credentials, which never
    // do (slice 1 changed JA_RECOVERY_INVALID for exactly that reason and
    // deliberately left this constant alone). Mentioning expiry here does
    // not violate property 1: the message still does not say *whether*
    // expiry is what actually happened, only that it is one honest
    // possibility among the causes this constant covers.
    assert!(
        JA_RELINK_INVALID.contains("有効期限")
            && EN_RELINK_INVALID.to_ascii_lowercase().contains("expired"),
        "RFC-081/slice 1: the relink-invalid message may and should mention expiry, \
         since relink codes genuinely expire (RELINK_CODE_TTL_SECONDS) — unlike \
         recovery credentials"
    );

    // RFC-082 (Handoff 058) legitimately adds "suspend"/"unsuspend" routes to
    // the shared community router — dropped from the forbidden list here for
    // the same reason `rfc063_removal_only_policy_is_locked` drops it above.
    // `removed_at` stays terminal, so "reactivate"/"restore" stay locked.
    for (label, src) in [
        ("help-signin handler", HELP_SIGNIN_HANDLER_SRC),
        ("relink handler", RELINK_HANDLER_SRC),
        ("community router", COMMUNITY_HANDLER_SRC),
    ] {
        let lowered = src.to_ascii_lowercase();
        for forbidden in ["reactivate", "restore"] {
            assert!(
                !lowered.contains(forbidden),
                "RFC-024 help-signin surface must not expose {forbidden:?} in {label}"
            );
        }
    }
}

#[test]
fn rfc024_relink_codes_are_membership_scoped_hmacs() {
    assert!(
        RELINK_DB_SRC.contains("membership_relink_codes")
            && RELINK_DB_SRC.contains("code_hmac")
            && RELINK_DB_SRC.contains("community_id")
            && RELINK_DB_SRC.contains("membership_id")
            && RELINK_DB_SRC.contains("created_by_membership_id"),
        "RFC-024 relink code table access must keep HMAC code, community, target membership, and creator membership fields"
    );
    assert!(
        RELINK_DB_SRC.contains("HMAC")
            || HELP_SIGNIN_HANDLER_SRC.contains("hmac_hex(&crate::crypto::pepper(env)")
            || HELP_SIGNIN_HANDLER_SRC.contains("hmac_hex(&pepper")
            || HELP_SIGNIN_HANDLER_SRC.contains("hmac_hex(pepper.as_str()"),
        "RFC-024 codes must be HMAC hashed before storage"
    );
    assert!(
        RELINK_DB_SRC.contains("pub async fn issue_required")
            && RELINK_DB_SRC.contains("revoked_at=?1")
            && HELP_SIGNIN_HANDLER_SRC.contains("relink_db::issue_required"),
        "RFC-024 must revoke prior unused codes when creating a new code for the same membership"
    );
}

#[test]
fn rfc024_redemption_rechecks_active_membership_and_community() {
    assert!(
        RELINK_DB_SRC.contains("JOIN community_memberships m ON m.id = r.membership_id")
            && RELINK_DB_SRC.contains("MEMBERSHIP_ACTIVE")
            && RELINK_DB_SRC.contains("m.community_id = r.community_id")
            && RELINK_DB_SRC.contains("m.user_id"),
        "RFC-024 redemption must resolve membership_id to user_id and re-check active community membership"
    );
    assert!(
        !RELINK_DB_SRC.contains("display_name")
            && !RELINK_HANDLER_SRC.contains("display_name")
            && !HELP_SIGNIN_HANDLER_SRC.contains("WHERE display_name"),
        "RFC-024 must not recover or merge by display name"
    );
    assert!(
        JOIN_HANDLER_SRC.contains("let user_id = crate::crypto::random_token();")
            && JOIN_HANDLER_SRC.contains("let membership_id = crate::crypto::random_token();")
            && INVITE_DB_SRC.contains("INSERT INTO community_memberships"),
        "RFC-024 invite-era help-signin relies on join minting a fresh user_id and membership per invite redemption"
    );
}

/// Handoff 057 §6 gate 2: the anonymous recovery-credential consumption
/// route is abuse-limited, and reserved *before* any credential lookup —
/// a stop condition per §5.2 / §10, since this route authenticates an
/// entire account and the credential it guards carries no expiry.
/// Default-fail: asserts the reserve call's own source position precedes
/// the credential lookup's, not merely that both strings appear somewhere
/// in the file — presence alone would pass even if the lookup ran first
/// and the limiter were reserved only afterward as an afterthought.
#[test]
fn rfc081_recovery_route_is_abuse_limited_before_any_credential_lookup() {
    // Comments stripped first — this file's own module doc comment
    // mentions `abuse_control::reserve` in prose near the top, which
    // would otherwise make the position check pass regardless of where
    // the *real* call sits (discovered while proving this gate fires:
    // reordering the real call after the lookup still passed, because
    // the doc-comment mention is always earliest in the file).
    let production = strip_line_comments(RECOVERY_HANDLER_SRC);
    let reserve_pos = production
        .find("abuse_control::reserve")
        .expect("post_recovery must call abuse_control::reserve");
    let lookup_pos = production
        .find("recovery_db::find_valid_by_hmac")
        .expect("post_recovery must look up the credential by its HMAC");
    assert!(
        reserve_pos < lookup_pos,
        "abuse_control::reserve must be called before the first credential lookup, not after — \
         found the reserve call at byte {reserve_pos} and the lookup at byte {lookup_pos}"
    );
    assert!(
        RECOVERY_HANDLER_SRC.contains("Scope::Recovery"),
        "the recovery route must use its own Scope::Recovery, never a scope shared with /relink"
    );
    assert!(
        !RECOVERY_HANDLER_SRC.contains("Scope::Relink"),
        "the recovery route must not reuse /relink's Scope — sharing a budget lets one flow \
         starve the other (Handoff 057 §5.2)"
    );
    assert!(
        RECOVERY_HANDLER_SRC.contains("i18n::RECOVERY_INVALID") // RFC-072/Handoff 075 locale-aware accessor
            && !RECOVERY_HANDLER_SRC.contains("already used")
            && !RECOVERY_HANDLER_SRC.contains("revoked")
            && !RECOVERY_HANDLER_SRC.contains("expired"),
        "recovery consumption failures must use one generic error, never a distinct message for \
         unknown/consumed/revoked/expired"
    );
}

/// Handoff 057 §5.1 / §7: the recovery credential's own required-test
/// list — HMAC at rest (never a raw code column), single-use consumption,
/// and regeneration revoking whatever was previously active in the same
/// batch as the new insert. Same shape as
/// `rfc024_relink_codes_are_membership_scoped_hmacs`.
#[test]
fn rfc081_recovery_credentials_are_hmac_only_and_regeneration_revokes_previous() {
    assert!(
        RECOVERY_DB_SRC.contains("account_recovery_credentials")
            && RECOVERY_DB_SRC.contains("code_hmac"),
        "recovery credential table access must reference the HMAC-shaped code_hmac column"
    );
    assert!(
        MIGRATION_0017_SRC.contains("account_recovery_credentials")
            && MIGRATION_0017_SRC.contains("code_hmac")
            && !MIGRATION_0017_SRC.contains("code TEXT"),
        "migration 0017 must define code_hmac only — no separate raw/plaintext code column"
    );
    assert!(
        RECOVERY_HANDLER_SRC.contains("hmac_hex(pepper.as_str()")
            || RECOVERY_HANDLER_SRC.contains("hmac_hex(pepper"),
        "the anonymous consumption route must hash the submitted code before any lookup"
    );
    // Deliberately the *raw* source here, not `compact_brace_block`'s
    // output — that helper collapses all whitespace (joining tokens with
    // nothing between them), which would turn `"revoked_at = ?1"` into an
    // unmatchable `"revoked_at=?1"` (the exact trap Handoff 056's own
    // gate work hit and documented).
    let regenerate_start = RECOVERY_DB_SRC
        .find("pub async fn regenerate_required")
        .expect("db/recovery.rs must define regenerate_required");
    let regenerate = &RECOVERY_DB_SRC[regenerate_start..];
    assert!(
        regenerate.contains("revoked_at = ?1")
            && regenerate.contains("consumed_at IS NULL AND revoked_at IS NULL")
            && regenerate.contains("execute_required_tail"),
        "regenerate_required must revoke whatever credential was previously active in the same \
         batch as the new insert, so a member can never hold two"
    );
    assert!(
        RECOVERY_DB_SRC.contains("pub async fn consume_required")
            && RECOVERY_DB_SRC.contains("consumed_at IS NULL AND revoked_at IS NULL")
            && RECOVERY_HANDLER_SRC.contains("recovery_db::consume_required"),
        "consumption must mark the credential consumed with a conditional single-use update"
    );
}

/// Handoff 057 §5.1 / §7 / §9: "No admin-facing recovery operation of any
/// kind" — RFC-081 §2's community-admin-authority boundary applies to
/// every account-level credential operation this package adds, the same
/// way it already applies to linking. Default-fail: scans every `.rs`
/// file under `handlers/admin/` for any reference to the recovery or
/// unlink machinery.
#[test]
fn rfc081_no_admin_surface_reaches_any_recovery_operation() {
    let admin_dir = workers_ssr_src_dir().join("handlers/admin");
    let mut files = Vec::new();
    walk_rs_files(&admin_dir, &mut files);
    assert!(
        files.len() > 3,
        "expected several .rs files under handlers/admin/, found only {} — directory walk is \
         probably broken, not the codebase actually shrinking",
        files.len()
    );

    let forbidden = [
        "db::recovery",
        "recovery_db",
        "unlink_required",
        "REGENERATE_RECOVERY",
        "REDEEM_RECOVERY",
        "UNLINK_IDENTITY",
        "account_recovery_credentials",
    ];
    let mut offenders: Vec<String> = Vec::new();
    let src_dir = workers_ssr_src_dir();
    for path in &files {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let rel = path
            .strip_prefix(&src_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for pattern in forbidden {
            if content.contains(pattern) {
                offenders.push(format!("{rel}: {pattern}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "an admin-surface file references recovery/unlink machinery: {} — RFC-081 §2 / Handoff \
         057 §5.1 forbids any admin path to an account-level credential operation, the same \
         boundary linking already respects",
        offenders.join(", ")
    );
}

#[test]
fn rfc024_redemption_is_single_use_generic_and_revokes_old_sessions() {
    assert!(
        RELINK_DB_SRC.contains("pub async fn redeem_required")
            && RELINK_DB_SRC.contains("used_at IS NULL")
            && RELINK_HANDLER_SRC.contains("relink_db::redeem_required"),
        "RFC-024 redemption must mark codes used with a conditional single-use update"
    );
    assert!(
        RELINK_HANDLER_SRC.contains("i18n::RELINK_INVALID") // RFC-072/Handoff 075 locale-aware accessor
            && !RELINK_HANDLER_SRC.contains("already used")
            && !RELINK_HANDLER_SRC.contains("wrong community"),
        "RFC-024 redemption failures must use one generic error"
    );
    assert!(
        RELINK_DB_SRC.contains("UPDATE sessions SET revoked_at=?1")
            && RELINK_DB_SRC.contains("id!=?3")
            && RELINK_DB_SRC.contains("EXISTS (SELECT 1 FROM sessions keep"),
        "RFC-024 redemption must revoke other active sessions for the target user after inserting the new session"
    );
    assert!(
        RELINK_HANDLER_SRC.contains("abuse_control::reserve")
            && RELINK_HANDLER_SRC.contains("abuse_control::reset")
            && !RELINK_HANDLER_SRC.contains("write_legacy")
            && RELINK_DB_SRC.contains("AuditAction::MembershipRelinkRedeemed"),
        "RFC-024 failed redemption should be fail-closed rate-limited (RFC-078), not audited as a membership event"
    );
}

#[test]
fn rfc069_operator_recovery_is_disabled_and_secret_authorized_by_default() {
    assert!(
        LIB_SRC.contains("(Method::Post, \"/operator/recovery/community-access\")"),
        "RFC-069 operator recovery must use the reviewed POST-only route"
    );
    assert!(
        WRANGLER_TOML_SRC.contains("COMMUNITY_RECOVERY_ENABLED = \"false\"")
            && !WRANGLER_TOML_SRC.contains("COMMUNITY_RECOVERY_ENABLED = \"true\""),
        "Tracked wrangler.toml must keep RFC-069 recovery disabled by default"
    );
    assert!(
        !WRANGLER_TOML_SRC.contains("COMMUNITY_RECOVERY_TOKEN ="),
        "Tracked wrangler.toml must not contain the operator recovery bearer token"
    );
    assert!(
        OPERATOR_HANDLER_SRC.contains("env.secret(\"COMMUNITY_RECOVERY_TOKEN\")")
            && !OPERATOR_HANDLER_SRC.contains("env.var(\"COMMUNITY_RECOVERY_TOKEN\")")
            && OPERATOR_HANDLER_SRC.contains("constant_time_eq")
            && OPERATOR_HANDLER_SRC.contains("Authorization")
            && OPERATOR_HANDLER_SRC.contains("\"Bearer \""),
        "RFC-069 endpoint must read the token only as a secret and compare the bearer in constant time"
    );
}

#[test]
fn rfc069_operator_recovery_targets_existing_active_admins_only() {
    assert!(
        OPERATOR_HANDLER_SRC.contains("community_db::find_active")
            && OPERATOR_HANDLER_SRC.contains("membership_db::find_active_by_id")
            && OPERATOR_HANDLER_SRC.contains("target.role == \"admin\"")
            && OPERATOR_HANDLER_SRC.contains("render::not_found()"),
        "RFC-069 endpoint must converge invalid community/membership/non-admin cases on generic not-found"
    );
    assert!(
        OPERATOR_HANDLER_SRC.contains("relink_db::issue_required")
            && RELINK_DB_SRC.contains("INSERT INTO membership_relink_codes")
            && RELINK_DB_SRC.contains("audit::required_record")
            && RELINK_DB_SRC.contains("audit::execute_required_tail")
            && !OPERATOR_HANDLER_SRC.contains("let _ = audit::write")
            && !OPERATOR_HANDLER_SRC.contains("audit::write("),
        "RFC-069 endpoint must batch relink-code mutation with required audit evidence and must not discard audit errors"
    );
}

#[test]
fn rfc069_operator_recovery_audit_correlates_creation_and_redemption() {
    assert!(
        RELINK_DB_SRC.contains("AuditAction::OperatorRecoveryAdminRelinkCreated")
            && RELINK_DB_SRC.contains("AuditMetadata::OperatorRecovery")
            && OPERATOR_HANDLER_SRC.contains("Some(body.operator_label)")
            && RELINK_DB_SRC.contains("relink_code_id: id.to_owned()")
            && !RELINK_DB_SRC.contains("let metadata = serde_json::json!"),
        "RFC-069 creation audit must include bounded operator label and relink correlation metadata"
    );
    assert!(
        RELINK_DB_SRC.contains("AuditAction::MembershipRelinkRedeemed")
            && RELINK_DB_SRC.contains("relink_code_id: target.id.clone()"),
        "RFC-069 redemption audit must include the relink_code_id created by the operator endpoint"
    );
}

#[test]
fn rfc069_operator_tool_requires_explicit_target_and_production_confirmation() {
    assert!(
        RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("COMMUNITY_RECOVERY_TOKEN")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("process.env.COMMUNITY_RECOVERY_TOKEN")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--target")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--url")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--community-id")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--admin-membership-id")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--operator-label"),
        "RFC-069 operator tool must require explicit environment, URL, target IDs, label, and env token"
    );
    assert!(
        RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--confirm-production")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("Type \"production\"")
            && !RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("writeFile")
            && !RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("appendFile"),
        "RFC-069 operator tool must require production confirmation and avoid plaintext-code evidence files"
    );
}

#[test]
fn rfc057_community_creation_is_guarded_active_admin_only() {
    assert!(
        LIB_SRC.contains("(Method::Get, \"/communities/new\")")
            && LIB_SRC.contains("(Method::Post, \"/communities/new\")"),
        "RFC-057 route must be top-level /communities/new, not scoped under /c/:id"
    );
    assert!(
        COMMUNITY_CREATE_HANDLER_SRC.contains("require_auth")
            && COMMUNITY_CREATE_HANDLER_SRC.contains("require_active_admin_somewhere"),
        "Community creation must require an authenticated active admin somewhere"
    );
    assert!(
        AUTHZ_SRC.contains("find_first_admin_for_user"),
        "Active-admin-somewhere eligibility must be enforced through authz"
    );
    assert!(
        COMMUNITY_CREATE_HANDLER_SRC.contains("COMMUNITY_CREATION_ENABLED"),
        "Community creation must be guarded by an operator feature flag"
    );
}

#[test]
fn rfc057_token_idempotency_rate_limit_and_timezone_are_fixed() {
    assert!(
        COMMUNITY_CREATE_HANDLER_SRC.contains("token_purpose::CREATE_COMMUNITY")
            && COMMUNITY_CREATE_HANDLER_SRC.contains("set_result")
            && COMMUNITY_CREATE_HANDLER_SRC.contains("ConsumeResult::Replay(Some(community_id))"),
        "Community creation must use scoped form tokens and replay to the created community"
    );
    assert!(
        COMMUNITY_CREATE_HANDLER_SRC.contains("Scope::CommunityUser")
            && COMMUNITY_CREATE_HANDLER_SRC.contains("Scope::CommunitySession")
            && COMMUNITY_CREATE_HANDLER_SRC.contains("Scope::CommunityNetwork"),
        "Community creation must be fail-closed rate-limited by user, session, and network (RFC-078)"
    );
    assert!(
        COMMUNITY_CREATE_HANDLER_SRC.contains("SUPPORTED_TIMEZONE: &str = \"Asia/Tokyo\"")
            && COMMUNITY_CREATE_HANDLER_SRC.contains("timezone != SUPPORTED_TIMEZONE"),
        "v0.41.0 must expose only the reviewed Japan-time selection"
    );
}

#[test]
fn rfc057_creation_writes_only_community_membership_and_audit() {
    assert!(
        COMMUNITY_DB_SRC.contains("INSERT INTO communities")
            && COMMUNITY_DB_SRC.contains("INSERT INTO community_memberships")
            && COMMUNITY_DB_SRC.contains("execute_required_batch")
            && COMMUNITY_DB_SRC.contains("AuditAction::CommunityCreated")
            && COMMUNITY_DB_SRC.contains("AuditAction::MembershipCreatedFirstAdmin"),
        "Community creation must batch community, first-admin membership, and audit writes"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("community.created")
            && RFC079_AUDIT_CORE_SRC.contains("membership.created_first_admin"),
        "Community creation must emit the reviewed audit events"
    );
    assert!(
        !COMMUNITY_DB_SRC.contains("INSERT INTO audit_log")
            && COMMUNITY_DB_SRC.matches("AuditMetadata::None").count() == 2,
        "Community creation audit insert must match the D1 schema column metadata_json"
    );
    for forbidden in [
        "event_days",
        "events",
        "attendance",
        "notes",
        "invite_codes",
        "event_templates",
    ] {
        assert!(
            !COMMUNITY_DB_SRC.contains(forbidden),
            "Community creation DB helper must not copy or generate {forbidden}"
        );
    }
    assert!(
        !COMMUNITY_CREATE_HANDLER_SRC.contains("GENERATE_INVITE")
            && !COMMUNITY_CREATE_HANDLER_SRC.contains("insert_invite")
            && !COMMUNITY_CREATE_HANDLER_SRC.contains("invite_code"),
        "Community creation must not auto-generate an invite code"
    );
}

#[test]
fn rfc057_me_entry_and_feature_flag_defaults_are_reviewed() {
    assert!(
        ME_HANDLER_SRC.contains("i18n::COMMUNITY_CREATE_LINK") // RFC-072 locale-aware accessor
            && ME_HANDLER_SRC.contains("/communities/new")
            && ME_HANDLER_SRC.contains("find_first_admin_for_user")
            && ME_HANDLER_SRC.contains("community_creation_enabled"),
        "Me page must show the quiet create-community entry only for eligible admins"
    );
    assert!(
        WRANGLER_TOML_SRC.contains("[env.dev.vars]")
            && WRANGLER_TOML_SRC.contains("COMMUNITY_CREATION_ENABLED = \"true\"")
            && WRANGLER_TOML_SRC.contains("[env.production.vars]")
            && WRANGLER_TOML_SRC.contains("COMMUNITY_CREATION_ENABLED = \"false\""),
        "Community creation flag should be enabled for local/staging review and off in production by default"
    );
}

#[test]
fn rfc070_self_display_name_editing_routes_are_member_scoped() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("\"me/display-name\"")
            && COMMUNITY_HANDLER_SRC.contains("get_display_name")
            && COMMUNITY_HANDLER_SRC.contains("post_display_name"),
        "RFC-070 must expose GET/POST /c/:cid/me/display-name through the community router"
    );
    assert!(
        ME_HANDLER_SRC.contains("require_membership(env, &auth, community_id, rid)")
            && !ME_HANDLER_SRC.contains("require_admin(env, &auth, community_id, rid)"),
        "RFC-070 display-name editing must require active membership, not admin role"
    );
    assert!(
        ME_HANDLER_SRC.contains("i18n::ME_CHANGE_DISPLAY_NAME") // RFC-072 locale-aware accessor
            && ME_HANDLER_SRC.contains("i18n::ME_DISPLAY_NAME_UPDATED") // RFC-072 locale-aware accessor
            && ME_HANDLER_SRC.contains("?flash={DISPLAY_NAME_UPDATED_REF}"),
        "RFC-070 Me page must expose the edit link and fixed-code success feedback"
    );
}

#[test]
fn rfc070_display_name_token_replay_and_validation_are_guarded() {
    assert!(
        ME_HANDLER_SRC.contains("token_purpose::CHANGE_DISPLAY_NAME")
            && ME_HANDLER_SRC.contains("validate_display_name(&raw_display_name)")
            && ME_HANDLER_SRC.contains("consume_detailed")
            && ME_HANDLER_SRC.contains("ConsumeResult::Replay(Some(result_ref))")
            && ME_HANDLER_SRC.contains("DISPLAY_NAME_UPDATED_REF")
            && ME_HANDLER_SRC.contains("DISPLAY_NAME_UNCHANGED_REF"),
        "RFC-070 must validate before detailed token consume and branch replays by stored result_ref"
    );
    assert!(
        ME_HANDLER_SRC.contains("form_token::set_result")
            && ME_HANDLER_SRC.contains("DISPLAY_NAME_UNCHANGED_REF")
            && ME_HANDLER_SRC.contains("return redirect(&format!(\"/c/{community_id}/me\"))"),
        "RFC-070 same-value no-op must store display_name_unchanged before redirecting"
    );
    assert!(
        ME_HANDLER_SRC.contains("ConsumeResult::Replay(_)")
            && !ME_HANDLER_SRC.contains("if replay.is_some()"),
        "RFC-070 must not use the older replay-is-some pattern that misses consumed tokens with no result_ref"
    );
}

#[test]
fn rfc070_display_name_update_audit_and_result_are_batched() {
    assert!(
        ME_HANDLER_SRC.contains("db.batch(vec![update_stmt, audit_stmt, result_stmt])")
            && ME_HANDLER_SRC.contains("UPDATE community_memberships")
            && ME_HANDLER_SRC.contains("SET display_name = ?1")
            && ME_HANDLER_SRC.contains("AND community_id = ?3")
            && ME_HANDLER_SRC.contains("AND user_id = ?4")
            && ME_HANDLER_SRC.contains("AND {MEMBERSHIP_ACTIVE}")
            && ME_HANDLER_SRC.contains("AND display_name != ?1"),
        "RFC-070 display-name update must be scoped to active membership, community, and authenticated user"
    );
    assert!(
        ME_HANDLER_SRC.contains("AuditAction::MembershipDisplayNameUpdated")
            && ME_HANDLER_SRC.contains("AuditMetadata::DisplayNameChanged")
            && ME_HANDLER_SRC.contains("statement_after_one_change")
            && ME_HANDLER_SRC.contains("UPDATE form_tokens")
            && ME_HANDLER_SRC.contains("result_ref = ?1")
            && ME_HANDLER_SRC.contains("require_changed(&results, 0")
            && ME_HANDLER_SRC.contains("require_changed(&results, 1")
            && ME_HANDLER_SRC.contains("require_changed(&results, 2"),
        "RFC-070 actual changes must batch update, audit insert, replay-result storage, and check each write"
    );
    assert!(
        !ME_HANDLER_SRC.contains("let _ = audit::write")
            && !ME_HANDLER_SRC.contains("audit::write("),
        "RFC-070 display-name updates must not use best-effort audit::write"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC
            .contains("DisplayNameChanged => json!({ \"changed_fields\": [\"display_name\"] })")
            && !ME_HANDLER_SRC.contains("serde_json::json!")
            && !ME_HANDLER_SRC.contains("INSERT INTO audit_log"),
        "RFC-070 metadata_json must keep IDs in audit columns and store only the changed field list"
    );
}

#[test]
fn rfc056_home_lists_communities_without_switcher() {
    assert!(
        HOME_HANDLER_SRC.contains("home_upcoming_for_communities"),
        "Home must batch nearby events across all user communities"
    );
    assert!(
        HOME_HANDLER_SRC.contains("render_home_communities"),
        "Home must render communities one by one"
    );
    assert!(
        HOME_HANDLER_SRC.contains("i18n::t(locale, i18n::NAV_HOME)") // RFC-072 locale-aware accessor
            && HOME_HANDLER_SRC.contains("render::header(title, \"\")"),
        "Home must use a simple header without the community switcher"
    );
    assert!(
        !HOME_HANDLER_SRC.contains("header_with_switcher"),
        "Home must not render the community switcher"
    );
}

#[test]
fn rfc056_calendar_page_owns_calendar_and_switcher() {
    assert!(
        COMMUNITIES_SRC.contains("render_calendar_month"),
        "The former Communities tab must render the active community calendar"
    );
    assert!(
        COMMUNITIES_SRC.contains("render_calendar_events"),
        "Calendar page must render the active community event list below the month grid"
    );
    assert!(
        COMMUNITIES_SRC.contains("event_db::calendar_month_for_community")
            && COMMUNITIES_SRC.contains("community_id")
            && COMMUNITIES_SRC.contains("month_start")
            && COMMUNITIES_SRC.contains("next_month_start"),
        "Calendar page events must be scoped to the selected active community and visible month"
    );
    assert!(
        !COMMUNITIES_SRC.contains("home_upcoming(&db, community_id"),
        "Calendar page must not use the Home next-30-days query for its month grid"
    );
    assert!(
        COMMUNITIES_SRC.contains("href=\\\"/c/{cid}/events/{eid}\\\""),
        "Calendar page event list must link into the selected community's Event Detail"
    );
    assert!(
        COMMUNITIES_SRC.contains("header_with_switcher_next"),
        "Calendar page must keep the community switcher"
    );
    assert!(
        COMMUNITIES_SRC.contains("switcher_next")
            && COMMUNITY_HANDLER_SRC.contains("communities:")
            && COMMUNITY_HANDLER_SRC.contains("calendar_next_destination"),
        "Calendar switcher must preserve the Calendar page, selected month, and selected day after switching communities"
    );
    assert!(
        COMMUNITIES_SRC.contains("query_pairs()")
            && COMMUNITIES_SRC.contains("\"month\"")
            && COMMUNITIES_SRC.contains("\"day\"")
            && COMMUNITIES_SRC.contains("i18n::CALENDAR_PREV_MONTH")
            && COMMUNITIES_SRC.contains("i18n::CALENDAR_NEXT_MONTH")
            && COMMUNITIES_SRC.contains("i18n::CALENDAR_THIS_MONTH")
            && COMMUNITIES_SRC.contains("i18n::CALENDAR_ALL_DAYS"),
        "Calendar page must support month navigation and a clearable selected-day agenda"
    );
    assert!(
        COMMUNITIES_SRC.contains("?month={month_key}&amp;day={day_date}")
            && COMMUNITIES_SRC.contains("aria-current=\\\"date\\\""),
        "Calendar day cells must link to a day-filtered agenda with accessible current-day state"
    );
    assert!(
        !RENDER_SRC.contains("onchange='this.form.submit()'"),
        "Community switcher must not rely on inline onchange handlers because CSP blocks them"
    );
    assert!(
        RENDER_SRC.contains("<button type='submit'")
            && RENDER_SRC.contains("i18n::t(locale, i18n::NAV_SWITCH_GO)") // RFC-072 locale-aware accessor
            && !RENDER_SRC.contains("<noscript><button type='submit'"),
        "Community switcher must have a visible submit fallback, not only a noscript-only button"
    );
    assert!(
        APP_JS_SRC.contains("form[action=\"/switch\"]")
            && APP_JS_SRC.contains("select[name=\"community\"]")
            && APP_JS_SRC.contains("button.hidden = true")
            && APP_JS_SRC.contains("form.submit()"),
        "External app.js must auto-submit the community switcher under CSP"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("admin_events_new"),
        "Admin event creation switcher must keep users on the create-event page for the selected community"
    );
    assert!(
        COMMUNITIES_SRC.contains("cz-calendar-grid")
            && APP_CSS_SOURCE.contains("grid-template-columns: repeat(7, minmax(0, 1fr));"),
        "Calendar overview must keep a stable seven-column grid"
    );
}

#[test]
fn rfc053_calendar_feed_privacy_and_revocation_ux_is_guarded() {
    assert!(
        CALENDAR_HANDLER_SRC.contains("i18n::t(locale, i18n::CALENDAR_PRIVACY_NOTE)") // RFC-072 locale-aware accessor, Handoff 030
            && CALENDAR_HANDLER_SRC.contains("i18n::t(locale, i18n::CALENDAR_GENERATED_FLASH)")
            && CALENDAR_HANDLER_SRC.contains("i18n::t(locale, i18n::CALENDAR_REVOKED_FLASH)")
            && CALENDAR_HANDLER_SRC.contains("calendar_flash_message")
            && CALENDAR_HANDLER_SRC.contains("?flash=generated")
            && CALENDAR_HANDLER_SRC.contains("?flash=disabled")
            && CALENDAR_HANDLER_SRC.contains("url.port()"),
        "RFC-053 calendar feed page must use reviewed fixed copy (now locale-aware, Handoff 030) and fixed flash codes"
    );
    assert!(
        !CALENDAR_HANDLER_SRC.contains("Feed+URL+generated")
            && !CALENDAR_HANDLER_SRC.contains("Feed+disabled")
            && !CALENDAR_HANDLER_SRC.contains("render::escape_html(&f)"),
        "Calendar feed actions must not surface raw or English flash query text"
    );
    assert!(
        CALENDAR_HANDLER_SRC.contains("cal_db::rotate_required")
            && CALENDAR_HANDLER_SRC.contains("cal_db::revoke_required")
            && CALENDAR_DB_SRC.contains("AuditAction::CalendarFeedTokenGenerated")
            && CALENDAR_DB_SRC.contains("AuditAction::CalendarFeedTokenRevoked")
            && CALENDAR_DB_SRC.contains("AuditMetadata::None")
            && !CALENDAR_DB_SRC.contains("LegacyAuditAction"),
        "Calendar token generation/revocation must be audited without token-bearing target_id or metadata"
    );
    assert!(
        CALENDAR_HANDLER_SRC.contains("Cache-Control")
            && CALENDAR_HANDLER_SRC.contains("no-store, private")
            && CALENDAR_HANDLER_SRC.contains("Referrer-Policy")
            && CALENDAR_HANDLER_SRC.contains("no-referrer")
            && CALENDAR_HANDLER_SRC.contains("X-Content-Type-Options")
            && CALENDAR_HANDLER_SRC.contains("nosniff")
            && LIB_SRC.contains("h.get(\"Referrer-Policy\")")
            && LIB_SRC.contains("Handlers may set a stricter policy"),
        "Bearer ICS responses must avoid caching, referrer leakage, and content sniffing"
    );

    assert!(
        CALENDAR_DB_SRC.contains("pub async fn events_for_feed")
            && CALENDAR_DB_SRC.contains("e.title")
            && CALENDAR_DB_SRC.contains("e.location")
            && CALENDAR_DB_SRC.contains("e.status")
            && CALENDAR_DB_SRC.contains("ed.starts_at_utc")
            && CALENDAR_DB_SRC.contains("ed.ends_at_utc")
            && CALENDAR_DB_SRC.contains("WHERE ed.community_id = ?1"),
        "ICS feed query must stay community-scoped and limited to event title/time/location/status"
    );
    let feed_query_src = CALENDAR_DB_SRC
        .split("pub async fn events_for_feed")
        .nth(1)
        .expect("events_for_feed must exist");
    for forbidden in [
        "attendance",
        "event_notes",
        "invite_codes",
        "community_memberships",
        "display_name",
        "description",
    ] {
        assert!(
            !feed_query_src.contains(forbidden),
            "ICS feed query must not expose {forbidden}"
        );
    }

    assert!(
        ICS_SRC.contains("SUMMARY:")
            && ICS_SRC.contains("DTSTART:")
            && ICS_SRC.contains("DTEND:")
            && ICS_SRC.contains("LOCATION:")
            && ICS_SRC.contains("STATUS:"),
        "ICS builder must keep the reviewed title/time/location/status output"
    );
    for forbidden in ["ATTENDEE", "DESCRIPTION", "COMMENT", "ORGANIZER"] {
        assert!(
            !ICS_SRC.contains(forbidden),
            "ICS output must not include participant, note, or admin fields: {forbidden}"
        );
    }
}

#[test]
fn calendar_overview_contract_is_explicit() {
    let calendar_src = COMMUNITIES_SRC
        .split("fn render_calendar_month")
        .nth(1)
        .expect("Calendar page must keep a dedicated calendar renderer");

    assert!(
        calendar_src.contains("i18n::t(locale, i18n::HOME_CALENDAR_HELPER)"),
        "Calendar overview must include helper copy explaining that details are in the list below"
    );
    assert!(
        calendar_src.contains("i18n::t(locale, i18n::TODAY)"),
        "Today must be identified by visible text, not color alone"
    );
    assert!(
        calendar_src.contains('●'),
        "Event presence must use a visible marker, not color alone"
    );
    assert!(
        calendar_src.contains("<a href=")
            && calendar_src.contains("aria-current=\\\"date\\\"")
            && calendar_src.contains("i18n::t(locale, i18n::CALENDAR_ALL_DAYS)"),
        "Calendar day cells are interactive in v0.42.0 and must expose selected-day state plus a clear filter"
    );
    // RFC-075 Slice 1: re-expressed against the rendered class set, not the
    // literal colour/border values that used to live in this file (moved to
    // app.css). Same guarantee, proven a different way — the class-migrated
    // twin of the RFC-067 gate rewrite made during RFC-074. Today and
    // selected must (a) be built from independent conditions, so a day can
    // never collapse them into one ambiguous state, and (b) render with a
    // genuinely different `app.css` treatment that is not colour alone —
    // checked here as differing `border-width`, mirroring the original
    // literal check's own "calmer... distinct" property.
    assert!(
        calendar_src.contains("day_class.push_str(\" cz-calendar-day--today\")")
            && calendar_src.contains("day_class.push_str(\" cz-calendar-day--selected\")")
            && calendar_src.contains("if is_today {")
            && calendar_src.contains("if is_selected {"),
        "Today and selected day-cell state must be built from independent conditions, never merged into one class"
    );
    let today_rule = css_rule_body(APP_CSS_SOURCE, ".cz-calendar-day--today");
    let selected_rule = css_rule_body(APP_CSS_SOURCE, ".cz-calendar-day--selected");
    assert!(
        today_rule.contains("border-width: 2px") && selected_rule.contains("border-width: 1px"),
        "Today styling must stay calmer than selected-day styling — distinct border-width, not colour alone"
    );
    assert_ne!(
        today_rule.split_whitespace().collect::<String>(),
        selected_rule.split_whitespace().collect::<String>(),
        "today and selected day-cell classes must render distinguishably from each other, not identically"
    );
}

/// Extract a top-level CSS rule's declaration body by selector, e.g.
/// `css_rule_body(src, ".cz-tab--active")` returns the text between that
/// selector's `{` and its matching `}`. Panics if the selector isn't found —
/// callers use this only to assert on rules that must exist.
///
/// Handoff 040 §7.3: tolerates any run of whitespace (including none)
/// between the selector and its opening brace — a rule written with
/// aligned braces (`.foo       { color: … }`) is not a different selector,
/// and the helper's job is to tolerate that formatting, not constrain it.
///
/// Still not a real CSS parser, and — corrected in Handoff 041 §7.3, which
/// found the prior wording here wrong — not the same narrowness the old
/// exact-string version had, either. The old version searched for the
/// literal `"{selector} {"`; an occurrence of `selector` not immediately
/// followed by `" {"` simply didn't match that literal, and `find` walked
/// on to the next occurrence of the full string. This version takes the
/// **first** occurrence of `selector` and panics if what follows isn't
/// whitespace-then-`{` — it does not search further. That is an
/// acceptable narrowing, not a hidden regression: the failure mode is a
/// loud panic naming the selector, never a silently wrong rule body, and
/// every selector these two files read (six today) occurs exactly once in
/// `app.css`.
fn css_rule_body<'a>(css: &'a str, selector: &str) -> &'a str {
    let start = css
        .find(selector)
        .unwrap_or_else(|| panic!("selector `{selector}` not found in app.css"));
    let after_selector = &css[start + selector.len()..];
    let brace_offset = after_selector
        .find('{')
        .filter(|&i| after_selector[..i].chars().all(char::is_whitespace))
        .unwrap_or_else(|| panic!("selector `{selector}` not found in app.css"));
    let after_brace = start + selector.len() + brace_offset + 1;
    let close = css[after_brace..]
        .find('}')
        .unwrap_or_else(|| panic!("selector `{selector}` has no closing brace in app.css"));
    &css[after_brace..after_brace + close]
}

#[test]
fn rfc067_monthly_attendance_matrix_contract_is_guarded() {
    assert!(
        COMMUNITIES_HANDLER_SRC.contains("mod calendar;")
            && COMMUNITIES_HANDLER_SRC.contains("mod matrix;")
            && COMMUNITIES_HANDLER_SRC.contains("matrix::render_matrix")
            && COMMUNITIES_HANDLER_SRC.contains("attendance_db::list_for_event_days"),
        "RFC-067 must keep Calendar route orchestration split from matrix rendering and use one batched attendance query"
    );
    assert!(
        COMMUNITIES_HANDLER_SRC.contains("calendar_month_for_community_limited")
            && COMMUNITIES_HANDLER_SRC.contains("matrix::EVENT_DAY_ROW_CAP + 1")
            && EVENT_DB_SRC.contains("pub async fn calendar_month_for_community_limited")
            && EVENT_DB_SRC.contains("LIMIT ?4"),
        "RFC-067 matrix mode must fetch one row past the event-day cap so over-cap months cannot render truncated matrices"
    );
    assert!(
        COMMUNITIES_MATRIX_SRC.contains("pub(super) const MEMBER_ROW_CAP: usize = 100")
            && COMMUNITIES_MATRIX_SRC.contains("pub(super) const EVENT_DAY_ROW_CAP: usize = 300")
            && COMMUNITIES_MATRIX_SRC.contains("i18n::CALENDAR_MATRIX_TOO_LARGE"),
        "RFC-067 matrix caps and too-large fallback must stay fixed"
    );
    assert!(
        MEMBERSHIP_DB_SRC.contains("ORDER BY display_name ASC, id ASC"),
        "RFC-067 matrix member ordering must be stable for duplicate display names"
    );
    assert!(
        COMMUNITY_HANDLER_SRC.contains("[\"communities\", month, \"matrix\"]")
            && COMMUNITY_HANDLER_SRC.contains("[\"communities\", month, day, \"matrix\"]")
            && COMMUNITY_HANDLER_SRC.contains("&view=matrix"),
        "RFC-067 community switcher grammar must preserve exact matrix mode shapes"
    );
    assert!(
        COMMUNITIES_MATRIX_SRC.contains("CalendarView::Matrix")
            && COMMUNITIES_MATRIX_SRC.contains("view=matrix")
            && COMMUNITIES_MATRIX_SRC.contains("i18n::CALENDAR_VIEW_MATRIX")
            && COMMUNITIES_MATRIX_SRC.contains("i18n::CALENDAR_MATRIX_TITLE"),
        "RFC-067 matrix mode must be route-backed and visibly switchable"
    );
    assert!(
        COMMUNITIES_MATRIX_SRC.contains("\"○\"")
            && COMMUNITIES_MATRIX_SRC.contains("\"×\"")
            && COMMUNITIES_MATRIX_SRC.contains("\"済\"")
            && COMMUNITIES_MATRIX_SRC.contains("\"?\"")
            && COMMUNITIES_MATRIX_SRC.contains("\"中\"")
            && COMMUNITIES_MATRIX_SRC.contains("format!(\"{answered}/{total}\")")
            && COMMUNITIES_MATRIX_SRC.contains("i18n::CALENDAR_MATRIX_CELL_BREAKDOWN")
            && zinnias_ciao_contracts::i18n::JA_CALENDAR_MATRIX_CELL_BREAKDOWN
                .contains("未回答{}件"),
        "RFC-067 matrix cells must keep the reviewed single-event symbols and multi-event answered/total accessible breakdown"
    );
    assert!(
        COMMUNITIES_MATRIX_SRC.contains("data-export-value")
            && COMMUNITIES_MATRIX_SRC.contains("can_export_csv")
            && COMMUNITIES_MATRIX_SRC.contains("export_token")
            && COMMUNITIES_MATRIX_SRC.contains("render_too_large"),
        "RFC-067/RFC-068 matrix export metadata must stay renderer-owned, admin-gated, and absent from too-large fallback"
    );
}

#[test]
fn rfc074_fallback_family_pages_stay_on_default_switcher() {
    // RFC-074's route-family matrix deliberately leaves these pages on the
    // default switcher (bare `header_with_switcher`, no `next` token), since
    // there is no cross-community equivalence for the event or member id
    // they view: Event Detail, note-delete confirmation, and the event admin
    // pages (attendance, cancel, edit) all fall back to the target's Home.
    // A bare call is what produces that fallback; this gate catches a future
    // edit silently handing one of them a `next` token the matrix does not
    // assign.
    let attendance = include_str!("../../../workers/ssr/src/handlers/admin/events/attendance.rs");
    let cancel = include_str!("../../../workers/ssr/src/handlers/admin/events/cancel.rs");
    let edit = include_str!("../../../workers/ssr/src/handlers/admin/events/edit.rs");
    let notes = include_str!("../../../workers/ssr/src/handlers/admin/events/notes.rs");

    for (name, src) in [
        (
            "event.rs (Event Detail + note-delete confirmation)",
            EVENT_HANDLER_SRC,
        ),
        ("admin/events/attendance.rs", attendance),
        ("admin/events/cancel.rs", cancel),
        ("admin/events/edit.rs", edit),
        ("admin/events/notes.rs", notes),
    ] {
        assert!(
            src.contains("header_with_switcher(")
                || src.contains("header_with_switcher_localized("),
            "{name} must still render the community switcher"
        );
        assert!(
            !src.contains("header_with_switcher_next(")
                && !src.contains("header_with_switcher_next_localized("),
            "{name} must stay on the default switcher and fall back to target Home; RFC-074's route-family matrix does not assign it a next token"
        );
    }
}

#[test]
fn rfc072_locale_resolution_never_panics_on_a_bad_stored_value() {
    // Moved from authz.rs to db/membership.rs in Slice B (§7.1): resolution
    // now happens next to the SELECT that reads ui_language, inside
    // find_active, and MembershipContext.locale is a pre-resolved Locale
    // — see rfc072_locale_is_only_ever_read_from_find_active below.
    //
    // RFC-085 §3.2: `unwrap_or_default()` no longer exists on this path —
    // `impl Default for Locale` was removed precisely so this fallback
    // could no longer be spelled anonymously; `Locale::FAIL_CLOSED` is the
    // named answer for a corrupt stored value (`resolve_locale_corrupt_value_arm_references_fail_closed_not_product_default`
    // above pins which arm uses which constant — this gate only needs to
    // confirm parsing is attempted and no bad value is ever assumed valid).
    assert!(
        MEMBERSHIP_DB_SRC.contains("fn resolve_locale")
            && MEMBERSHIP_DB_SRC.contains("Locale::parse")
            && MEMBERSHIP_DB_SRC.contains("Locale::FAIL_CLOSED"),
        "RFC-072 locale resolution must parse-or-fall-back, not assume a valid stored value"
    );
    let resolve_locale_fn = MEMBERSHIP_DB_SRC
        .split("fn resolve_locale")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("resolve_locale function body must be present");
    assert!(
        !resolve_locale_fn.contains(".unwrap()") && !resolve_locale_fn.contains(".expect("),
        "RFC-072 locale resolution must not unwrap/expect on the locale read path — a panic in a render path is an SEC-5 violation"
    );
}

#[test]
fn rfc072_locale_is_only_ever_read_from_find_active() {
    // §7.1's trap, made structurally unrepresentable in Slice B: only
    // `find_active` returns a row with a `locale` field at all (as
    // `ActiveMembershipRow`); `find_active_by_id`, `list_active_for_user`,
    // and `find_first_admin_for_user` return the plain `MembershipRow`,
    // which has no locale field to reach for. A future page migrated from
    // one of those three cannot silently render the wrong language — it
    // would be a compile error, not a passing test with a wrong result.
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub struct ActiveMembershipRow")
            && MEMBERSHIP_DB_SRC.contains("pub locale: Locale")
            && MEMBERSHIP_DB_SRC.contains("pub async fn find_active(")
            && MEMBERSHIP_DB_SRC.contains("Result<Option<ActiveMembershipRow>>"),
        "RFC-072: only find_active may return a row carrying a resolved locale"
    );
    assert!(
        !MEMBERSHIP_DB_SRC.contains("pub struct MembershipRow")
            || !MEMBERSHIP_DB_SRC
                .split("pub struct MembershipRow")
                .nth(1)
                .and_then(|rest| rest.split("}\n").next())
                .unwrap_or_default()
                .contains("locale"),
        "RFC-072: the plain MembershipRow (used by find_active_by_id, list_active_for_user, \
         find_first_admin_for_user) must never carry a locale field"
    );
}

#[test]
fn rfc072_language_settings_post_is_reject_no_op_replay_and_target_safe() {
    // RFC-072 Slice B §7.3/§8: post_language's shape mirrors post_display_name
    // (RFC-070) — validated below via static source assertions, the same
    // technique the RFC-070 gates above use, since `worker::Request`/D1
    // cannot be constructed in a native test environment.
    let post_language_src = ME_HANDLER_SRC
        .split("pub async fn post_language")
        .nth(1)
        .expect("post_language handler must exist")
        .split("\nasync fn refresh_language_form")
        .next()
        .expect("post_language handler must end before refresh_language_form");

    // Reject: an out-of-allow-list ui_language value must be rejected before
    // the token is ever consumed, so a legitimate mistaken resubmission can
    // retry with the same token (same reasoning as RFC-070's validate-before-
    // consume order).
    assert!(
        post_language_src.contains("Locale::parse(&raw_ui_language)")
            && post_language_src.contains("refresh_language_form(")
            && post_language_src
                .find("Locale::parse(&raw_ui_language)")
                .unwrap()
                < post_language_src.find("consume_detailed(").unwrap(),
        "post_language must validate the submitted locale before consuming the form token"
    );

    // No-op: submitting the member's current locale must not write, only
    // record a replay-safe result_ref before redirecting — the actual DB
    // write must be reachable only after this branch's early return.
    assert!(
        post_language_src.contains("if submitted == membership.locale {")
            && post_language_src.contains(
                "crate::form_token::set_result(&db, pepper.as_str(), &raw_token, UI_LANGUAGE_UNCHANGED_REF)"
            )
            && post_language_src
                .find("crate::form_token::set_result(&db, pepper.as_str(), &raw_token, UI_LANGUAGE_UNCHANGED_REF)")
                .unwrap()
                < post_language_src.find("update_ui_language_with_result(").unwrap(),
        "post_language same-value submission must be a no-op that records UI_LANGUAGE_UNCHANGED_REF, not a write"
    );

    // Replay: consumed token detection must branch by stored result_ref, not
    // the older `.is_some()` pattern that misses consumed tokens with no ref
    // (the same lesson RFC-070's gate above pins).
    assert!(
        post_language_src.contains(
            "ConsumeResult::Replay(Some(result_ref)) if result_ref == UI_LANGUAGE_UPDATED_REF"
        ) && post_language_src.contains(
            "ConsumeResult::Replay(Some(result_ref)) if result_ref == UI_LANGUAGE_UNCHANGED_REF"
        ) && post_language_src.contains("ConsumeResult::Replay(_)")
            && !post_language_src.contains("if replay.is_some()"),
        "post_language must classify every ConsumeResult::Replay case by its stored result_ref"
    );

    // No hidden field determines target: the form must read only its own
    // token and the selected language — the membership/community being
    // updated comes from `require_membership`'s URL-scoped lookup, not from
    // attacker-controlled form data.
    assert!(
        post_language_src.matches("form.get_field(").count() == 2
            && post_language_src.contains("form.get_field(\"_token\")")
            && post_language_src.contains("form.get_field(\"ui_language\")"),
        "post_language must not accept a hidden field that could redirect the write to another membership"
    );
}

/// RFC-072 criterion 9, Handoff 030 §7.3: one documented exception to the
/// default-fail localization gate below. `ja_count` is the file's exact,
/// pinned bare `i18n::JA_` reference count; `bare_helper_calls` is the
/// file's exact, pinned count of calls to any render helper that has a
/// `_localized` sibling (Handoff 073 — generalizes what used to be a
/// single-helper `calls_bare_page: bool`, see
/// `locale_blind_helpers_with_localized_sibling` below). Both are asserted
/// exactly, not as a floor or ceiling, so a partial edit to an excluded
/// file — the exact defect this gate exists to catch — still fails it.
struct LocalizationException {
    path: &'static str,
    ja_count: usize,
    bare_helper_calls: usize,
    reason: &'static str,
}

// This table replaces the hand-maintained `*_SRC` constant list the old
// `rfc072_member_facing_core_has_no_half_migrated_page` gate used. That gate
// only checked a file if someone remembered to add it — which is exactly
// why `calendar.rs` (Handoff 030) went unnoticed through three RFC-072
// slices despite being linked directly from My Page. This table inverts
// that: `rfc072_every_handler_and_render_file_is_localized_or_documented_exception`
// below walks every file under `handlers/` and `render/` and fails on
// anything not localized *and* not listed here, so an unlisted file is a
// failure, not a silent pass.
//
// Built by walking the tree and reading every flagged file, not carried
// over from Handoff 030's own sketch — which, checked against this walk,
// turned out to be missing two files (`admin/events/forms.rs` and
// `admin/events/summary.rs`, both shared admin form-rendering helpers with
// bare Japanese strings). Reported in the review request as a finding, not
// silently added to make a prior list "complete."
const LOCALIZATION_EXCEPTIONS: &[LocalizationException] = &[
    LocalizationException {
        path: "handlers/calendar.rs",
        ja_count: 2,
        bare_helper_calls: 0,
        reason: "get_ics_feed is an unauthenticated bearer-token route with no membership lookup, so no locale is resolvable yet — same rationale as render/errors.rs (Handoff 030 §7.1)",
    },
    LocalizationException {
        path: "handlers/communities.rs",
        ja_count: 1,
        bare_helper_calls: 0,
        reason: "post_matrix_export_audit's pre-auth 401 branch rejects before any membership lookup exists, so no locale is resolvable yet — same rationale as render/errors.rs",
    },
    LocalizationException {
        path: "render/errors.rs",
        ja_count: 20,
        bare_helper_calls: 0,
        reason: "these functions take no arguments, so they have no membership and no locale to resolve (RFC-072 §6 non-change scope)",
    },
];

/// RFC-083 §6.1: the exception table may only shrink. Pinned at Handoff 062
/// (RFC-083 Slice D1a), which converted 9 of the table's 10 D1a files
/// (27→18 entries, 308→203 sites — `create.rs` stayed pending an architect
/// decision). Handoff 071 (RFC-083 F1) resolved that decision and converted
/// the tenth: 18→17 entries, 203→196 sites, closing Slice D1a. Handoff 072
/// (RFC-083 Slice D1b) converted the five member-administration files:
/// 17→12 entries, 196→121 sites. Handoff 074 (RFC-083 Slice D1c) converted
/// the last two admin files, `templates.rs` and `export.rs`: 12→10 entries,
/// 121→98 `ja_count` sites, and — the third dimension this table has
/// carried since Handoff 073 but never had its own shrink-only total until
/// then — 14→8 `bare_helper_calls` sites, closing RFC-083 Slice D1. Handoff
/// 075 (RFC-083 Slice D2a) converted the four anonymous/redemption routes
/// (`join.rs`, `relink.rs`, `recovery.rs`, `identity/mod.rs`): 10→6
/// entries, 98→54 `ja_count` sites, 8→3 `bare_helper_calls`. RFC-084
/// (Handoff 084) discharged D2b and closed the localization programme's
/// last convertible work — the account tier (`account/mod.rs`,
/// `account/link.rs`, `account/unlink.rs`): 6→3 entries, 54→23 `ja_count`
/// sites, 3→0 `bare_helper_calls`. The three entries remaining are exactly
/// the structurally-unresolvable files (RFC-083 §4.4) — nothing left is a
/// deferred decision. A future addition to the table must lower these
/// pinned values deliberately, not raise them: growth here means a page
/// stopped being localized, not a documented decision to leave one alone.
#[test]
fn rfc083_localization_exceptions_table_only_shrinks() {
    assert_eq!(
        LOCALIZATION_EXCEPTIONS.len(),
        3,
        "LOCALIZATION_EXCEPTIONS grew to {} entries. This table is shrink-only \
         (RFC-083 §6.1) — re-pin this value only alongside a deliberate, reviewed \
         decision to leave a newly-discovered file unlocalized, never to make a \
         gate pass silently.",
        LOCALIZATION_EXCEPTIONS.len()
    );
    let total_sites: usize = LOCALIZATION_EXCEPTIONS.iter().map(|e| e.ja_count).sum();
    assert_eq!(
        total_sites, 23,
        "LOCALIZATION_EXCEPTIONS site total grew to {total_sites}. Re-pin only \
         alongside a reviewed, deliberate change to an individual entry's ja_count."
    );
    let total_bare_helper_calls: usize = LOCALIZATION_EXCEPTIONS
        .iter()
        .map(|e| e.bare_helper_calls)
        .sum();
    assert_eq!(
        total_bare_helper_calls, 0,
        "LOCALIZATION_EXCEPTIONS bare_helper_calls total grew to {total_bare_helper_calls}. \
         Re-pin only alongside a reviewed, deliberate change to an individual entry's \
         bare_helper_calls."
    );
}

/// RFC-083 §6.3 (Handoff 075): these four anonymous/redemption routes must
/// rely on `lib.rs`'s default `Cache-Control: no-store` (set only when a
/// handler hasn't already set one — `lib.rs:281`) rather than opting into
/// caching themselves. A cacheable response on any of these routes risks
/// serving one visitor's negotiated locale, or a redemption outcome, to
/// the next visitor sharing that cache entry. None of the four may write
/// a `Cache-Control` header anywhere in their own source.
#[test]
fn anonymous_routes_rely_on_the_default_no_store_cache_control() {
    let identity_mod_path = workers_ssr_src_dir().join("handlers/identity/mod.rs");
    let identity_mod = std::fs::read_to_string(&identity_mod_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", identity_mod_path.display()));
    for (name, source) in [
        ("handlers/join.rs", JOIN_HANDLER_SRC),
        ("handlers/relink.rs", RELINK_HANDLER_SRC),
        ("handlers/recovery.rs", RECOVERY_HANDLER_SRC),
        ("handlers/identity/mod.rs", identity_mod.as_str()),
    ] {
        assert!(
            !source.contains("Cache-Control"),
            "{name} must not set its own Cache-Control header — these anonymous/redemption \
             routes rely on lib.rs's default `no-store` (lib.rs:281). A route-specific value \
             here would risk making a redemption or locale-bearing response cacheable."
        );
    }
}

/// RFC-085 §3.4/§5 (Handoff 085): `impl Default for Locale` must never
/// return. Its whole purpose was one value silently answering three
/// different questions (RFC-085 §2) — a member's unexpressed preference, a
/// corrupt stored value, and an unmatched `Accept-Language`. Reintroducing
/// it as a convenience re-merges them invisibly, from anywhere a bare
/// `Locale::default()`/`.unwrap_or_default()` on an `Option<Locale>` cared
/// to reach it. Comments stripped first — this gate is necessarily
/// surrounded by prose about the very impl it forbids — and matched on the
/// specific `impl Default for Locale` shape, never the bare word
/// `Default`, so `Locale`'s own `#[derive(Debug, Clone, Copy, PartialEq,
/// Eq)]` and every unrelated `#[derive(Default)]` elsewhere in the
/// codebase cannot trip it.
#[test]
fn locale_never_regains_a_default_impl() {
    let production = strip_line_comments(LOCALE_SRC);
    assert!(
        !production.contains("impl Default for Locale"),
        "packages/contracts/src/locale.rs must never implement Default for Locale — RFC-085 \
         removed it because one value was silently answering three different questions (a \
         member's unexpressed preference, a corrupt stored value, and an unmatched \
         Accept-Language). Reintroducing it re-merges them. Name which question you're \
         answering instead: Locale::PRODUCT_DEFAULT or Locale::FAIL_CLOSED."
    );
}

/// RFC-085 §6.4 (Handoff 085): the re-merge guard. A purely behavioral test
/// cannot catch someone pointing `resolve_locale`'s corrupt-value arm at
/// `Locale::PRODUCT_DEFAULT` instead of `Locale::FAIL_CLOSED` — both
/// constants are `Locale::Ja` today, so the two arms would produce
/// identical *output* right up until the product default flips, at which
/// point the fail-closed answer would silently follow it. This scans which
/// named constant the source actually references, which a value-only
/// comparison cannot distinguish.
#[test]
fn resolve_locale_corrupt_value_arm_references_fail_closed_not_product_default() {
    let production = strip_line_comments(MEMBERSHIP_DB_SRC);
    let body = compact_brace_block(&production, "fn resolve_locale");
    assert!(
        body.contains("None=>Locale::PRODUCT_DEFAULT"),
        "resolve_locale's None (no stored preference) arm must resolve to \
         Locale::PRODUCT_DEFAULT — the product's own answer, not the fail-closed one. Found: \
         {body}"
    );
    assert!(
        body.contains("unwrap_or(Locale::FAIL_CLOSED)"),
        "resolve_locale's corrupt-value arm must resolve to Locale::FAIL_CLOSED, not a literal \
         or the product default. Found: {body}"
    );
    assert!(
        !body.contains("unwrap_or(Locale::PRODUCT_DEFAULT)"),
        "resolve_locale's corrupt-value arm must never reference Locale::PRODUCT_DEFAULT — \
         RFC-085 §5 requires the fail-closed answer to never move as a side effect of a \
         product decision. This is exactly the re-merge this gate exists to catch."
    );
}

fn handlers_and_render_files() -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    walk_rs_files(&workers_ssr_src_dir().join("handlers"), &mut files);
    walk_rs_files(&workers_ssr_src_dir().join("render"), &mut files);
    files
        .into_iter()
        .filter(|p| p.file_name().and_then(|n| n.to_str()) != Some("tests.rs"))
        .collect()
}

/// The render helpers a converted file must never call directly (Handoff
/// 073, F1 of the RFC-083 Slice D1b review): every `pub fn <name>_localized`
/// under `workers/ssr/src/render.rs`/`render/*.rs` that also has a bare
/// `pub fn <name>` sibling. Derived, not hard-coded — a future helper
/// gaining a `_localized` sibling is covered by existing rather than by
/// anyone remembering, the same rule as `LOCALIZATION_EXCEPTIONS` itself,
/// the smoke run set, EN/JA parity, and the leakage baseline. Comments
/// stripped first: five gates in this project have now matched their own
/// explanatory prose, one of them in D1b.
///
/// Deliberately narrow: only a literal `pub fn ` prefix counts (not
/// `pub(crate) fn`/`pub(super) fn`), matching every helper this project has
/// actually written this way; `header` (bare, no `_localized` sibling of
/// its own) and `header_with_switcher_localized` (no bare sibling) are
/// correctly excluded by the intersection, not by name.
fn locale_blind_helpers_with_localized_sibling() -> Vec<String> {
    let src_dir = workers_ssr_src_dir();
    let mut render_files = vec![src_dir.join("render.rs")];
    walk_rs_files(&src_dir.join("render"), &mut render_files);
    render_files.retain(|p| p.file_name().and_then(|n| n.to_str()) != Some("tests.rs"));

    let mut localized_names = std::collections::BTreeSet::new();
    let mut bare_names = std::collections::BTreeSet::new();
    for path in &render_files {
        let raw = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let stripped = strip_line_comments(&raw);
        for line in stripped.lines() {
            let Some(rest) = line.trim_start().strip_prefix("pub fn ") else {
                continue;
            };
            let Some(paren) = rest.find('(') else {
                continue;
            };
            let name = rest[..paren].trim();
            match name.strip_suffix("_localized") {
                Some(base) => {
                    localized_names.insert(base.to_string());
                }
                None => {
                    bare_names.insert(name.to_string());
                }
            }
        }
    }
    localized_names.intersection(&bare_names).cloned().collect()
}

/// Per-helper bare-call counts for `content` against the derived `helpers`
/// set, comments stripped first. Kept per-helper (not pre-summed) so a
/// failing assertion can name exactly which helper was called bare, not
/// just how many times something in the set matched.
fn bare_locale_blind_helper_call_counts(content: &str, helpers: &[String]) -> Vec<(String, usize)> {
    let stripped = strip_line_comments(content);
    helpers
        .iter()
        .map(|name| {
            (
                name.clone(),
                stripped.matches(&format!("render::{name}(")).count(),
            )
        })
        .collect()
}

#[test]
fn rfc072_every_handler_and_render_file_is_localized_or_documented_exception() {
    // RFC-072 criterion 9: every page reachable from My Page honours the
    // member's language, with out-of-boundary surfaces Japanese "by
    // documented decision, not by omission." Handoff 030: the prior gate
    // (a hand-maintained list of `*_SRC` constants) checked a file only if
    // someone remembered to add it, and missed `calendar.rs` through three
    // RFC-072 slices as a result. This version is default-fail: it walks
    // every non-test file under `handlers/` and `render/`, and anything
    // that calls a locale-blind render helper (derived — see
    // `locale_blind_helpers_with_localized_sibling`) or contains a bare
    // `i18n::JA_` reference must be named in `LOCALIZATION_EXCEPTIONS` with
    // an exact pinned count and a written reason — otherwise it fails.
    //
    // Handoff 073 (F1 of the D1b review): originally checked one named
    // helper, `render::page(`, via `calls_bare_page: bool`. A converted
    // file calling `render::bottom_nav(` or `render::header_with_switcher_next(`
    // directly produces a page with the correct `html lang` and an English
    // body over a Japanese navigation bar — and every gate, including this
    // one's prior form, passed. Generalized to every render helper with a
    // `_localized` sibling, derived by scanning the render layer rather
    // than naming helpers by hand.
    let files = handlers_and_render_files();
    assert!(
        files.len() > 30,
        "expected many .rs files under handlers/ and render/, found only {} — \
         directory walk is probably broken, not the codebase actually shrinking",
        files.len()
    );

    // Handoff 083 Part A deleted `render::page` (the last helper this
    // derivation had found), so this set is now correctly empty — the
    // derivation working, not the scan breaking. The floor below no longer
    // requires a non-empty result; a genuinely broken scan is still caught
    // by an upper bound never being sanely large, and by the demonstration
    // in Handoff 073's own review that this same function correctly finds
    // one when a bare/`_localized` pair exists. If a helper gains a
    // `_localized` sibling again, this set picks it up with no edit here.
    let helpers = locale_blind_helpers_with_localized_sibling();
    assert!(
        helpers.len() < 10,
        "expected a small derived set of render helpers with a _localized sibling, found {} \
         ({helpers:?}) — the render-layer scan is probably broken, not the codebase having \
         dozens",
        helpers.len()
    );

    let src_dir = workers_ssr_src_dir();
    let mut seen_exception_paths = std::collections::HashSet::new();

    for path in &files {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let ja_count = content.matches("i18n::JA_").count();
        let helper_counts = bare_locale_blind_helper_call_counts(&content, &helpers);
        let bare_helper_calls: usize = helper_counts.iter().map(|(_, c)| c).sum();
        let helper_breakdown: Vec<String> = helper_counts
            .iter()
            .filter(|(_, c)| *c > 0)
            .map(|(name, c)| format!("render::{name}( x{c}"))
            .collect();
        let rel = path
            .strip_prefix(&src_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        match LOCALIZATION_EXCEPTIONS.iter().find(|e| e.path == rel) {
            Some(exc) => {
                seen_exception_paths.insert(exc.path);
                assert_eq!(
                    ja_count, exc.ja_count,
                    "{rel}: bare i18n::JA_ count is {ja_count}, pinned exception value is {} \
                     ({}). Re-pin only if this is a deliberate, reviewed change — a partial \
                     edit to an excluded file must not pass silently.",
                    exc.ja_count, exc.reason
                );
                assert_eq!(
                    bare_helper_calls,
                    exc.bare_helper_calls,
                    "{rel}: bare calls to a locale-blind render helper ({}) changed from its \
                     pinned exception value ({}) — re-pin only if deliberate ({})",
                    helper_breakdown.join(", "),
                    exc.bare_helper_calls,
                    exc.reason
                );
            }
            None => {
                assert_eq!(
                    ja_count, 0,
                    "{rel} has {ja_count} bare i18n::JA_ reference(s) and is not in \
                     LOCALIZATION_EXCEPTIONS. RFC-072 criterion 9 requires every member-facing \
                     page to honour locale unless excluded for a written reason — either \
                     localize this file, or add a table entry with the exact count and why."
                );
                assert_eq!(
                    bare_helper_calls,
                    0,
                    "{rel} calls a locale-blind render helper {bare_helper_calls} time(s) ({}) \
                     and is not in LOCALIZATION_EXCEPTIONS — use its _localized sibling \
                     instead, or add a table entry with a written reason. This is the exact \
                     defect that produces a correct html lang over a wrong-language \
                     navigation bar, with a source scan for i18n::JA_ alone seeing nothing.",
                    helper_breakdown.join(", ")
                );
            }
        }
    }

    // Keep the table itself honest: every entry must correspond to a file
    // the walk actually found, so a renamed or deleted file can't leave a
    // stale, unverifiable row behind.
    for exc in LOCALIZATION_EXCEPTIONS {
        assert!(
            seen_exception_paths.contains(exc.path),
            "LOCALIZATION_EXCEPTIONS names {} but the walk never found or matched it — \
             stale table entry?",
            exc.path
        );
    }

    // Preserved from the prior gate: communities.rs's one exception must be
    // exactly the documented pre-auth 401 branch, not some other bare
    // reference that happens to keep the count at 1.
    assert!(
        COMMUNITIES_HANDLER_SRC.contains("json_error(401, i18n::JA_SESSION_EXPIRED)"),
        "communities.rs's one bare i18n::JA_ reference must be the documented pre-auth 401 branch"
    );
}

/// Handoff 064 (F3 of the RFC-054 Slice 1 review, corrected): what
/// `en_ja_parity` in `packages/contracts/src/i18n/tests.rs` used to be named
/// for. That test's entire body asserted a literal 230-element array's
/// length against a literal `230` and that no literal in the array was
/// empty — it never referenced a single `EN_`/`JA_` identifier, so it would
/// have passed unchanged if every `JA_` constant in the project were
/// deleted. This gate replaces it: default-fail, same shape as
/// `LOCALIZATION_EXCEPTIONS`/`SMOKE_COVERAGE_EXCEPTIONS`, a pinned exception
/// table for the genuinely-unpaired stems with a written reason each.
struct EnJaParityException {
    stem: &'static str,
    reason: &'static str,
}

/// RFC-083 Slice D's not-yet-converted stems (Handoff 064 §2.1,
/// independently re-derived, not copied from the finding that reported
/// them; originally six — Handoff 071 removed `ADMIN_USE_TEMPLATE_LINK`
/// closing D1a, Handoff 072 removed `ADMIN_INVITE_REVOKED_FLASH` closing
/// D1b, Handoff 074 removed the last four (`ADMIN_EXPORT_SUMMARY_COUNTS`,
/// `ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH`, `ADMIN_TEMPLATE_SAVED_FLASH`,
/// `ADMIN_TEMPLATE_DELETED_FLASH`) closing D1c). **Empty since Handoff
/// 074** — every `EN_` constant in the corpus now has a `JA_` counterpart.
/// Confirmed inert: the stale-entry loop below simply iterates zero times
/// over an empty slice, and no other assertion in this gate requires the
/// table to be non-empty. Left in place (not deleted) as the table this
/// project's next Slice D2/D3 unpaired stem will land in — the shape that
/// shrinks to zero and stays ready, not a table that must be re-created
/// from scratch.
const EN_JA_PARITY_EXCEPTIONS: &[EnJaParityException] = &[];

/// Handoff 070 Part B: folds the `EN != JA` check that `i18n_en_ja_parity_count`
/// used to perform (via a one-entry `INTENTIONALLY_IDENTICAL` list) into this
/// derived gate, replacing that hand-maintained test entirely. Separate table
/// from `EN_JA_PARITY_EXCEPTIONS` above — that one names stems with no
/// counterpart at all; this one names stems whose EN and JA halves are the
/// same text on purpose. A second entry appearing here is a stop condition
/// (Handoff 070 §12), not a second row to add without escalating — two
/// constants sharing text across languages is far more likely a copy-paste
/// than a second legitimate product name.
struct EnJaIdenticalException {
    stem: &'static str,
    reason: &'static str,
}

const EN_JA_IDENTICAL_EXCEPTIONS: &[EnJaIdenticalException] = &[EnJaIdenticalException {
    stem: "JOIN_HEADING",
    reason: "the product name \"ciao.zinnias\" reads identically in both languages",
}];

fn i18n_module_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/i18n")
}

/// Every `packages/contracts/src/i18n/*.rs` file except `tests.rs` — the
/// module that defines the constants this gate checks, not fixtures for
/// checking them.
fn i18n_module_files() -> Vec<std::path::PathBuf> {
    let dir = i18n_module_dir();
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read directory {}: {e}", dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|e| panic!("failed to read directory entry: {e}"))
                .path()
        })
        .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
        .filter(|p| p.file_name().and_then(|n| n.to_str()) != Some("tests.rs"))
        .collect();
    files.sort();
    files
}

/// Every `{...}` placeholder in `s`, in order, as raw tokens (`{}`,
/// `{events}`, ...) — this codebase's own two substitution conventions
/// (`substitute_positional`'s positional `{}`, and `.replace("{name}", ...)`
/// by name), neither of which is `format!`, so `{{`/`}}` escaping never
/// applies here.
fn placeholders(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut base = 0usize;
    let mut rest = s;
    while let Some(start) = rest.find('{') {
        match rest[start..].find('}') {
            Some(rel_end) => {
                let end = start + rel_end + 1;
                out.push(&s[base + start..base + end]);
                base += end;
                rest = &s[base..];
            }
            None => break,
        }
    }
    out
}

/// One `pub const EN_<STEM>`/`pub const JA_<STEM>: &str = "..."` found by
/// `extract_i18n_constants`.
struct I18nConstant {
    stem: String,
    value: String,
    file: String,
}

/// Parses every `pub const EN_<STEM>`/`JA_<STEM>: &str = "..."` out of
/// `content` (already comment-stripped by the caller). Scans the string
/// value character by character rather than with a regex — Handoff 061's
/// own hand-rolled extractor undercounted by two because its regex lacked
/// `DOTALL` and so didn't match value text across a `\`-newline
/// continuation; scanning characters (not `.`) has no such mode to forget.
/// Every `\`-escape (including `\`-newline continuation) is copied through
/// unresolved rather than decoded — this gate only needs accurate value
/// *boundaries* and raw `{...}` placeholder text, not a fully decoded Rust
/// string, and copying escapes through verbatim can never accidentally
/// terminate the scan early on an escaped quote.
fn extract_i18n_constants(file: &str, content: &str) -> (Vec<I18nConstant>, Vec<I18nConstant>) {
    let mut en = Vec::new();
    let mut ja = Vec::new();
    let marker = "pub const ";
    let mut search_from = 0usize;
    while let Some(rel) = content[search_from..].find(marker) {
        let after_marker = search_from + rel + marker.len();
        search_from = after_marker;
        let rest = &content[after_marker..];
        let lang = if rest.starts_with("EN_") {
            "EN"
        } else if rest.starts_with("JA_") {
            "JA"
        } else {
            continue;
        };
        let after_prefix = &rest[3..];
        let Some(colon_rel) = after_prefix.find(':') else {
            continue;
        };
        let stem = after_prefix[..colon_rel].trim().to_string();
        let after_colon = &after_prefix[colon_rel..];
        let Some(quote_rel) = after_colon.find('"') else {
            continue;
        };
        let value_region = &after_colon[quote_rel + 1..];
        let mut value = String::new();
        let mut chars = value_region.char_indices();
        let mut closed = false;
        while let Some((_, c)) = chars.next() {
            if c == '\\' {
                value.push('\\');
                if let Some((_, escaped)) = chars.next() {
                    value.push(escaped);
                }
                continue;
            }
            if c == '"' {
                closed = true;
                break;
            }
            value.push(c);
        }
        if !closed {
            continue;
        }
        let constant = I18nConstant {
            stem,
            value,
            file: file.to_string(),
        };
        match lang {
            "EN" => en.push(constant),
            "JA" => ja.push(constant),
            _ => unreachable!(),
        }
    }
    (en, ja)
}

/// Handoff 064: the property `en_ja_parity` was named for but never
/// checked. Default-fail, walking `packages/contracts/src/i18n/*.rs` itself
/// rather than trusting a hand-maintained list — the exact defect that let
/// the old test drift to zero real coverage without anyone noticing.
#[test]
fn en_ja_parity_is_derived_from_the_constants_themselves() {
    let files = i18n_module_files();
    assert!(
        files.len() >= 13,
        "expected at least 13 i18n module files, found {} — has the directory moved, or did \
         modules merge? An unexpectedly small count likely means this gate's file walk is \
         broken, not that there is nothing left to check.",
        files.len()
    );

    let mut en_map: std::collections::BTreeMap<String, I18nConstant> =
        std::collections::BTreeMap::new();
    let mut ja_map: std::collections::BTreeMap<String, I18nConstant> =
        std::collections::BTreeMap::new();

    for path in &files {
        let raw = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let stripped = strip_line_comments(&raw);
        let rel = path.file_name().unwrap().to_string_lossy().into_owned();
        let (en, ja) = extract_i18n_constants(&rel, &stripped);
        for constant in en {
            if let Some(prior) = en_map.insert(constant.stem.clone(), constant) {
                panic!(
                    "EN_{} is declared more than once (last seen in {}) — duplicate stems are \
                     not a parity gap, they are a compile-time-shadowing bug waiting to happen",
                    prior.stem, prior.file
                );
            }
        }
        for constant in ja {
            if let Some(prior) = ja_map.insert(constant.stem.clone(), constant) {
                panic!(
                    "JA_{} is declared more than once (last seen in {}) — duplicate stems are \
                     not a parity gap, they are a compile-time-shadowing bug waiting to happen",
                    prior.stem, prior.file
                );
            }
        }
    }

    assert!(
        en_map.len() > 100 && ja_map.len() > 100,
        "expected well over 100 EN_/JA_ constants each, found {} EN_ and {} JA_ — this gate's \
         extractor is likely broken, not that the corpus shrank this much",
        en_map.len(),
        ja_map.len()
    );

    let mut seen_exceptions = std::collections::HashSet::new();
    let mut unpaired: Vec<String> = Vec::new();

    for stem in en_map.keys() {
        if !ja_map.contains_key(stem) {
            match EN_JA_PARITY_EXCEPTIONS.iter().find(|e| e.stem == stem) {
                Some(exc) => {
                    seen_exceptions.insert(exc.stem);
                }
                None => unpaired.push(format!("EN_{stem} has no JA_{stem}")),
            }
        }
    }
    for stem in ja_map.keys() {
        if !en_map.contains_key(stem) {
            match EN_JA_PARITY_EXCEPTIONS.iter().find(|e| e.stem == stem) {
                Some(exc) => {
                    seen_exceptions.insert(exc.stem);
                }
                None => unpaired.push(format!("JA_{stem} has no EN_{stem}")),
            }
        }
    }

    assert!(
        unpaired.is_empty(),
        "these stems are unpaired and not in EN_JA_PARITY_EXCEPTIONS:\n{}\n\nEvery EN_ constant \
         must have a JA_ counterpart with the same stem, and vice versa, or a pinned exception \
         with a written reason — this is the property en_ja_parity was named for and never \
         checked.",
        unpaired.join("\n")
    );

    for exc in EN_JA_PARITY_EXCEPTIONS {
        assert!(
            seen_exceptions.contains(exc.stem),
            "EN_JA_PARITY_EXCEPTIONS names {} ({}) but that stem is not currently unpaired — \
             either it was paired (delete the entry, the table is meant to shrink) or it no \
             longer exists (stale entry, delete it) or it is unpaired on both sides at once \
             (a stop condition per Handoff 064 §11, not something to silently accept)",
            exc.stem,
            exc.reason
        );
    }

    let mut empty_or_whitespace: Vec<String> = Vec::new();
    for constant in en_map.values().chain(ja_map.values()) {
        if constant.value.trim().is_empty() {
            empty_or_whitespace.push(format!(
                "{} in {} is empty or whitespace-only",
                constant.stem, constant.file
            ));
        }
    }
    assert!(
        empty_or_whitespace.is_empty(),
        "these constants are empty or whitespace-only:\n{}",
        empty_or_whitespace.join("\n")
    );

    let mut placeholder_mismatches: Vec<String> = Vec::new();
    for (stem, en_constant) in &en_map {
        let Some(ja_constant) = ja_map.get(stem) else {
            continue;
        };
        let mut en_ph = placeholders(&en_constant.value);
        let mut ja_ph = placeholders(&ja_constant.value);
        en_ph.sort_unstable();
        ja_ph.sort_unstable();
        if en_ph != ja_ph {
            placeholder_mismatches.push(format!(
                "{stem}: EN placeholders {en_ph:?} != JA placeholders {ja_ph:?}"
            ));
        }
    }
    assert!(
        placeholder_mismatches.is_empty(),
        "these pairs disagree on {{...}} placeholders between EN and JA — this is a live \
         rendering-argument-mismatch bug, not a gate gap:\n{}",
        placeholder_mismatches.join("\n")
    );

    // Handoff 070 Part B: what `i18n_en_ja_parity_count`'s `assert_ne!(en,
    // ja, ...)` used to check, folded in here. Structural equality of the
    // two string values only — this must never grow into asserting anything
    // about what a string *says* (RFC-054 owns wording; RFC-081 §3.2's
    // generic recovery message and RFC-082 §4's suspension page carry
    // deliberate non-disclosure properties this gate must not touch).
    let mut seen_identical_exceptions = std::collections::HashSet::new();
    let mut unexpected_identical: Vec<String> = Vec::new();
    for (stem, en_constant) in &en_map {
        let Some(ja_constant) = ja_map.get(stem) else {
            continue;
        };
        if en_constant.value == ja_constant.value {
            match EN_JA_IDENTICAL_EXCEPTIONS.iter().find(|e| e.stem == stem) {
                Some(exc) => {
                    seen_identical_exceptions.insert(exc.stem);
                }
                None => unexpected_identical.push(format!(
                    "{stem}: EN and JA are identical (likely copy-paste): {:?}",
                    en_constant.value
                )),
            }
        }
    }
    assert!(
        unexpected_identical.is_empty(),
        "these pairs are unexpectedly identical between EN and JA and not in \
         EN_JA_IDENTICAL_EXCEPTIONS:\n{}\n\nA second identical pair is a stop condition \
         (Handoff 070 §12), not a second table row — escalate rather than adding an entry.",
        unexpected_identical.join("\n")
    );
    for exc in EN_JA_IDENTICAL_EXCEPTIONS {
        assert!(
            seen_identical_exceptions.contains(exc.stem),
            "EN_JA_IDENTICAL_EXCEPTIONS names {} ({}) but that stem's EN and JA values are not \
             currently identical — either they diverged (delete the entry) or the stem no \
             longer exists (stale entry, delete it)",
            exc.stem,
            exc.reason
        );
    }
}

/// Handoff 041 §7.4: same shape as `LOCALIZATION_EXCEPTIONS` (explicit key,
/// written reason, stale-entry assertion). Keyed by class name, not by
/// file: the property is about the class being safe to rename/restyle
/// without checking every caller, and one class can be legitimately used
/// from more than one non-admin-directory file (`cz-admin-title`, from both
/// `templates.rs` and `export.rs`).
///
/// **What this proves, and what it does not.** The property that actually
/// matters is "an admin-named class is rendered on a non-admin surface" —
/// a page-level fact. This gate's proxy is "referenced from a file outside
/// `handlers/admin/`" — a file-level fact. They differ in both directions:
/// `templates.rs` and `export.rs` are themselves admin-only surfaces that
/// merely live outside the `admin/` directory, so the proxy flags them
/// anyway, which is why they are exceptions here rather than renames. A
/// class referenced only from a shared `render/` helper would be invisible
/// to the page-level question (which page is it rendered on?) but *would*
/// trip this file-level proxy — correctly, since a helper's callers are not
/// enumerable by reading the helper alone, and that is exactly the case
/// that needs a human decision, not a silent pass.
struct AdminClassLeakException {
    class: &'static str,
    reason: &'static str,
}

const ADMIN_CLASS_LEAK_EXCEPTIONS: &[AdminClassLeakException] = &[
    AdminClassLeakException {
        class: "cz-admin-field-label",
        reason: "admin surface outside the admin/ directory",
    },
    AdminClassLeakException {
        class: "cz-admin-invite-flash",
        reason: "admin surface outside the admin/ directory",
    },
    AdminClassLeakException {
        class: "cz-admin-invites-body",
        reason: "admin surface outside the admin/ directory",
    },
    AdminClassLeakException {
        class: "cz-admin-title",
        reason: "admin surface outside the admin/ directory",
    },
    AdminClassLeakException {
        class: "cz-admin-title--snug",
        reason: "admin surface outside the admin/ directory",
    },
    AdminClassLeakException {
        class: "cz-admin-title--tight",
        reason: "admin surface outside the admin/ directory",
    },
];

/// Every `cz-admin-*` token in `content`, comments stripped first — the
/// third time a gate in this family has needed that discipline (the flash
/// gate and `count_inline_styles` both hit the same trap: a doc comment
/// *about* the pattern matching the pattern itself).
fn find_admin_classes(content: &str) -> Vec<String> {
    let stripped = strip_line_comments(content);
    let mut found = Vec::new();
    let mut search_from = 0;
    while let Some(rel_idx) = stripped[search_from..].find("cz-admin-") {
        let start = search_from + rel_idx;
        let end = stripped[start..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .map(|i| start + i)
            .unwrap_or(stripped.len());
        found.push(stripped[start..end].to_string());
        search_from = end;
    }
    found
}

#[test]
fn rfc041_admin_named_classes_stay_inside_the_admin_directory_or_are_excepted() {
    // Handoff 041: `cz-admin-field-input` was rendered on `join.rs` — the
    // anonymous, unauthenticated invite-redemption page — for as long as
    // RFC-075 Slice 4 existed, and hand-enumeration missed it (Handoff 040
    // renamed the one instance a review happened to spot). `cz-admin-plain-link`
    // had the identical shape and was found only because the same review
    // swept every `cz-admin-*` class afterward. Default-fail, so a third
    // instance cannot happen the same way: any `cz-admin-*` class referenced
    // from a file outside `handlers/admin/` must be named in
    // `ADMIN_CLASS_LEAK_EXCEPTIONS` with a reason, or the walk fails.
    let files = handlers_and_render_files();
    let src_dir = workers_ssr_src_dir();
    let mut seen_exception_classes = std::collections::HashSet::new();
    let mut unexpected: Vec<String> = Vec::new();

    for path in &files {
        let rel = path
            .strip_prefix(&src_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.starts_with("handlers/admin/") {
            continue;
        }
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        for class in find_admin_classes(&content) {
            match ADMIN_CLASS_LEAK_EXCEPTIONS
                .iter()
                .find(|e| e.class == class)
            {
                Some(exc) => {
                    seen_exception_classes.insert(exc.class);
                }
                None => {
                    unexpected.push(format!("{rel}: {class}"));
                }
            }
        }
    }

    assert!(
        unexpected.is_empty(),
        "an admin-named class is referenced from a file outside handlers/admin/ and is not in \
         ADMIN_CLASS_LEAK_EXCEPTIONS:\n{}\n\
         Either this class is only ever meant for handlers/admin/ — in which case the reference \
         above is a real leak (RFC-075 Slice 4's cz-admin-field-input on join.rs, and \
         cz-admin-plain-link, were both exactly this shape) and should be renamed to drop the \
         admin- prefix — or this file is itself an admin-only surface that happens to live \
         outside admin/, in which case add a table entry with a written reason.",
        unexpected.join("\n")
    );

    for exc in ADMIN_CLASS_LEAK_EXCEPTIONS {
        assert!(
            seen_exception_classes.contains(exc.class),
            "ADMIN_CLASS_LEAK_EXCEPTIONS names `{}` ({}) but the walk never found it referenced \
             from any qualifying file — stale table entry?",
            exc.class,
            exc.reason
        );
    }
}

#[test]
fn rfc072_communities_and_event_pages_resolve_locale_and_html_lang_together() {
    // Complements the executable Ja/En render tests for calendar.rs/matrix.rs
    // (which have extractable pure render functions): communities.rs's
    // get_communities and event.rs's get_event_detail are async, D1-bound
    // handlers with no such extractable function, so — like the RFC-070 and
    // RFC-072 settings-page gates above — this checks statically that each
    // resolves a representative string through `i18n::t(locale, ...)` and
    // threads that same `locale` into `page_localized`, so `html lang` and
    // the page's rendered language cannot drift apart.
    assert!(
        COMMUNITIES_HANDLER_SRC.contains("let locale = active_membership.locale;")
            && COMMUNITIES_HANDLER_SRC.contains("i18n::t(locale, i18n::NAV_COMMUNITIES)")
            && COMMUNITIES_HANDLER_SRC.contains("render::page_localized(locale, title, &body)"),
        "communities.rs's get_communities must resolve locale from find_active and thread it into page_localized"
    );
    assert!(
        EVENT_HANDLER_SRC.contains("let locale = membership.locale;")
            && EVENT_HANDLER_SRC.contains("i18n::t(locale, i18n::EVENT_TITLE_HEADER)")
            && EVENT_HANDLER_SRC.contains("render::page_localized(locale, &event.title, &body)"),
        "event.rs's get_event_detail must resolve locale from require_membership and thread it into page_localized"
    );
    // Handoff 030 §7.1/§7.2: the two pages RFC-072 criterion 9 was missing.
    assert!(
        CALENDAR_HANDLER_SRC.contains("let locale = membership.locale;")
            && CALENDAR_HANDLER_SRC.contains("i18n::t(locale, i18n::CALENDAR_TITLE)")
            && CALENDAR_HANDLER_SRC.contains("render::page_localized(locale, title, &body)"),
        "calendar.rs's get_me_calendar must resolve locale from require_membership and thread it into page_localized"
    );
    assert!(
        COMMUNITY_CREATE_HANDLER_SRC.contains("let locale = admin.locale;")
            && COMMUNITY_CREATE_HANDLER_SRC
                .contains("i18n::t(locale, i18n::COMMUNITY_CREATE_TITLE)")
            && COMMUNITY_CREATE_HANDLER_SRC
                .contains("render::page_localized(locale, title, &body)"),
        "community_create.rs's render_form/render_disabled must resolve locale from the authorizing admin membership (Handoff 030 §7.2) and thread it into page_localized"
    );
}

#[test]
fn rfc072_language_setting_is_linked_from_my_page() {
    // RFC-072 Slice C §5.5/§10 acceptance criterion 9: the setting is now
    // reachable. This is the flip side of Slice A/B's "not yet linked"
    // requirement — checked here since no earlier gate asserted the link's
    // absence in code (only the tester checklist tracked it).
    assert!(
        ME_HANDLER_SRC.contains("/c/{cid}/me/language")
            && ME_HANDLER_SRC.contains("i18n::t(locale, i18n::ME_LANGUAGE_TITLE)"),
        "My Page must link to the language setting, added only after §5.1-5.4 closed (handoff §7.1)"
    );
}

/// Drops SQL `--...` line-comment text before scanning — Handoff 083 Part B.
/// The eighth instance of a gate matching its own explanatory prose in this
/// project, and the first in SQL: `rfc072_migration_0011_ui_language_check_is_closed`
/// read the raw file, so writing the word "default" anywhere — including a
/// comment explaining the *absence* of a default — false-tripped it
/// (Handoff 079 hit this and worked around it by wording the comment to
/// avoid the word). `strip_line_comments` above is the `//` counterpart for
/// Rust source; this is the SQL one, applied only to the DEFAULT/UPDATE
/// checks below — the structural checks (`ALTER TABLE`/`ADD COLUMN`/`CHECK`)
/// keep reading the raw statement, since comments are irrelevant to whether
/// the real SQL contains them.
fn strip_sql_line_comments(content: &str) -> String {
    content
        .lines()
        .map(|line| line.split("--").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn rfc072_migration_0011_ui_language_check_is_closed() {
    assert!(
        MIGRATION_0011_SRC.contains("ALTER TABLE community_memberships")
            && MIGRATION_0011_SRC.contains("ADD COLUMN ui_language TEXT"),
        "RFC-072 migration 0011 must add a nullable ui_language column to community_memberships"
    );
    assert!(
        MIGRATION_0011_SRC.contains("CHECK(ui_language IN ('ja', 'en') OR ui_language IS NULL)"),
        "RFC-072 migration 0011's CHECK set must stay exactly {{'ja', 'en', NULL}} — closed, no third value, no backfill"
    );
    // Comments stripped for these two checks only — a SQL comment
    // explaining the absence of a default (or an update) must not be able
    // to trip a check about the real statement. Deliberately broad
    // (`DEFAULT` anywhere, not `DEFAULT '` or similar): a real DEFAULT
    // clause on ui_language would silently backfill every existing row,
    // defeating RFC-072's no-backfill design, so narrowing the pattern is
    // not an option even though it would also dodge the false-trip.
    let stripped = strip_sql_line_comments(MIGRATION_0011_SRC);
    assert!(
        !stripped.to_ascii_uppercase().contains("UPDATE ")
            && !stripped.to_ascii_uppercase().contains("DEFAULT"),
        "RFC-072 migration 0011 must not backfill or default any existing row — NULL means the caller resolves Locale::PRODUCT_DEFAULT (RFC-085/Handoff 079), not a value baked into the row"
    );
}

#[test]
fn rfc068_calendar_matrix_csv_export_contract_is_guarded() {
    assert!(
        COMMUNITIES_HANDLER_SRC.contains("token_purpose::CALENDAR_MATRIX_CSV_EXPORT")
            && COMMUNITIES_HANDLER_SRC.contains("calendar_matrix_csv_bound_resource")
            && COMMUNITIES_HANDLER_SRC.contains("post_matrix_export_audit")
            && COMMUNITIES_HANDLER_SRC.contains("form_token::set_result")
            && COMMUNITY_HANDLER_SRC.contains("\"calendar/matrix-export/audit\""),
        "RFC-068 matrix CSV export must use a dedicated month-bound single-use token and audited admin POST route"
    );
    assert!(
        COMMUNITIES_HANDLER_SRC.contains("\"calendar_matrix_csv.export_requested\"")
            && !COMMUNITIES_HANDLER_SRC.contains("\"calendar_matrix_csv.exported\"")
            && COMMUNITIES_HANDLER_SRC.contains("\"month\"")
            && COMMUNITIES_HANDLER_SRC.contains("\"export_type\""),
        "RFC-068 audit action must be metadata-only export_requested, not exported"
    );
    assert!(
        COMMUNITIES_MATRIX_SRC.contains("data-calendar-matrix-export-button")
            && COMMUNITIES_MATRIX_SRC.contains("data-calendar-matrix-export=\\\"true\\\"")
            && COMMUNITIES_MATRIX_SRC.contains("data-export-value")
            && COMMUNITIES_MATRIX_SRC.contains("data-member-name")
            && COMMUNITIES_MATRIX_SRC.contains("data-date")
            && COMMUNITIES_MATRIX_SRC.contains("i18n::CALENDAR_MATRIX_CSV_EXPORT"),
        "RFC-068 admin matrix markup must carry the reviewed export controls and explicit cell values"
    );
    assert!(
        APP_JS_SOURCE.contains("matrixCsvFromTable")
            && APP_JS_SOURCE.contains("requestMatrixCsvAudit")
            && APP_JS_SOURCE.contains("downloadMatrixCsv")
            && APP_JS_SOURCE.contains("URL.createObjectURL")
            && APP_JS_SOURCE.contains("new Blob")
            && APP_JS_SOURCE.contains("fetch(button.dataset.auditUrl")
            && APP_JS_SOURCE.contains("/^[\\s]*[=+\\-@]/"),
        "RFC-068 CSV must be generated client-side from rendered matrix after audit request, with formula hardening"
    );
    assert!(
        !APP_JS_SOURCE.contains("/export/csv") && !COMMUNITY_HANDLER_SRC.contains("export/csv"),
        "RFC-068 must not add a server CSV/data export endpoint"
    );
}

#[test]
fn rfc059_calendar_create_from_day_is_route_backed() {
    assert!(
        // Handoff 049 §4.1: get_communities now gates through
        // authz::require_membership (which itself resolves find_active)
        // instead of calling membership_db::find_active directly — the
        // property this checks (admin status derived from an active
        // membership) is unchanged, only the call site moved.
        COMMUNITIES_SRC.contains("authz::require_membership")
            && COMMUNITIES_SRC.contains("membership.role == \"admin\"")
            && COMMUNITIES_SRC.contains("can_create_event"),
        "Calendar create-from-day action must be rendered only for active admins"
    );
    assert!(
        COMMUNITIES_SRC.contains("/admin/events/new?day={day}")
            && COMMUNITIES_SRC.contains("i18n::CALENDAR_CREATE_ON_DAY"),
        "Selected Calendar days must expose a route-backed create-event link"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("valid_prefill_day")
            && ADMIN_EVENTS_SRC.contains("query_pairs()")
            && ADMIN_EVENTS_SRC.contains("\"day\"")
            && ADMIN_EVENTS_SRC.contains("prefill_day.as_deref()"),
        "Create Event must validate and prefill the Calendar-selected day"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("admin_events_new_next")
            && COMMUNITY_HANDLER_SRC.contains("admin_events_new:")
            && COMMUNITY_HANDLER_SRC.contains("admin_events_new_destination"),
        "Create Event switcher must preserve a Calendar-selected day when switching communities"
    );
}

#[test]
fn rfc051_event_edit_semantics_are_details_only_for_multi_day() {
    assert!(
        ADMIN_EVENTS_SRC.contains("fn event_schedule_editable")
            && ADMIN_EVENTS_SRC.contains("days.len() == 1 && !event_is_recurring(event)")
            && ADMIN_EVENTS_SRC.contains("repeat_rule != \"none\"")
            && ADMIN_EVENTS_SRC.contains("repeat_count.is_some()"),
        "RFC-051 schedule editing must be limited to one-day non-recurring events"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("render_single_day_edit_fields")
            && ADMIN_EVENTS_SRC.contains("render_details_only_event_edit_fields")
            && ADMIN_EVENTS_SRC.contains("render_schedule_summary"),
        "RFC-051 edit UI must split single-day edit from details-only edit with a schedule summary"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("ADMIN_EDIT_MULTI_DAY_HELPER")
            && ADMIN_EVENTS_SRC.contains("ADMIN_EDIT_RECURRING_HELPER")
            && ADMIN_EVENTS_SRC.contains("ADMIN_EDIT_RESPONSES_PRESERVED"),
        "Details-only edit must explain what can be changed and that schedule/attendance stay unchanged"
    );
    let details_only_src = ADMIN_EVENTS_SRC
        .split("fn render_details_only_event_edit_fields")
        .nth(1)
        .and_then(|s| s.split("fn render_error_html").next())
        .expect("details-only edit renderer must exist");
    for forbidden in [
        "name=\"day_date\"",
        "name=\"starts_at\"",
        "name=\"ends_at\"",
        "name=\"repeat_rule\"",
        "name=\"repeat_count\"",
    ] {
        assert!(
            !details_only_src.contains(forbidden),
            "Details-only edit form must not render schedule or recurrence control {forbidden}"
        );
    }
    assert!(
        ADMIN_EVENTS_SRC.contains("edit_post_contains_schedule_fields")
            && ADMIN_EVENTS_SRC.contains("ADMIN_EDIT_SCHEDULE_NOT_EDITABLE")
            && ADMIN_EVENTS_SRC.contains("validate_event_details")
            && RFC079_EVENT_WRITE_DB_SRC.contains("edit_scope"),
        "Details-only POST must reject direct schedule fields, validate only details, and audit the edit scope"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("ADMIN_CANCEL_EVENT_BODY_ALL_DAYS")
            && ADMIN_EVENTS_SRC.contains("ADMIN_CANCEL_EVENT_CONFIRM_ALL_DAYS"),
        "Cancellation confirmation must state whole-event scope for multi-day/recurring events"
    );
}

#[test]
fn rfc060_cancelled_event_recreate_is_admin_only_and_details_only() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("\"recreate\"")
            && COMMUNITY_HANDLER_SRC.contains("get_recreate_event"),
        "RFC-060 must route GET /c/:cid/admin/events/:eid/recreate"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("pub async fn get_recreate_event")
            && ADMIN_EVENTS_SRC.contains("require_admin")
            && ADMIN_EVENTS_SRC.contains("event_can_seed_recreate(&event)")
            && ADMIN_EVENTS_SRC.contains("token_purpose::CREATE_EVENT"),
        "Recreate GET must require an active same-community admin, a cancelled source, and a create token"
    );
    assert!(
        EVENT_HANDLER_SRC.contains("membership.is_admin() && event.status == \"cancelled\"")
            && EVENT_HANDLER_SRC.contains("i18n::ADMIN_RECREATE_EVENT_ACTION")
            && EVENT_HANDLER_SRC.contains("/admin/events/{eid}/recreate"),
        "Event Detail must show the recreate action only to admins on cancelled events"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("copy_source_event_id")
            && ADMIN_EVENTS_SRC
                .contains("event_db::find_for_community(&db, &source_id, community_id)")
            && ADMIN_EVENTS_SRC.contains("return render::not_found()")
            && ADMIN_EVENTS_SRC.contains("EventCreationMode::CancelledRecreate")
            && ADMIN_EVENTS_SRC.contains("source_event_id: Some(source_id.clone())"),
        "Create POST must re-check source event community/status and record safe provenance"
    );
    let recreate_fields_src = ADMIN_EVENTS_SRC
        .split("fn render_recreate_event_create_fields")
        .nth(1)
        .and_then(|s| s.split("fn render_single_day_edit_fields").next())
        .expect("recreate form renderer must exist");
    assert!(
        recreate_fields_src.contains("ADMIN_RECREATE_EVENT_HELPER")
            && recreate_fields_src.contains("event.location.as_deref()")
            && recreate_fields_src.contains("event.description.as_deref()"),
        "Recreate form must explain the boundary and prefill only title/location/description"
    );
    for copied_schedule in [
        "event.repeat_rule",
        "event.repeat_count",
        "day_date: Some",
        "starts_at: Some",
        "ends_at: Some",
    ] {
        assert!(
            !recreate_fields_src.contains(copied_schedule),
            "Recreate form must not copy schedule/recurrence field {copied_schedule}"
        );
    }
}

#[test]
fn rfc066_event_copy_is_admin_reviewed_prefill_not_clone() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("\"copy\"")
            && COMMUNITY_HANDLER_SRC.contains("get_copy_event"),
        "RFC-066 must route GET /c/:cid/admin/events/:eid/copy"
    );
    assert!(
        ADMIN_EVENTS_COPY_SRC.contains("pub async fn get_copy_event")
            && ADMIN_EVENTS_COPY_SRC.contains("require_admin")
            && ADMIN_EVENTS_COPY_SRC.contains("event_db::find_for_community")
            && ADMIN_EVENTS_COPY_SRC.contains("event_db::days_for_event")
            && ADMIN_EVENTS_COPY_SRC.contains("series_db::find_for_event")
            && ADMIN_EVENTS_COPY_SRC.contains("token_purpose::CREATE_EVENT"),
        "Copy GET must require an active same-community admin and load only scoped event/day/series source data"
    );
    for forbidden_source in [
        "attendance",
        "event_note",
        "invite",
        "audit::",
        "form_token",
    ] {
        assert!(
            !ADMIN_EVENTS_COPY_SRC.contains(forbidden_source),
            "Copy source prefill must not load {forbidden_source}"
        );
    }
    assert!(
        EVENT_HANDLER_SRC.contains("membership.is_admin()")
            && EVENT_HANDLER_SRC.contains("i18n::ADMIN_COPY_EVENT_ACTION")
            && EVENT_HANDLER_SRC.contains("/admin/events/{eid}/copy"),
        "Event Detail must expose the copy action to active admins"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("copy_mode")
            && ADMIN_EVENTS_SRC.contains("\"event_copy\"")
            && ADMIN_EVENTS_SRC.contains("event_can_seed_copy")
            && ADMIN_EVENTS_SRC.contains("event_can_seed_recreate")
            && ADMIN_EVENTS_SRC.contains("EventCreationMode::CancelledRecreate")
            && ADMIN_EVENTS_SRC.contains("EventCreationMode::EventCopy")
            && ADMIN_EVENTS_SRC.contains("source_event_id: Some(source_id.clone())"),
        "Create POST must separate RFC-066 event-copy provenance from RFC-060 cancelled-event recreate"
    );
    assert!(
        ADMIN_EVENTS_COPY_SRC.contains("ADMIN_COPY_EVENT_RECURRING_PAST")
            && ADMIN_EVENTS_COPY_SRC.contains("ADMIN_COPY_EVENT_RECURRING_WINDOW")
            && ADMIN_EVENTS_COPY_SRC.contains("normal_create_default")
            && ADMIN_EVENTS_COPY_SRC.contains("until >= series.start_day_date.as_str()"),
        "Copy prefill must implement reviewed recurring normalization rules"
    );
}

#[test]
fn note_form_has_counter_element_for_js() {
    // The app.js memo counter targets `.note-counter`. If the rendered form
    // omits that class, the live N/200 counter silently never updates (the
    // button-disable still works, but the visible count does not). This
    // regression shipped in v0.35.x.
    assert!(
        RENDER_SRC.contains("note-counter"),
        "note_form must render an element with class \"note-counter\" so the \
         app.js character counter has a target. Without it the live count is dead."
    );
}

// ── Invite code generation security gates ────────────────────────────────
//
// §7.1: fail-closed randomness. The generator must not silently fall back to
// deterministic output if the OS RNG is unavailable. The previous implementation
// used `.unwrap_or_default()` on `getrandom`, which on failure left the byte
// buffer zeroed, producing the code "AAAAAA". The fix uses `?` propagation.
//
// §7.2: rejection sampling. The alphabet has 31 characters; 256 % 31 = 8.
// The previous implementation used `b % 31`, which over-represents the first
// 8 characters by one count out of every 256 draws. The fix discards bytes
// >= 248 and redraws.

const MEMBERS_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/members.rs");
const JOIN_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/join.rs");
const INVITE_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/invite.rs");
const INVITE_SMOKE_SRC: &str = include_str!("../../../scripts/smoke/invite-redemption.mjs");

#[test]
fn rfc076_one_time_invite_response_isolation_is_pinned() {
    let production = MEMBERS_HANDLER_SRC
        .split("#[cfg(test)]\nmod tests")
        .next()
        .expect("members handler production source must precede tests");
    assert!(
        !production.contains("invites?code=")
            && !production.contains("query_pairs().find(|(k, _)| k == \"code\")"),
        "production must never create or render a plaintext invite-code query handoff"
    );

    let get = compact_brace_block(production, "pub async fn get_invites");
    assert!(
        get.contains("leturl=req.url()?")
            && get.contains("run_invite_get_preflight(invite_get_preflight(")
            && get.contains("ControlFlow::Break(location)=>legacy_query_redirect(&location)")
            && get.contains("get_invites_authenticated(req,env,rid,community_id,flash_code)")
            && !get.contains("require_auth")
            && !get.contains("require_admin")
            && !get.contains("env.d1")
            && !get.contains("issue_token")
            && !get.contains("render_invites_page"),
        "legacy code-query containment must run before every authenticated/application continuation"
    );
    let authenticated = compact_brace_block(production, "async fn get_invites_authenticated");
    assert!(
        authenticated.contains("require_auth")
            && authenticated.contains("require_admin")
            && authenticated.contains("render_invites_page"),
        "clean invite GET must delegate all authenticated work to the private continuation"
    );
    let legacy_redirect = compact_brace_block(production, "fn legacy_query_redirect");
    assert!(
        legacy_redirect.contains("with_status(303)")
            && legacy_redirect.contains("set(\"Location\",location)")
            && legacy_redirect.contains("set(\"Referrer-Policy\",\"no-referrer\")"),
        "legacy query containment must emit a clean 303 with no-referrer"
    );

    let canonical = compact_brace_block(production, "fn canonical_invites_path");
    assert!(
        canonical.contains("byte.is_ascii_alphanumeric()")
            && canonical.contains("matches!(byte,b'_'|b'-')")
            && canonical.contains("\"%{byte:02X}\"")
            && canonical.contains("\"/c/{encoded}/admin/invites\""),
        "canonical invite path must use the reviewed byte allowlist and uppercase percent encoding"
    );
    let runner = compact_brace_block(production, "fn run_invite_get_preflight");
    assert!(
        runner.contains("InviteGetPreflight::Continue=>ControlFlow::Continue(continuation())")
            && runner.contains(
                "InviteGetPreflight::CanonicalRedirect(location)=>ControlFlow::Break(location)"
            ),
        "preflight runner must make the early-return continuation boundary executable"
    );

    assert!(
        production.contains("struct InviteCodeReveal(String);")
            && !production.contains("derive(Debug")
            && !production.contains("impl std::fmt::Display for InviteCodeReveal")
            && !production.contains("Serialize for InviteCodeReveal"),
        "plaintext reveal must remain a narrow private non-formatting/non-serializing type"
    );
    let reveal = compact_brace_block(production, "fn invite_reveal_html");
    assert!(
        reveal.contains("render::escape_html(reveal.as_str())")
            && reveal.contains("i18n::ADMIN_INVITES_REVEAL_WARNING") // RFC-072/Handoff 072 locale-aware accessor
            && !reveal.contains("data-")
            && !reveal.contains("<script"),
        "reveal renderer must place escaped plaintext only in the dedicated text panel"
    );

    let post = compact_brace_block(production, "pub async fn post_generate_invite");
    assert!(
        post.contains("returnredirect(&canonical_invites_path(community_id))")
            && post.contains("letcreated=matchinvite_db::insert_required(")
            && post.contains("Err(_)=>returnrender::service_unavailable()")
            && post.contains("consume_detailed(")
            && post.contains("matches!(consume,ConsumeResult::Replay(_))")
            && post.contains("InviteCodeReveal::new(code)")
            && post.contains("letmutresponse=matchrender_invites_page(")
            && post
                .matches("Err(_)=>returnrender::service_unavailable()")
                .count()
                == 2
            && post.contains("set(\"Cache-Control\",\"no-store,private\")")
            && post.contains("set(\"Referrer-Policy\",\"no-referrer\")")
            && !post.contains("Location")
            && !post.contains("?code="),
        "first generation must render directly with strict headers; replay must use a clean canonical 303"
    );
    assert!(
        INVITE_DB_SRC.contains("AuditAction::InviteCodeGenerated")
            && INVITE_DB_SRC.contains("audit::execute_required(db, mutation, &record)")
            && post.contains("\"member\""),
        "RFC-076 must preserve the central RFC-079 Class A batch and member-only grant"
    );
    assert!(
        MEMBERS_HANDLER_SRC
            .contains("legacy_query_preflight_never_invokes_authenticated_continuation")
            && MEMBERS_HANDLER_SRC
                .contains("code_query_preflight_matches_empty_repeated_and_encoded_keys")
            && MEMBERS_HANDLER_SRC
                .contains("canonical_invite_path_encodes_every_non_allowlisted_byte")
            && MEMBERS_HANDLER_SRC.contains("reveal_html_contains_code_once_and_only_as_text"),
        "focused native tests must pin early ordering, query decoding, path containment, and reveal placement"
    );

    for required in [
        "legacy-code-query-is-contained-before-authentication",
        "redirectRequestHasNoReferrer",
        "admin-generated-invite-is-stored-and-shown-once",
        "directStatus",
        "noStorePrivate",
        "codeFreeBrowserUrl",
        "codeAppearsOnce",
        "consumed-generation-token-replay-is-clean-and-non-mutating",
        "required-audit-failure-rolls-back-and-discloses-nothing",
        "proof_fail_invite_generation_audit",
        "exactlyOneCentralEvent",
        "audit.required_batch_failed",
        "failure_category=storage route_class=class_a",
        "generation-recovers-after-local-trigger-cleanup",
        "DROP TRIGGER IF EXISTS proof_fail_invite_generation_audit",
    ] {
        assert!(
            INVITE_SMOKE_SRC.contains(required),
            "RFC-076 smoke must retain required assertion/evidence marker: {required}"
        );
    }
    for forbidden in [
        "inviteCodeFromLocation",
        "screenshot(adminPage, 'admin-generated-invite-is-stored-and-shown-once')",
        "observed:",
        "inviteRows:",
        "values:",
    ] {
        assert!(
            !INVITE_SMOKE_SRC.contains(forbidden),
            "RFC-076 smoke/report must not retain plaintext-sensitive marker: {forbidden}"
        );
    }
}

// ── RFC-079 Package 0A audit inventory gate ─────────────────────────────

const RFC079_ASSERTION_FIXTURE_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit_change_assertion.sql");
const RFC079_ASSERTION_WORKER_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit-assertion-worker.mjs");
const RFC079_ASSERTION_RUNNER_SRC: &str = include_str!("../../../scripts/test-audit-assertion.mjs");
const RFC079_AUDIT_CORE_SRC: &str = include_str!("../../../workers/ssr/src/audit.rs");
const RFC079_MIGRATION_SRC: &str = include_str!("../../../migrations/0010_audit_integrity.sql");
const RFC079_MIGRATION_RUNNER_SRC: &str = include_str!("../../../scripts/test-audit-migration.mjs");
const RFC079_AUDIT_POLICY_SRC: &str = include_str!("../../../docs/src/maintainer/audit-policy.md");
const RFC079_BACKUP_RECOVERY_SRC: &str =
    include_str!("../../../docs/src/maintainer/backup-recovery.md");
const RFC079_DEPLOYMENT_SRC: &str = include_str!("../../../docs/src/shared/deployment.md");
const RFC079_OPERATIONS_SRC: &str = include_str!("../../../docs/src/maintainer/operations.md");
const RFC079_RFC014_SRC: &str =
    include_str!("../../../rfcs/done/014-observability-audit-and-privacy-logging.md");
const RFC079_RFC052_SRC: &str =
    include_str!("../../../rfcs/done/052-audit-retention-and-operator-access-policy.md");
const RFC079_RFC071_SRC: &str =
    include_str!("../../../rfcs/done/071-application-threat-model-and-form-security-baseline.md");
const RFC079_RFC050_SRC: &str =
    include_str!("../../../rfcs/accepted/050-staging-runtime-verification-evidence-pack.md");
const RFC079_THREAT_MODEL_SRC: &str =
    include_str!("../../../docs/src/developer/security-threat-model.md");
const RFC079_ARCHITECTURE_SRC: &str = include_str!("../../../docs/src/developer/architecture.md");
const RFC079_RELEASE_CHECKLIST_SRC: &str =
    include_str!("../../../docs/src/tester/release-checklist.md");
const RFC079_ATTENDANCE_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/attendance.rs");
const RFC079_EVENT_WRITE_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/event_write.rs");
const RFC079_EVENT_NOTE_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/event_note.rs");
const RFC079_EVENT_TEMPLATE_DB_SRC: &str =
    include_str!("../../../workers/ssr/src/db/event_template.rs");
const RFC079_TEMPLATES_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/templates.rs");
const RFC079_NOTE_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/events/notes.rs");
const RFC079_ATOMICITY_RUNNER_SRC: &str = include_str!("../../../scripts/test-audit-atomicity.mjs");
const RFC079_ATOMICITY_WORKER_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit-atomicity-worker.mjs");
const RFC079_ATOMICITY_FIXTURE_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit_atomicity.sql");
const RFC079_BOUNDARY_RUNNER_SRC: &str = include_str!("../../../scripts/test-audit-boundaries.mjs");
const RFC079_CLASS_A_FAILURE_RUNNER_SRC: &str =
    include_str!("../../../scripts/test-audit-class-a-failures.mjs");
const RFC079_BOUNDARY_WORKER_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit-boundaries-worker.mjs");
const RFC079_BOUNDARY_FIXTURE_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit_response_boundaries.sql");
const RFC079_COMMUNITY_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/community.rs");
const PACKAGE_JSON_SRC: &str = include_str!("../../../package.json");
const RECURRENCE_V2_SMOKE_SRC: &str = include_str!("../../../scripts/smoke/recurrence-v2.mjs");

#[derive(Default)]
struct AuditSourceScan {
    direct_inserts: std::collections::BTreeMap<String, usize>,
    assertion_table_refs: Vec<String>,
    operation_id_refs: Vec<String>,
    compatibility_refs: Vec<String>,
    ignored_audit_results: Vec<String>,
    background_audit_refs: Vec<String>,
}

fn compact_source(value: &str) -> String {
    value.split_whitespace().collect()
}

fn compact_brace_block(source: &str, marker: &str) -> String {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("missing source marker {marker:?}"));
    let open = source[start..]
        .find('{')
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("source marker {marker:?} has no opening brace"));
    let mut depth = 0usize;
    let mut end = None;
    for (offset, ch) in source[open..].char_indices() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth
                .checked_sub(1)
                .expect("brace parser encountered an unmatched closing brace");
            if depth == 0 {
                end = Some(open + offset + ch.len_utf8());
                break;
            }
        }
    }
    let end = end.unwrap_or_else(|| panic!("source marker {marker:?} has unbalanced braces"));
    compact_source(&source[start..end])
}

fn collect_rust_sources(
    root: &std::path::Path,
    directory: &std::path::Path,
    output: &mut Vec<(String, String)>,
) {
    let mut entries = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to scan {}: {error}", directory.display()))
        .map(|entry| entry.expect("Rust source directory entry must be readable"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .expect("Rust source entry type must be readable");
        if file_type.is_dir() {
            collect_rust_sources(root, &path, output);
        } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            let relative = path
                .strip_prefix(root)
                .expect("scanned Rust source must stay under workers/ssr/src")
                .to_string_lossy()
                .replace('\\', "/");
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
            output.push((relative, source));
        }
    }
}

fn scan_audit_sources(sources: &[(String, String)]) -> AuditSourceScan {
    let mut scan = AuditSourceScan::default();
    for (path, source) in sources {
        let direct_count = source.matches("INSERT INTO audit_log").count();
        if direct_count > 0 {
            scan.direct_inserts.insert(path.clone(), direct_count);
        }
        if source.contains("audit_change_assertions") {
            scan.assertion_table_refs.push(path.clone());
        }
        if source.contains("operation_id") {
            scan.operation_id_refs.push(path.clone());
        }
        if source.contains("LegacyAuditAction")
            || source.contains("LegacyAuditMetadata")
            || source.contains("write_legacy")
        {
            scan.compatibility_refs.push(path.clone());
        }
        if source.contains("let _ = audit::") || source.contains("let _ = crate::audit::") {
            scan.ignored_audit_results.push(path.clone());
        }
        let compact = compact_source(source);
        if (compact.contains("waitUntil")
            || compact.contains("wait_until")
            || compact.contains("spawn_local"))
            && compact.contains("audit")
        {
            scan.background_audit_refs.push(path.clone());
        }
    }
    scan
}

fn audit_action_owners<'a>(sources: &'a [(String, String)], variant: &str) -> Vec<&'a str> {
    let needle = format!("AuditAction::{variant}");
    sources
        .iter()
        .filter(|(path, source)| {
            path != "audit.rs" && !source.contains("LegacyAuditAction") && source.contains(&needle)
        })
        .map(|(path, _)| path.as_str())
        .collect()
}

#[test]
fn rfc079_class_a_failure_telemetry_is_centrally_and_exhaustively_owned() {
    let production_core = RFC079_AUDIT_CORE_SRC
        .split("#[cfg(test)]\nmod tests")
        .next()
        .expect("audit.rs production source must precede its test module");
    assert_eq!(
        production_core
            .matches("audit.required_batch_failed")
            .count(),
        1,
        "audit.rs must own exactly one production Class A event literal"
    );
    for required in [
        "RequiredBatch",
        "\"class_a\"",
        "INVALID_REQUEST_ID",
        "\"invalid_request_id\"",
        "safe_event_request_id",
        "required_record_with_sink",
        "owned_record_with_sink",
        "emit_failure_with",
        "log_class_a_failure",
    ] {
        assert!(
            production_core.contains(required),
            "Class A failure owner is missing {required}"
        );
    }

    let classification = compact_brace_block(production_core, "pub(crate) const fn is_class_a");
    let class_a_actions = [
        "CommunityCreated",
        "MembershipCreatedFirstAdmin",
        "MembershipDisplayNameUpdated",
        "InviteCodeGenerated",
        "InviteCodeRevoked",
        "InviteCodeRedeemed",
        "MembershipRelinkCodeCreated",
        "MembershipRelinkRedeemed",
        "OperatorRecoveryAdminRelinkCreated",
        "MembershipRemoved",
        "MembershipPromotedToAdmin",
        "MembershipDemotedToMember",
        "EventCreated",
        "EventEdited",
        "EventCancelled",
        "EventOccurrenceCancelled",
        "AttendanceAdminOverride",
        "AttendanceAdminSetAttended",
        "EventNoteAdminHidden",
        "CalendarFeedTokenGenerated",
        "CalendarFeedTokenRevoked",
        "EventTemplateCreated",
        "EventTemplateDeleted",
    ];
    for action in class_a_actions {
        assert!(
            classification.contains(&format!("Self::{action}")),
            "Class A classification lost {action}"
        );
    }
    for action in [
        "CommunityExportAuthorized",
        "CalendarMatrixCsvExportRequested",
        "SessionLogout",
    ] {
        assert!(
            !classification.contains(&format!("Self::{action}")),
            "Class B/C action {action} must not classify as Class A"
        );
    }

    let executors = [
        "execute_required(",
        "execute_required_bounded(",
        "execute_required_attendance_override(",
        "execute_required_batch(",
        "execute_required_tail(",
        "execute_asserted_required(",
        "execute_required_standalone(",
    ];
    assert_eq!(
        production_core
            .matches("pub(crate) async fn execute_")
            .count(),
        executors.len(),
        "a new central audit executor must be added to the exhaustive failure-ownership gate"
    );
    for executor in executors {
        let block = compact_brace_block(production_core, executor);
        assert!(
            block.contains("log_class_a_failure"),
            "{executor} must route failures through the central Class A owner"
        );
        assert!(
            block.contains("AuditFailureCategory::Construction")
                && block.contains("AuditFailureCategory::Storage")
                && block.contains("db.batch"),
            "{executor} must cover setup, D1, and post-D1 failure phases"
        );
        assert!(
            !block.contains("?;"),
            "{executor} contains a bare fallible escape that can bypass telemetry"
        );
    }

    let class_b = compact_brace_block(production_core, "pub(crate) async fn write_pre_disclosure");
    let class_c = compact_brace_block(
        production_core,
        "pub(crate) async fn write_logout_secondary",
    );
    assert!(
        class_b.contains("owned_record_with_sink")
            && class_b.contains("AuditFailureEvent::PreDisclosure")
            && !class_b.contains("RequiredBatch"),
        "Class B must retain exclusive PreDisclosure failure ownership"
    );
    assert!(
        class_c.contains("owned_record_with_sink")
            && class_c.contains("AuditFailureEvent::SecondaryWrite")
            && !class_c.contains("RequiredBatch"),
        "Class C must retain exclusive SecondaryWrite failure ownership"
    );

    let formatter = compact_brace_block(production_core, "fn format_failure_event");
    assert!(
        formatter.contains("safe_event_request_id(request_id)")
            && !formatter.contains("metadata")
            && !formatter.contains("community_id")
            && !formatter.contains("actor_membership_id")
            && !formatter.contains("target_id")
            && !formatter.contains("error")
            && !formatter.contains("sql")
            && !formatter.contains("bind"),
        "failure formatter must use the safe request-ID boundary and only bounded fields"
    );

    let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../workers/ssr/src")
        .canonicalize()
        .expect("workers/ssr/src must exist for the Class A ownership scan");
    let mut sources = Vec::new();
    collect_rust_sources(&source_root, &source_root, &mut sources);
    let outside_owners = sources
        .iter()
        .filter(|(path, source)| {
            path != "audit.rs"
                && (source.contains("audit.required_batch_failed")
                    || source.contains("AuditFailureEvent::RequiredBatch")
                    || source.contains("log_class_a_failure"))
        })
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    assert!(
        outside_owners.is_empty(),
        "Class A failure telemetry escaped audit.rs: {outside_owners:?}"
    );

    let community = compact_source(RFC079_COMMUNITY_DB_SRC);
    assert!(
        community.contains("letprimary_audit=audit::required_record(")
            && community.contains("AuditAction::CommunityCreated")
            && community.contains("letmembership_audit=audit::required_record(")
            && community.contains("AuditAction::MembershipCreatedFirstAdmin")
            && community.contains("&primary_audit,&[membership_audit]"),
        "community creation must pin community.created as primary and membership.created_first_admin as additional"
    );

    let runner = compact_source(RFC079_CLASS_A_FAILURE_RUNNER_SRC);
    for required in [
        "prepareIsolatedWorkerTest(",
        "isolated.spawnDev(",
        "/c/${communityId}/admin/invites",
        "/communities/new",
        "/join/profile",
        "audit.required_batch_failed",
        "DROPTRIGGERIFEXISTS",
        "awaitisolated.cleanup()",
    ] {
        assert!(
            runner.contains(required),
            "compiled Class A proof runner lost containment or route evidence: {required}"
        );
    }
    let isolated = compact_source(ISOLATED_WORKER_TEST_SRC);
    for required in [
        "mkdtemp(",
        "--persist-to",
        "wrangler.toml",
        "'dev'",
        "'--local'",
        "'127.0.0.1'",
        "awaitrm(container,{recursive:true,force:true",
    ] {
        assert!(
            isolated.contains(required),
            "shared compiled-Worker fixture lost containment evidence: {required}"
        );
    }
    for forbidden in [
        "--remote",
        "audit-atomicity-worker.mjs",
        "audit-assertion-worker.mjs",
        "writeFile(",
        "copyFile(",
    ] {
        if forbidden == "--remote" {
            assert_eq!(
                RFC079_CLASS_A_FAILURE_RUNNER_SRC.matches(forbidden).count(),
                1,
                "the only --remote occurrence must remain the refusal guard"
            );
        } else {
            assert!(
                !RFC079_CLASS_A_FAILURE_RUNNER_SRC.contains(forbidden),
                "compiled Class A proof must not create or reuse a synthetic Worker: {forbidden}"
            );
        }
    }
    assert!(
        PACKAGE_JSON_SRC.contains(
            "\"test:audit-class-a-failures\": \"node scripts/test-audit-class-a-failures.mjs\""
        ),
        "package.json must expose the domain-named Class A proof command"
    );
}

#[test]
fn rfc079_package0a_current_audit_inventory_is_pinned() {
    let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../workers/ssr/src")
        .canonicalize()
        .expect("workers/ssr/src must exist for the RFC-079 repository-wide source gate");
    let mut sources = Vec::new();
    collect_rust_sources(&source_root, &source_root, &mut sources);
    assert!(
        sources.len() >= 78,
        "RFC-079 source gate scanned too few Rust files; repository root may be wrong"
    );
    let scan = scan_audit_sources(&sources);

    let expected_direct_inserts =
        std::collections::BTreeMap::from([("audit.rs".to_owned(), 1usize)]);
    assert_eq!(
        scan.direct_inserts, expected_direct_inserts,
        "repository-wide audit INSERT distribution changed; audit.rs must remain the sole production audit INSERT owner"
    );

    assert!(
        scan.compatibility_refs.is_empty()
            && scan.ignored_audit_results.is_empty()
            && scan.background_audit_refs.is_empty(),
        "Package 7 removal boundary rejects compatibility, ignored-result, and background audit surfaces: compatibility={:?}, ignored={:?}, background={:?}",
        scan.compatibility_refs,
        scan.ignored_audit_results,
        scan.background_audit_refs,
    );
    assert!(
        scan.assertion_table_refs == vec!["audit.rs".to_owned()]
            && scan.operation_id_refs == vec!["audit.rs".to_owned()],
        "Package 7 assertion table/operation IDs must be owned only by audit.rs: table={:?}, operation_id={:?}",
        scan.assertion_table_refs,
        scan.operation_id_refs
    );

    let source_by_path = sources
        .iter()
        .map(|(path, source)| (path.as_str(), compact_source(source)))
        .collect::<std::collections::BTreeMap<_, _>>();
    let role_source = sources
        .iter()
        .find(|(path, _)| path == "handlers/admin/role_transfer.rs")
        .map(|(_, source)| source.as_str())
        .expect("role-transfer audit source must exist");
    assert_eq!(
        compact_brace_block(role_source, "enum RoleMutation"),
        "enumRoleMutation{Promote,Demote,}",
        "RoleMutation variants changed; reconcile the accepted dynamic audit inventory"
    );
    for required in [
        "AuditAction::MembershipPromotedToAdmin",
        "AuditAction::MembershipDemotedToMember",
    ] {
        assert!(
            source_by_path["db/membership.rs"].contains(&compact_source(required)),
            "role audit-action mapping is missing {required}"
        );
    }
    let class_a = [
        "community.created",
        "membership.created_first_admin",
        "membership.display_name_updated",
        "invite_code.generated",
        "invite_code.revoked",
        "invite_code.redeemed",
        "membership.relink_code_created",
        "membership.relink_redeemed",
        "operator_recovery.admin_relink_created",
        "membership.removed",
        "membership.promoted_to_admin",
        "membership.demoted_to_member",
        "event.created",
        "event.edited",
        "event.cancelled",
        "event.occurrence_cancelled",
        "attendance.admin_override",
        "attendance.admin_set_attended",
        "event_note.admin_hidden",
        "calendar_feed.token_generated",
        "calendar_feed.token_revoked",
        "event_template.created",
        "event_template.deleted",
    ];
    let class_b = [
        "community.export_authorized",
        "calendar_matrix_csv.export_requested",
    ];
    let class_c = ["session.logout"];
    let mut canonical_actions = std::collections::BTreeSet::new();
    for canonical in class_a.into_iter().chain(class_b).chain(class_c) {
        assert!(
            canonical_actions.insert(canonical),
            "duplicate canonical RFC-079 action in inventory: {canonical}"
        );
    }
    assert_eq!(
        canonical_actions.len(),
        26,
        "RFC-079 inventory must remain 23 Class A + 2 Class B + 1 Class C"
    );
    for (variant, expected_owners) in [
        ("CommunityCreated", &["db/community.rs"][..]),
        ("MembershipCreatedFirstAdmin", &["db/community.rs"][..]),
        ("MembershipDisplayNameUpdated", &["handlers/me.rs"][..]),
        ("InviteCodeGenerated", &["db/invite.rs"][..]),
        ("InviteCodeRevoked", &["db/invite.rs"][..]),
        (
            "InviteCodeRedeemed",
            &["db/auth_transaction.rs", "db/invite.rs"][..],
        ),
        ("MembershipRelinkCodeCreated", &["db/relink.rs"][..]),
        ("MembershipRelinkRedeemed", &["db/relink.rs"][..]),
        ("OperatorRecoveryAdminRelinkCreated", &["db/relink.rs"][..]),
        ("MembershipRemoved", &["db/membership.rs"][..]),
        ("MembershipPromotedToAdmin", &["db/membership.rs"][..]),
        ("MembershipDemotedToMember", &["db/membership.rs"][..]),
        ("EventCreated", &["db/event_write.rs"][..]),
        ("EventEdited", &["db/event_write.rs"][..]),
        ("EventCancelled", &["db/event_write.rs"][..]),
        ("EventOccurrenceCancelled", &["db/event_write.rs"][..]),
        ("AttendanceAdminOverride", &["db/attendance.rs"][..]),
        ("AttendanceAdminSetAttended", &["db/attendance.rs"][..]),
        ("EventNoteAdminHidden", &["db/event_note.rs"][..]),
        ("CalendarFeedTokenGenerated", &["db/calendar.rs"][..]),
        ("CalendarFeedTokenRevoked", &["db/calendar.rs"][..]),
        ("EventTemplateCreated", &["db/event_template.rs"][..]),
        ("EventTemplateDeleted", &["db/event_template.rs"][..]),
        ("CommunityExportAuthorized", &["handlers/export.rs"][..]),
        (
            "CalendarMatrixCsvExportRequested",
            &["handlers/communities.rs"][..],
        ),
        ("SessionLogout", &[][..]),
    ] {
        assert_eq!(
            audit_action_owners(&sources, variant),
            expected_owners,
            "typed audit action ownership changed for {variant}; reconcile the RFC-079 inventory"
        );
    }
}

#[test]
fn rfc079_closed_core_and_package7_removal_boundary_are_pinned() {
    for required in [
        "pub(crate) enum AuditAction",
        "pub(crate) enum AuditMetadata",
        "pub(crate) struct AuditRecord",
        "fn sanitize_and_serialize",
        "fn format_event",
        "const MAX_METADATA_DEPTH: usize = 8",
        "const MAX_METADATA_NODES: usize = 128",
        "const MAX_METADATA_BYTES: usize = 2_048",
    ] {
        assert!(
            RFC079_AUDIT_CORE_SRC.contains(required),
            "RFC-079 closed-core contract is missing {required:?}"
        );
    }
    assert!(
        !RFC079_AUDIT_CORE_SRC.contains("pub async fn write(")
            && !RFC079_AUDIT_CORE_SRC.contains("pub(crate) async fn write(")
            && !RFC079_AUDIT_CORE_SRC.contains("action: &str")
            && !RFC079_AUDIT_CORE_SRC.contains("metadata: Option<serde_json::Value>"),
        "Package 7 must not restore an arbitrary string/Value audit writer"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("event=audit.write request_id={}")
            && !RFC079_AUDIT_CORE_SRC.contains("target={}:{}")
            && !RFC079_AUDIT_CORE_SRC.contains("actor={} community={}"),
        "Package 7 structured audit events must exclude raw actor/community/target identifiers"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("earliest deployable code")
            && !RFC079_AUDIT_CORE_SRC.contains("LegacyAuditAction")
            && !RFC079_AUDIT_CORE_SRC.contains("LegacyAuditMetadata")
            && !RFC079_AUDIT_CORE_SRC.contains("write_legacy"),
        "Package 7 must remove the compatibility adapter while preserving the separately reviewed release boundary"
    );
}

#[test]
fn rfc079_package2_migration_and_operator_policy_are_pinned() {
    for required in [
        "CREATE TABLE audit_log_v2",
        "request_id           TEXT NOT NULL",
        "json_valid(metadata_json)",
        "json_type(metadata_json) = 'object'",
        "length(CAST(metadata_json AS BLOB)) <= 2048",
        "CHECK (length(request_id) BETWEEN 1 AND 96)",
        "CREATE TABLE audit_change_assertions",
        "length(operation_id) = 26",
        "changed_count INTEGER NOT NULL CHECK (changed_count = 1)",
        "CREATE INDEX idx_audit_log_community_created_at",
        "CREATE INDEX idx_audit_log_action_created_at",
    ] {
        assert!(
            RFC079_MIGRATION_SRC.contains(required),
            "RFC-079 migration 0010 is missing {required:?}"
        );
    }
    assert!(
        RFC079_MIGRATION_SRC.contains("'legacy'")
            && RFC079_MIGRATION_SRC.contains("'{}'")
            && !RFC079_MIGRATION_SRC.contains("SELECT metadata_json")
            && !RFC079_MIGRATION_SRC.contains("metadata_json FROM audit_log"),
        "migration 0010 must assign legacy request IDs and empty metadata without reading legacy metadata"
    );
    assert_eq!(
        RFC079_MIGRATION_SRC.matches("EXCEPT").count(),
        2,
        "migration 0010 must compare preserved core rows in both directions"
    );
    let verify_position = RFC079_MIGRATION_SRC
        .find("'core_rows_reverse'")
        .expect("migration must contain reverse core-row verification");
    let swap_position = RFC079_MIGRATION_SRC
        .find("ALTER TABLE audit_log RENAME TO audit_log_legacy_0010")
        .expect("migration must swap the old audit table only after verification");
    let drop_position = RFC079_MIGRATION_SRC
        .find("DROP TABLE audit_log_legacy_0010")
        .expect("migration must remove the old audit table after the swap");
    assert!(
        verify_position < swap_position && swap_position < drop_position,
        "migration verification, swap, and drop order changed"
    );
    assert!(
        RFC079_MIGRATION_SRC.contains("CHECK (passed = 1)")
            && RFC079_MIGRATION_SRC.contains("DROP TABLE audit_migration_0010_guard"),
        "migration mismatch checks must fail closed and leave no guard table"
    );
    let central_statement_builder = RFC079_AUDIT_CORE_SRC
        .split("fn statement_with_suffix")
        .nth(1)
        .and_then(|source| source.split("fn success_event").next())
        .expect("central audit statement builder must exist");
    assert!(
        RFC079_AUDIT_CORE_SRC.contains(
            "(id, request_id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at)"
        ) && central_statement_builder.contains("D1Type::Text(self.request_id.as_str())")
            && central_statement_builder.matches("unwrap_or(D1Type::Null)").count() == 3,
        "the central statement builder must bind the validated request ID required by migration 0010"
    );

    assert!(
        RFC079_MIGRATION_RUNNER_SRC.contains("'migrations', 'apply'")
            && RFC079_MIGRATION_RUNNER_SRC.contains("'--local'")
            && RFC079_MIGRATION_RUNNER_SRC.contains("'--persist-to'")
            && !RFC079_MIGRATION_RUNNER_SRC.contains("'--remote'")
            && RFC079_MIGRATION_RUNNER_SRC.contains("legacyMetadataSelected: false")
            && RFC079_MIGRATION_RUNNER_SRC.contains("legacyMetadataPrinted: false")
            && RFC079_MIGRATION_RUNNER_SRC.contains("inheritNonAuthorityEnvironment")
            && RFC079_MIGRATION_RUNNER_SRC.contains("sentinel authority value was read"),
        "Package 2 rehearsal must apply the real ledger locally, isolate authority, and never select/print legacy metadata"
    );

    let alias_occurrence = RFC079_AUDIT_POLICY_SRC
        .find("target_kind = 'event_day' AND action = 'occurrence_cancelled'")
        .expect("audit policy must define the event-day alias");
    let alias_generated = RFC079_AUDIT_POLICY_SRC
        .find("target_kind = 'calendar_feed' AND action = 'calendar_token_generated'")
        .expect("audit policy must define the calendar-generation alias");
    let alias_revoked = RFC079_AUDIT_POLICY_SRC
        .find("target_kind = 'calendar_feed' AND action = 'calendar_token_revoked'")
        .expect("audit policy must define the calendar-revocation alias");
    let generic_rule = RFC079_AUDIT_POLICY_SRC
        .find("WHEN instr(action, '.') > 0 THEN action")
        .expect("audit policy must define the namespaced-action fallback");
    assert!(
        alias_occurrence < generic_rule
            && alias_generated < generic_rule
            && alias_revoked < generic_rule
            && RFC079_AUDIT_POLICY_SRC.contains("target_kind AS raw_target_kind")
            && RFC079_AUDIT_POLICY_SRC.contains("action AS raw_action")
            && RFC079_AUDIT_POLICY_SRC.contains("community.exported")
            && RFC079_AUDIT_POLICY_SRC.contains("community.export_authorized"),
        "raw-history compatibility policy must preserve raw values and apply explicit aliases before generic rules"
    );
    assert!(
        RFC079_BACKUP_RECOVERY_SRC.contains("pre-0010")
            && RFC079_BACKUP_RECOVERY_SRC.contains("potentially sensitive")
            && RFC079_BACKUP_RECOVERY_SRC.contains("Roll-forward recovery")
            && RFC079_DEPLOYMENT_SRC.contains("earliest deployable code boundary")
            && RFC079_DEPLOYMENT_SRC.contains("Package 8")
            && RFC079_DEPLOYMENT_SRC.contains("roll-forward only")
            && RFC079_OPERATIONS_SRC.contains("Mixed legacy and canonical audit actions")
            && RFC079_OPERATIONS_SRC.contains("Do not select `metadata_json`"),
        "Package 2 operator policy must cover sensitive backups, non-deployment, compatibility queries, and roll-forward recovery"
    );
}

#[test]
fn rfc079_package3_simple_required_batches_are_pinned() {
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("pub(crate) fn statement_after_one_change")
            && RFC079_AUDIT_CORE_SRC.contains("WHERE changes() = 1")
            && RFC079_AUDIT_CORE_SRC.contains("pub(crate) async fn execute_required")
            && RFC079_AUDIT_CORE_SRC
                .contains("let audit_statement = match audit.statement_after_one_change(db)")
            && RFC079_AUDIT_CORE_SRC.contains("db.batch(vec![mutation, audit_statement])")
            && RFC079_AUDIT_CORE_SRC
                .contains("log_class_a_failure(audit, AuditFailureCategory::Construction)")
            && RFC079_AUDIT_CORE_SRC.contains("(0, 0) => Ok(false)")
            && RFC079_AUDIT_CORE_SRC.contains("(1, 1)"),
        "Package 3 conditional required-audit primitive must keep mutation/audit adjacency and zero-or-one cardinality"
    );

    for (name, source, expected_batches) in [
        ("templates", RFC079_EVENT_TEMPLATE_DB_SRC, 2usize),
        ("invites", INVITE_DB_SRC, 2),
        ("membership", MEMBERSHIP_DB_SRC, 5),
        ("note moderation", RFC079_EVENT_NOTE_DB_SRC, 1),
        ("single attendance", RFC079_ATTENDANCE_DB_SRC, 1),
    ] {
        assert_eq!(
            source
                .matches("audit::execute_required(db, mutation, &record)")
                .count(),
            expected_batches,
            "Package 3 {name} helper count changed"
        );
        assert!(
            source.contains("role = 'admin'") && source.contains("MEMBERSHIP_ACTIVE"),
            "Package 3 {name} mutation must repeat active-admin authorization in SQL"
        );
    }

    for (name, source) in [
        ("community", COMMUNITY_DB_SRC),
        ("display name", ME_HANDLER_SRC),
        ("operator recovery", OPERATOR_HANDLER_SRC),
        ("templates", RFC079_TEMPLATES_HANDLER_SRC),
        ("note moderation", RFC079_NOTE_HANDLER_SRC),
        ("single attendance", EVENT_HANDLER_SRC),
        ("invites", MEMBERS_HANDLER_SRC),
        ("member removal", MEMBER_REMOVE_HANDLER_SRC),
        ("role transfer", ROLE_TRANSFER_HANDLER_SRC),
    ] {
        assert!(
            !source.contains("INSERT INTO audit_log")
                && !source.contains("let _ = audit::write_legacy")
                && !source.contains("let _ = crate::audit::write_legacy"),
            "Package 3 {name} surface must use only the central typed audit builder"
        );
    }

    assert!(
        COMMUNITY_DB_SRC.matches("audit::required_record").count() == 2
            && COMMUNITY_DB_SRC.contains("audit::execute_required_batch")
            && ME_HANDLER_SRC.contains("AuditMetadata::DisplayNameChanged")
            && ME_HANDLER_SRC.contains("AND display_name != ?1")
            && RELINK_DB_SRC.contains("AuditMetadata::OperatorRecovery")
            && OPERATOR_HANDLER_SRC.contains("relink_db::issue_required"),
        "former direct-insert surfaces must remain centralized after the Package 5 operator boundary move"
    );

    for (name, source, current_state) in [
        (
            "template deletion",
            RFC079_EVENT_TEMPLATE_DB_SRC,
            "is_active = 1",
        ),
        (
            "invite revocation",
            INVITE_DB_SRC,
            "used_at IS NULL AND revoked_at IS NULL",
        ),
        (
            "note moderation",
            RFC079_EVENT_NOTE_DB_SRC,
            "hidden_by_admin_at IS NULL",
        ),
        (
            "attendance",
            RFC079_ATTENDANCE_DB_SRC,
            "attendances.status IS NOT 'attended'",
        ),
        ("role transfer", MEMBERSHIP_DB_SRC, "role = 'member'"),
    ] {
        assert!(
            source.contains(current_state),
            "Package 3 {name} mutation must repeat expected current state"
        );
    }

    assert!(
        RFC079_ATOMICITY_RUNNER_SRC.contains("'d1', 'migrations', 'apply'")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("'--local'")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("'--persist-to'")
            && !RFC079_ATOMICITY_RUNNER_SRC.contains("'--remote'")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("inheritNonAuthorityEnvironment")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("sentinel authority value was read")
            && RFC079_ATOMICITY_WORKER_SRC.contains("WHERE changes() = 1")
            && RFC079_ATOMICITY_WORKER_SRC.contains("db.batch(statements)")
            && !RFC079_ATOMICITY_WORKER_SRC.contains("console.")
            && RFC079_ATOMICITY_FIXTURE_SRC.contains("CREATE TRIGGER reject_proof_audit")
            && RFC079_ATOMICITY_FIXTURE_SRC
                .contains("RAISE(ABORT, 'synthetic required audit rejection')"),
        "Package 3 real-D1 proof must remain local-only, authority-isolated, privacy-bounded, and cover audit-trigger rollback"
    );
}

#[test]
fn rfc079_package4_event_calendar_and_attendance_batches_are_pinned() {
    assert!(
        RFC079_EVENT_WRITE_DB_SRC.contains("RECURRENCE_MATERIALIZATION_INSERT_CAP + 3")
            && RFC079_EVENT_WRITE_DB_SRC.contains("event create statement budget exceeded")
            && RFC079_EVENT_WRITE_DB_SRC.contains("execute_required_tail")
            && RFC079_EVENT_WRITE_DB_SRC.contains("EventOccurrenceCancelled")
            && RFC079_EVENT_WRITE_DB_SRC.contains("EventEditScope::SingleDaySchedule")
            && RFC079_EVENT_WRITE_DB_SRC.contains("AuditAction::EventCancelled"),
        "Package 4 event creation and mutation batches must remain bounded and typed"
    );
    for required in [
        "actor.role='admin'",
        "MEMBERSHIP_ACTIVE",
        "e.status='scheduled'",
        "occurrence_status='scheduled'",
        "WHERE changes()=1",
    ] {
        assert!(
            RFC079_EVENT_WRITE_DB_SRC.contains(required),
            "Package 4 event mutation boundary is missing {required:?}"
        );
    }
    for required in [
        "AND (SELECT COUNT(*) FROM event_days WHERE event_id=?5)=1",
        "AND EXISTS (SELECT 1 FROM event_days d",
        "d.occurrence_status='scheduled'",
        "d.day_date IS ?8",
        "d.starts_at_utc IS ?9 AND d.ends_at_utc IS ?10",
        "d.starts_at_utc>?4",
    ] {
        assert!(
            RFC079_EVENT_WRITE_DB_SRC.contains(required),
            "single-day edit tail must prove its exact eligible day post-state: missing {required:?}"
        );
    }
    assert!(
        RFC079_ATTENDANCE_DB_SRC.contains("ADMIN_OVERRIDE_CELL_CAP: usize = 10_000")
            && RFC079_ATTENDANCE_DB_SRC.contains("FROM json_each(?1)")
            && RFC079_ATTENDANCE_DB_SRC.contains("SELECT COUNT(*) FROM eligible")
            && RFC079_ATTENDANCE_DB_SRC.contains("attendances.status IS NOT excluded.status")
            && RFC079_ATTENDANCE_DB_SRC.contains("execute_required_attendance_override"),
        "Package 4 attendance override must remain one bounded, all-or-nothing set-based mutation"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("json_object('changed_count', changes())")
            && RFC079_AUDIT_CORE_SRC.contains("WHERE changes() BETWEEN 1 AND")
            && RFC079_AUDIT_CORE_SRC.contains("attendance audit cardinality mismatch"),
        "attendance audit metadata must use the database mutation cardinality"
    );
    assert!(
        CALENDAR_DB_SRC.contains("pub async fn rotate_required")
            && CALENDAR_DB_SRC.contains("pub async fn revoke_required")
            && CALENDAR_DB_SRC.contains("execute_required_tail")
            && CALENDAR_DB_SRC.contains("execute_required_bounded")
            && CALENDAR_DB_SRC.contains("MEMBERSHIP_ACTIVE")
            && !CALENDAR_HANDLER_SRC.contains("write_calendar_token_audit"),
        "Package 4 calendar-token rotation/revocation must remain typed and atomic"
    );
    assert!(
        RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/event/audit-failure")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/edit/eligibility-loss")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/occurrence/success")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/occurrence/replay")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/occurrence/audit-failure")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/attendance/replay")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/calendar/audit-failure")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("predecessorRestored: true")
            && RFC079_ATOMICITY_WORKER_SRC.contains("json_object('changed_count', changes())")
            && RFC079_ATOMICITY_WORKER_SRC.contains("proof_event_parts")
            && RFC079_ATOMICITY_FIXTURE_SRC.contains("proof_edit_days")
            && RFC079_ATOMICITY_FIXTURE_SRC.contains("proof_occurrence_exceptions")
            && RFC079_ATOMICITY_FIXTURE_SRC.contains("proof_calendar_tokens"),
        "Package 4 real-D1 proof must cover edit eligibility loss, occurrence success/no-op/rollback, multi-write rollback, database-derived attendance count/no-op, and calendar predecessor restoration"
    );
}

#[test]
fn rfc079_package5_one_winner_batches_and_correlation_are_pinned() {
    for required in [
        "pub(crate) async fn execute_asserted_required",
        "format!(\"ast_{}\", &random_token()[..22])",
        "INSERT INTO audit_change_assertions (operation_id, changed_count)",
        "VALUES (?1, changes())",
        "UPDATE audit_change_assertions SET changed_count=changes()",
        "let audit_statement = match audit.statement(db)",
        "statements.push(audit_statement)",
        "DELETE FROM audit_change_assertions WHERE operation_id=?1",
        "assertion_changes,",
        "cleanup_changes,",
        ") != (1, 1, 1, 1)",
    ] {
        assert!(
            RFC079_AUDIT_CORE_SRC.contains(required),
            "Package 5 central assertion executor is missing {required:?}"
        );
    }
    assert!(
        INVITE_DB_SRC.contains("pub async fn redeem_required")
            && INVITE_DB_SRC.contains("audit::execute_asserted_required")
            && INVITE_DB_SRC.contains("INSERT INTO users")
            && INVITE_DB_SRC.contains("INSERT INTO community_memberships")
            && INVITE_DB_SRC.contains("UPDATE invite_codes SET used_by_membership_id=?1")
            && INVITE_DB_SRC.contains("INSERT INTO sessions")
            && INVITE_DB_SRC.contains("AuditAction::InviteCodeRedeemed"),
        "Package 5 join claim, candidate identities/session, linkage, and audit must share one asserted batch"
    );
    assert!(
        RELINK_DB_SRC.contains("pub async fn redeem_required")
            && RELINK_DB_SRC.contains("audit::execute_asserted_required")
            && RELINK_DB_SRC.contains("INSERT INTO sessions")
            && RELINK_DB_SRC.contains("UPDATE sessions SET revoked_at=?1")
            && RELINK_DB_SRC.contains("AuditAction::MembershipRelinkRedeemed")
            && RELINK_DB_SRC.contains("relink_code_id: target.id.clone()"),
        "Package 5 relink claim, replacement session, predecessor revocation, and correlated audit must share one asserted batch"
    );
    assert!(
        HELP_SIGNIN_HANDLER_SRC.contains("relink_db::issue_required")
            && OPERATOR_HANDLER_SRC.contains("relink_db::issue_required")
            && RELINK_DB_SRC.contains("AuditAction::MembershipRelinkCodeCreated")
            && RELINK_DB_SRC.contains("AuditAction::OperatorRecoveryAdminRelinkCreated")
            && RELINK_DB_SRC.contains("AuditMetadata::RelinkCorrelation")
            && RELINK_DB_SRC.contains("AuditMetadata::OperatorRecovery")
            && RELINK_DB_SRC.contains("audit::execute_required_tail"),
        "help-signin and operator relink-code issuance must share guarded typed correlation"
    );
    let help_signin_post = HELP_SIGNIN_HANDLER_SRC
        .split("pub async fn post_help_signin")
        .nth(1)
        .expect("help-signin POST handler must exist");
    let guarded_issuance = help_signin_post
        .find("if !relink_db::issue_required(")
        .expect("help-signin must inspect guarded issuance success");
    let rejected_issuance = help_signin_post[guarded_issuance..]
        .find("return render::not_found();")
        .map(|offset| guarded_issuance + offset)
        .expect("help-signin must reject a zero-row guarded issuance");
    let code_response = help_signin_post
        .find("data-copy-code-value")
        .expect("help-signin success must render the generated code");
    assert!(
        help_signin_post.contains(".await?\n    {\n        return render::not_found();\n    }")
            && guarded_issuance < rejected_issuance
            && rejected_issuance < code_response,
        "help-signin must reject a zero-row guarded issuance before rendering the generated code"
    );
    assert!(
        RFC079_ASSERTION_RUNNER_SRC.contains("/flow/join/concurrent")
            && RFC079_ASSERTION_RUNNER_SRC.contains("/flow/relink/concurrent")
            && RFC079_ASSERTION_RUNNER_SRC.contains("/flow/join/audit-failure")
            && RFC079_ASSERTION_RUNNER_SRC.contains("/flow/relink/audit-failure")
            && RFC079_ASSERTION_RUNNER_SRC
                .contains("users: 1, memberships: 1, links: 1, sessions: 1, audits: 1, guards: 0")
            && RFC079_ASSERTION_RUNNER_SRC
                .contains("users: 0, memberships: 0, links: 0, sessions: 1, audits: 1, guards: 0")
            && RFC079_ASSERTION_WORKER_SRC.contains("async function runFlow")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("proof_flow_users")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("proof_flow_memberships")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("proof_flow_links")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("proof_flow_sessions")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("proof_flow_audits"),
        "Package 5 local D1 proof must show one winner/one audit, no losing candidate identity/session residue, and full audit-failure rollback"
    );
}

#[test]
fn rfc079_package6_disclosure_and_logout_boundaries_are_pinned() {
    let export_audit = EXPORT_HANDLER_SRC
        .find("audit::write_pre_disclosure(")
        .expect("community JSON export must persist pre-disclosure audit evidence");
    let export_payload = EXPORT_HANDLER_SRC
        .find("let payload = build_export(")
        .expect("community JSON export payload construction must remain explicit");
    let export_response = EXPORT_HANDLER_SRC
        .find("let mut resp = Response::ok(json)?")
        .expect("community JSON export response must remain explicit");
    assert!(
        export_audit < export_payload
            && export_payload < export_response
            && EXPORT_HANDLER_SRC.contains("AuditAction::CommunityExportAuthorized")
            && EXPORT_HANDLER_SRC.contains("return render::service_unavailable();"),
        "community JSON export must return disclosure-free 503 unless typed audit evidence is durable"
    );

    let matrix_post = COMMUNITIES_HANDLER_SRC
        .split("pub async fn post_matrix_export_audit")
        .nth(1)
        .expect("matrix export acknowledgement handler must exist");
    let matrix_audit = matrix_post
        .find("crate::audit::write_pre_disclosure(")
        .expect("matrix export acknowledgement must persist pre-disclosure evidence");
    let matrix_response = matrix_post
        .find("Response::from_json")
        .expect("matrix export acknowledgement response must remain explicit");
    assert!(
        matrix_audit < matrix_response
            && matrix_post.contains("AuditAction::CalendarMatrixCsvExportRequested")
            && matrix_post.contains("AuditMetadata::MatrixExportRequested")
            && matrix_post
                .contains("return json_error(503, i18n::t(locale, i18n::GENERAL_ERROR));"),
        "matrix export acknowledgement must return privacy-safe JSON 503 when typed audit evidence fails"
    );
    assert!(
        RENDER_SRC.contains("pub fn service_unavailable()")
            && RENDER_SRC.contains("with_status(503)"),
        "Class B HTML failure must use a generic 503 renderer"
    );

    let revoke = AUTH_HANDLER_SRC
        .find("session_db::revoke(&db, &auth.session_id).await?;")
        .expect("logout must require awaited server-side revocation");
    let secondary = AUTH_HANDLER_SRC
        .find("crate::audit::write_logout_secondary(&db, rid).await;")
        .expect("logout must await its bounded secondary audit attempt");
    let clear_cookie = AUTH_HANDLER_SRC
        .find("crate::session::clear_session_cookie")
        .expect("logout must clear the client credential");
    assert!(
        revoke < secondary && secondary < clear_cookie,
        "logout must revoke first, await secondary audit, then clear the cookie"
    );
    let logout_secondary = RFC079_AUDIT_CORE_SRC
        .split("pub(crate) async fn write_logout_secondary")
        .nth(1)
        .and_then(|source| source.split("pub(crate) fn result_changes").next())
        .expect("logout-only secondary writer must remain bounded");
    assert!(
        logout_secondary.contains("let action = AuditAction::SessionLogout;")
            && logout_secondary.contains("AuditFailureEvent::SecondaryWrite")
            && logout_secondary.contains("AuditMetadata::None")
            && !logout_secondary.contains("session_id")
            && !logout_secondary.contains("target_id"),
        "logout secondary audit must accept no credential/subject identifier and must emit its bounded incident"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("audit.pre_disclosure_failed")
            && RFC079_AUDIT_CORE_SRC.contains("audit.secondary_write_failed")
            && RFC079_AUDIT_CORE_SRC.contains("failure_category={}")
            && RFC079_AUDIT_CORE_SRC.contains("route_class={}"),
        "Package 6 audit failures must emit bounded structured operational events"
    );

    let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../workers/ssr/src")
        .canonicalize()
        .expect("workers/ssr/src must exist for the Package 6 source gate");
    let mut sources = Vec::new();
    collect_rust_sources(&source_root, &source_root, &mut sources);
    let secondary_owners = sources
        .iter()
        .filter(|(_, source)| source.contains("write_logout_secondary("))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        secondary_owners,
        vec!["audit.rs", "handlers/auth.rs"],
        "logout must remain the only caller and owner of the Class C secondary-audit exception"
    );
    let pre_disclosure_owners = sources
        .iter()
        .filter(|(_, source)| source.contains("write_pre_disclosure("))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        pre_disclosure_owners,
        vec!["audit.rs", "handlers/communities.rs", "handlers/export.rs"],
        "only the two reviewed Class B surfaces may use the pre-disclosure writer"
    );
    assert!(
        PACKAGE_JSON_SRC.contains("\"test:audit-boundaries\"")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("/class-b/community/audit-failure")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("/class-b/matrix/audit-failure")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("/class-c/logout/audit-failure")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("status === 503")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("status === 303")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("Max-Age=0")
            && RFC079_BOUNDARY_WORKER_SRC.contains("community.export_authorized")
            && RFC079_BOUNDARY_WORKER_SRC.contains("calendar_matrix_csv.export_requested")
            && RFC079_BOUNDARY_WORKER_SRC.contains("session.logout")
            && RFC079_BOUNDARY_FIXTURE_SRC.contains("proof_boundary_sessions")
            && RFC079_BOUNDARY_FIXTURE_SRC.contains("proof_boundary_audits")
            && !RFC079_BOUNDARY_WORKER_SRC.contains("console."),
        "Package 6 local D1 proof must cover disclosure-free Class B failure and safety-first Class C failure without Worker logging"
    );
}

#[test]
fn rfc079_package7_removal_and_documentation_boundary_are_pinned() {
    let migration_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let mut migration_filenames = std::fs::read_dir(&migration_root)
        .expect("migration directory must be readable")
        .map(|entry| entry.expect("migration entry must be readable").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "sql"))
        .map(|path| {
            path.file_name()
                .expect("migration must have a filename")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    migration_filenames.sort();
    let expected_migration_filenames = [
        "0001_initial.sql",
        "0002_form_tokens_nullable_user.sql",
        "0003_invite_grants_role.sql",
        "0004_calendar_tokens.sql",
        "0005_event_templates.sql",
        "0006_event_recurrence.sql",
        "0007_codlet_tables.sql",
        "0008_membership_relink_codes.sql",
        "0009_recurrence_v2.sql",
        "0010_audit_integrity.sql",
        "0011_membership_ui_language.sql",
        "0012_session_provenance.sql",
        "0013_identity_namespaces.sql",
        "0014_auth_transactions.sql",
        "0015_session_authenticated_at.sql",
        "0016_auth_transaction_initiating_user.sql",
        "0017_account_recovery_credentials.sql",
        "0018_membership_suspension.sql",
    ];
    assert_eq!(
        migration_filenames, expected_migration_filenames,
        "architecture migration inventory gate must be updated with the actual SQL ledger"
    );
    for filename in expected_migration_filenames {
        assert!(
            RFC079_ARCHITECTURE_SRC.contains(filename),
            "architecture migration inventory is missing actual ledger entry {filename}"
        );
    }
    assert!(
        !RFC079_ARCHITECTURE_SRC.contains("0007_event_day_attendance_grain.sql"),
        "architecture must not name the nonexistent migration 0007 entry"
    );

    for forbidden in [
        "pub(crate) enum LegacyAuditAction",
        "pub(crate) enum LegacyAuditMetadata",
        "pub(crate) async fn write_legacy",
        "metadata: Option<serde_json::Value>",
        "action: &str",
    ] {
        assert!(
            !RFC079_AUDIT_CORE_SRC.contains(forbidden),
            "Package 7 must remove forbidden audit surface {forbidden:?}"
        );
    }
    assert!(
        LIB_SRC.contains("event=worker.request_failed request_id={}")
            && LIB_SRC.contains("failure_category=unhandled route_class=request")
            && !LIB_SRC.contains("unhandled error: {:?}"),
        "unhandled Worker errors must use bounded categories rather than raw Debug output"
    );
    for (name, source, required) in [
        ("RFC-014", RFC079_RFC014_SRC, "Current RFC-079 boundary"),
        (
            "RFC-052",
            RFC079_RFC052_SRC,
            "RFC-079 reconciliation (2026-07-16)",
        ),
        ("RFC-071", RFC079_RFC071_SRC, "sole Class C exception"),
        (
            "RFC-050",
            RFC079_RFC050_SRC,
            "earliest deployable **code** boundary",
        ),
        (
            "threat model",
            RFC079_THREAT_MODEL_SRC,
            "closed 26-action model",
        ),
        (
            "architecture",
            RFC079_ARCHITECTURE_SRC,
            "Audit integrity boundary",
        ),
        (
            "operations",
            RFC079_OPERATIONS_SRC,
            "audit.secondary_write_failed",
        ),
        (
            "backup/recovery",
            RFC079_BACKUP_RECOVERY_SRC,
            "removed compatibility adapter",
        ),
        (
            "release checklist",
            RFC079_RELEASE_CHECKLIST_SRC,
            "RFC-079 audit integrity and redaction",
        ),
        (
            "audit/operator query policy",
            RFC079_AUDIT_POLICY_SRC,
            "Raw-history compatibility query",
        ),
    ] {
        assert!(
            source.contains(required),
            "Package 7 {name} reconciliation is missing {required:?}"
        );
    }
    for source in [
        RFC079_RFC014_SRC,
        RFC079_RFC071_SRC,
        RFC079_THREAT_MODEL_SRC,
        RFC079_ARCHITECTURE_SRC,
        RFC079_AUDIT_POLICY_SRC,
        RFC079_BACKUP_RECOVERY_SRC,
        RFC079_RELEASE_CHECKLIST_SRC,
    ] {
        assert!(
            source.contains("Class A") || source.contains("Class A/B/C"),
            "Package 7 durable audit documents must preserve the Class A/B/C model"
        );
    }
    assert!(
        RFC079_AUDIT_POLICY_SRC.contains("not deployment approval")
            && RFC079_DEPLOYMENT_SRC.contains("Package 8")
            && RFC079_RELEASE_CHECKLIST_SRC.contains("exact-candidate hosted")
            && RFC079_RFC050_SRC.contains("local fixture success"),
        "Package 7 must distinguish earliest deployable code from release/hosted authorization"
    );
    assert!(
        RFC079_BACKUP_RECOVERY_SRC.contains("D1 Time Travel")
            && RFC079_BACKUP_RECOVERY_SRC.contains("7 days on Workers Free")
            && RFC079_BACKUP_RECOVERY_SRC.contains("30 days on Workers Paid")
            && !RFC079_BACKUP_RECOVERY_SRC.contains("30 days on all plans"),
        "backup/recovery must pin plan-aware D1 Time Travel retention without a plan-independent guarantee"
    );
}

#[test]
fn rfc079_package0a_assertion_fixture_is_bounded_and_outside_the_ledger() {
    assert!(
        RFC079_ASSERTION_FIXTURE_SRC.contains("CREATE TABLE audit_change_assertions")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("length(operation_id) = 26")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("changed_count = 1"),
        "Package 0A fixture must pin the reviewed operation-ID and one-row CHECK constraints"
    );
    assert!(
        !RFC079_ASSERTION_FIXTURE_SRC.contains("d1_migrations")
            && !RFC079_ASSERTION_FIXTURE_SRC.contains("audit_log"),
        "Package 0A fixture must stay outside the D1 ledger and production audit schema"
    );
    assert!(
        RFC079_ASSERTION_WORKER_SRC.contains("VALUES (?1, changes())")
            && RFC079_ASSERTION_WORKER_SRC.contains("db.batch(statements)")
            && !RFC079_ASSERTION_WORKER_SRC.contains("console."),
        "Package 0A Worker must exercise the exact D1 batch primitive without console output"
    );
    assert!(
        RFC079_ASSERTION_RUNNER_SRC.contains("cloudflareAuthorityKey")
            && RFC079_ASSERTION_RUNNER_SRC.contains("CLOUDFLARE_")
            && RFC079_ASSERTION_RUNNER_SRC.contains("Object.keys(source)")
            && !RFC079_ASSERTION_RUNNER_SRC.contains("Object.entries(process.env)")
            && RFC079_ASSERTION_RUNNER_SRC.contains("sentinel authority value was read")
            && !RFC079_ASSERTION_RUNNER_SRC.contains("genericCallSites")
            && !RFC079_ASSERTION_RUNNER_SRC.contains("directSqlOutsideAuditModule")
            && !RFC079_ASSERTION_RUNNER_SRC.contains("classifiedActions"),
        "Package 0A runner must scrub inherited Cloudflare authority and must not report declarative inventory as D1 evidence"
    );
    let proof_audit_schema = RFC079_ASSERTION_FIXTURE_SRC
        .split("CREATE TABLE proof_audits")
        .nth(1)
        .expect("proof fixture must define its synthetic audit table");
    assert!(
        !proof_audit_schema.contains("operation_id"),
        "assertion operation IDs must not enter even the synthetic audit row"
    );
}

#[test]
fn rfc079_removal_scanner_detects_forbidden_surfaces_and_action_substitution() {
    let synthetic_sources = vec![
        (
            "known.rs".to_owned(),
            "fn known() { let action = AuditAction::MembershipRemoved; }".to_owned(),
        ),
        (
            "compatibility.rs".to_owned(),
            "fn old() { write_legacy(db, LegacyAuditAction::MembershipRemoved); }".to_owned(),
        ),
        (
            "new_direct.rs".to_owned(),
            "const SQL: &str = \"INSERT INTO audit_log VALUES (...)\";".to_owned(),
        ),
        (
            "new_privacy_ref.rs".to_owned(),
            "fn leaked(operation_id: &str) { let _ = audit_change_assertions; }".to_owned(),
        ),
        (
            "ignored.rs".to_owned(),
            "fn ignored() { let _ = audit::execute_required(db, mutation, audit); }".to_owned(),
        ),
        (
            "background.rs".to_owned(),
            "fn background() { spawn_local(async move { audit_required().await; }); }".to_owned(),
        ),
    ];
    let scan = scan_audit_sources(&synthetic_sources);
    assert_eq!(scan.direct_inserts["new_direct.rs"], 1);
    assert_eq!(scan.assertion_table_refs, ["new_privacy_ref.rs"]);
    assert_eq!(scan.operation_id_refs, ["new_privacy_ref.rs"]);
    assert_eq!(scan.compatibility_refs, ["compatibility.rs"]);
    assert_eq!(scan.ignored_audit_results, ["ignored.rs"]);
    assert_eq!(scan.background_audit_refs, ["background.rs"]);
    assert_eq!(
        audit_action_owners(&synthetic_sources, "MembershipRemoved"),
        ["known.rs"]
    );
    assert!(
        audit_action_owners(&synthetic_sources, "MembershipPromotedToAdmin").is_empty(),
        "a substituted expected action owner must fail the exact inventory gate"
    );
}

#[test]
fn invite_code_generator_does_not_use_unwrap_or_default_on_getrandom() {
    // If this fails, the generator has regressed to fail-open: randomness
    // failure would silently produce a deterministic invite code.
    //
    // getrandom 0.4 renamed the entry point from `getrandom::getrandom` to
    // `getrandom::fill`. The source must use `?` or `.expect()` after the
    // call, not `.unwrap_or_default()` or `.ok()`.
    // Invite generation must propagate getrandom errors instead of silently
    // falling back to deterministic bytes.
    let lines: Vec<&str> = MEMBERS_HANDLER_SRC
        .lines()
        .filter(|l| l.contains("getrandom::fill") || l.contains("getrandom::getrandom"))
        .collect();
    for l in &lines {
        assert!(
            !l.contains("unwrap_or_default") && !l.contains(".ok()"),
            "getrandom call uses fail-open error handling: {l:?}\n\
             Must use `?` or `.expect()` — silence on RNG failure produces \
             a deterministic invite code."
        );
    }
}

#[test]
fn invite_code_generator_uses_rejection_sampling() {
    // The unbiased ceiling must appear in the source to confirm rejection
    // sampling is in use. 248 = 256 - (256 % 31) is the exact value.
    assert!(
        MEMBERS_HANDLER_SRC.contains("248")
            || MEMBERS_HANDLER_SRC.contains("unbiased_ceiling")
            || MEMBERS_HANDLER_SRC.contains("256 - (256 % alpha_len)"),
        "generate_invite_code no longer references the rejection-sampling ceiling (248 or \
         unbiased_ceiling or the expression). Verify the modulo-bias fix is still in place."
    );
    // The old biased pattern must not be present.
    assert!(
        !MEMBERS_HANDLER_SRC.contains("unwrap_or_default();\n    bytes.iter()"),
        "generate_invite_code appears to have reverted to the biased modulo pattern."
    );
}

#[test]
fn join_profile_backfills_invite_membership_after_membership_exists() {
    let redeem = INVITE_DB_SRC
        .split("pub async fn redeem_required")
        .nth(1)
        .expect("join profile required redemption helper must exist");
    let mark_used = redeem
        .find("UPDATE invite_codes SET used_at=?1")
        .expect("join profile must atomically claim the invite");
    let insert_user = redeem
        .find("INSERT INTO users")
        .expect("join profile must insert user in the asserted batch");
    let insert_membership = redeem
        .find("INSERT INTO community_memberships")
        .expect("join profile must insert membership in the asserted batch");
    let assign_used_membership = redeem
        .find("UPDATE invite_codes SET used_by_membership_id=?1")
        .expect("join profile must link the claimed invite to its membership");

    assert!(
        mark_used < insert_user && mark_used < insert_membership,
        "invite must be claimed before user/session side effects so races create one winner"
    );
    assert!(
        insert_membership < assign_used_membership,
        "used_by_membership_id references community_memberships(id); backfill it only after \
         insert_membership succeeds"
    );
}

#[test]
fn invite_mark_used_does_not_write_membership_fk() {
    let redeem = INVITE_DB_SRC
        .split("pub async fn redeem_required")
        .nth(1)
        .expect("invite::redeem_required must exist");
    let claim_end = redeem
        .find("let user =")
        .expect("asserted claim must precede candidate creation");
    let mark_body = &redeem[..claim_end];
    let assign_body = &redeem[claim_end..];

    assert!(
        mark_body.contains("SET used_at=?1"),
        "mark_used should perform the atomic one-winner claim"
    );
    assert!(
        !mark_body.contains("used_by_membership_id"),
        "mark_used must not write used_by_membership_id before the membership FK target exists"
    );
    assert!(
        assign_body.contains("SET used_by_membership_id=?1"),
        "assign_used_membership should perform the post-membership FK backfill"
    );
}

// ── RFC-075: hardcoded hex may only shrink; inline styling must stay zero ──
//
// Hardcoded hex is a ratchet (Resolved Decision 2): not a threshold, a count
// that may only shrink, re-measured against the whole `workers/ssr/src` tree
// at test time (a filesystem walk, not `include_str!`, so a new file or an
// untouched existing one is covered automatically) and pinned here. A count
// that increases means a new hardcoded colour was added instead of reaching
// for a `--cz-*` token; lower this pin whenever a slice migrates more.
//
// Inline styling is no longer a ratchet (Handoff 038, RFC-075's terminal
// slice): `style-src` no longer carries `'unsafe-inline'`, so any inline
// `style=` attribute reintroduced now is a CSP regression, not an
// incomplete migration — asserted at exactly zero, not "never increases".

const HARDCODED_HEX_RATCHET: usize = 25;

fn workers_ssr_src_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../workers/ssr/src")
}

fn walk_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read directory {}: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("failed to read directory entry: {e}"))
            .path();
        if path.is_dir() {
            walk_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn count_over_tree(count_in_file: impl Fn(&str) -> usize) -> usize {
    let mut files = Vec::new();
    walk_rs_files(&workers_ssr_src_dir(), &mut files);
    assert!(
        files.len() > 50,
        "expected many .rs files under workers/ssr/src, found only {} — \
         directory walk is probably broken, not the codebase actually shrinking",
        files.len()
    );
    files
        .iter()
        .map(|path| {
            let content = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            count_in_file(&content)
        })
        .sum()
}

/// Handoff 038 §7.3: comments stripped first — this counter previously
/// treated `lib.rs`'s own CSP comment (which mentioned `style=` while
/// explaining the directive) as an inline style, which is why this sat at 1
/// instead of the true 0. `strip_line_comments` was built for the flash
/// gate (`rfc072_flash_query_values_are_lowercase_snake_case_codes_not_prose`)
/// and is reused here rather than wording comments to dodge the literal —
/// making a comment's phrasing load-bearing was rejected in Handoff 034 §3.1
/// and is not repeated here.
fn count_inline_styles(content: &str) -> usize {
    strip_line_comments(content).matches("style=").count()
}

/// Count `#RRGGBB` / `#RGB` hex-colour literals: a `#` followed by exactly 3
/// or exactly 6 hex digits, with no hex digit immediately before or after
/// (so this doesn't match a 6-digit prefix of a longer run, or double-count
/// a 3-digit prefix of a 6-digit colour).
fn count_hex_literals(content: &str) -> usize {
    let bytes = content.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
                end += 1;
            }
            let len = end - start;
            if (len == 3 || len == 6) && end == start + len {
                count += 1;
                i = end;
                continue;
            }
            if len > 6 {
                i = end;
                continue;
            }
        }
        i += 1;
    }
    count
}

#[test]
fn inline_style_count_is_zero() {
    // Handoff 038: RFC-075's terminal criterion. `style-src` no longer
    // carries `'unsafe-inline'` (see `style_src_has_no_unsafe_inline`
    // below), so a reintroduced inline style is a CSP regression, not an
    // incomplete migration — asserted at exactly zero, not ratcheted.
    let count = count_over_tree(count_inline_styles);
    assert_eq!(
        count, 0,
        "inline `style=` count is {count}, expected 0. RFC-075 removed every inline style \
         across seven slices and the terminal slice dropped 'unsafe-inline' from style-src — a \
         reintroduced inline style now fails silently in the browser (CSP drops it) as well as \
         here. Use a cz-* class from app.css instead."
    );
}

#[test]
fn style_src_has_no_unsafe_inline() {
    // Handoff 038 §7.6: RFC-075's terminal criterion. Seven slices removed
    // every inline style from the SSR templates specifically so this
    // directive could be dropped; reintroducing it would silently undo that
    // work — a rendered inline style would simply start working again
    // instead of being dropped by the browser and caught by every smoke's
    // `no-csp-violations` check (Handoff 038 §7.1).
    //
    // Comments stripped first — this gate's own doc comment mentions
    // 'unsafe-inline' while explaining what it checks (caught when this
    // gate was first written: it failed against its own file, the same
    // self-referential trap `count_inline_styles` and the flash gate hit).
    let code = strip_line_comments(LIB_SRC);
    assert!(
        !code.contains("'unsafe-inline'"),
        "lib.rs's Content-Security-Policy header contains 'unsafe-inline'. RFC-075 removed every \
         inline style across seven slices specifically so this directive could be dropped — \
         reintroducing it undoes that work. If a new inline style is genuinely required, that is \
         a design decision for the RFC owner, not something to fix by loosening this header."
    );
    assert!(
        code.contains("style-src 'self';"),
        "expected the literal `style-src 'self';` directive in lib.rs's Content-Security-Policy \
         header — has the header's formatting changed? Update this gate's expected substring if \
         so, but do not weaken what it checks."
    );
}

#[test]
fn hardcoded_hex_color_count_never_increases() {
    let count = count_over_tree(count_hex_literals);
    assert!(
        count <= HARDCODED_HEX_RATCHET,
        "hardcoded hex-colour literal count is {count}, above the RFC-075 ratchet of \
         {HARDCODED_HEX_RATCHET}. This count may only go down. If you added a new hex value in \
         Rust, reach for a --cz-* token in app.css instead; if you migrated more of the tree and \
         lowered the real count, lower HARDCODED_HEX_RATCHET to match — never raise it."
    );
}

/// Handoff 048 §8 (RFC-081 §2/§2.1a): the whole session-provenance design
/// rests on there being exactly two session-minting sites (§3's
/// enumeration). Default-fail, same shape as `ADMIN_CLASS_LEAK_EXCEPTIONS`:
/// every `INSERT INTO sessions` anywhere under `workers/ssr/src` must be
/// named here with a written reason, or the walk fails — an unguarded
/// third minting site added later would otherwise silently reintroduce a
/// NULL-provenance session with nothing to catch it before authorization's
/// fail-closed refusal (§7.3) does, at request time, in production.
struct SessionMintingSite {
    file: &'static str,
    reason: &'static str,
}

const KNOWN_SESSION_MINTING_SITES: &[SessionMintingSite] = &[
    SessionMintingSite {
        file: "db/invite.rs",
        reason: "invite redemption — first-class session, provenance InviteRedemption, no scope",
    },
    SessionMintingSite {
        file: "db/relink.rs",
        reason: "relink redemption — community-bound session, provenance Relink, scope from the redeemed code's community_id",
    },
    SessionMintingSite {
        file: "db/auth_transaction.rs",
        reason: "external identity sign-in/join/re-authentication (Handoff 054, Handoff 056 §3.2) — provenance ExternalIdentity, three INSERT INTO sessions occurrences in this one file (issue_sign_in_required, issue_join_required, reauthenticate_required)",
    },
    SessionMintingSite {
        file: "db/identity.rs",
        reason: "external identity linking (Handoff 056 §5.1) — provenance ExternalIdentity, one INSERT INTO sessions occurrence (link_required), atomic with the user_identities insert and the revoke-others rotation",
    },
    SessionMintingSite {
        file: "db/recovery.rs",
        reason: "recovery-credential consumption (Handoff 057 §5.2) — provenance AccountRecovery, unscoped, account-tier and fresh by construction (pinned by authz::account_recovery_provenance_is_account_tier_and_eligible_when_fresh)",
    },
];

/// The SQL string literal for one `db.prepare("...")` call, from `start`
/// (the byte offset of `INSERT INTO sessions` within it) up to the next
/// `.bind(` — this codebase's own convention is that every `db.prepare`
/// call is immediately followed by `.bind(`, so this reliably captures
/// just the one statement's column list without over- or under-matching.
fn sql_statement_text(content: &str, start: usize) -> &str {
    let after = &content[start..];
    let end = after.find(".bind(").unwrap_or(after.len());
    &after[..end]
}

/// From `start` (the byte offset of `INSERT INTO sessions`) through the
/// end of the immediately following `.bind(&[ ... ])?;` call — a fixed,
/// generous window rather than exact bracket-matching, since every real
/// call site's full `prepare(...).bind(...)` is well under this many
/// characters and a generous window only risks a false negative (missing
/// a `SessionProvenance::` reference that is genuinely there), never a
/// false positive.
fn rust_call_site_text(content: &str, start: usize) -> &str {
    let after = &content[start..];
    let end = after.len().min(2_000);
    let mut boundary = end;
    // Prefer to cut at the `.bind(...)?;` call's own closing `;` if it
    // falls inside the window, so the captured text does not spill into
    // unrelated code that happens to follow.
    if let Some(bind_start) = after[..end].find(".bind(")
        && let Some(close) = after[bind_start..end].find("])?;")
    {
        boundary = bind_start + close + "])?;".len();
    }
    &after[..boundary]
}

#[test]
fn rfc081_session_minting_sites_are_enumerated_and_set_a_provenance() {
    let mut files = Vec::new();
    walk_rs_files(&workers_ssr_src_dir(), &mut files);
    let src_dir = workers_ssr_src_dir();
    let mut seen_files = std::collections::HashSet::new();
    let mut unexpected: Vec<String> = Vec::new();

    for path in &files {
        let rel = path
            .strip_prefix(&src_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let mut search_from = 0;
        while let Some(rel_idx) = content[search_from..].find("INSERT INTO sessions") {
            let start = search_from + rel_idx;
            match KNOWN_SESSION_MINTING_SITES.iter().find(|s| s.file == rel) {
                Some(site) => {
                    seen_files.insert(site.file);
                    let statement = sql_statement_text(&content, start);
                    assert!(
                        statement.contains("provenance"),
                        "{rel}'s INSERT INTO sessions does not set provenance — every session \
                         must have one after migration 0012 (Handoff 048 §7.1); a session \
                         minted without it is refused by authorization (§7.3), fail-closed, but \
                         that is a production-time symptom of a bug this gate exists to catch \
                         at build time instead."
                    );
                    // Handoff 055 §7: extends this same gate rather than
                    // adding a second one — the same class of omission as
                    // a missing provenance, now for authenticated_at
                    // (migration 0015). A session minted without it is
                    // refused by the step-up predicate (fail-closed, NULL
                    // is never fresh), but that is a production-time
                    // symptom this gate exists to catch at build time.
                    assert!(
                        statement.contains("authenticated_at"),
                        "{rel}'s INSERT INTO sessions does not set authenticated_at — every \
                         session must have one after migration 0015 (Handoff 055 §5.1); a \
                         session minted without it is refused fresh by \
                         authz::is_fresh_for_account_operations (fail-closed), but that is a \
                         production-time symptom of a bug this gate exists to catch at build \
                         time instead."
                    );
                    // Handoff 054 §5.4: provenance must be written through
                    // `SessionProvenance`, never a bare SQL string literal
                    // — a typo'd literal is still a non-null string, which
                    // passes `decide_membership_scope`'s null check and is
                    // silently treated as an unscoped, first-class session.
                    for literal in ["'invite_redemption'", "'relink'", "'external_identity'"] {
                        assert!(
                            !statement.contains(literal),
                            "{rel}'s INSERT INTO sessions sets provenance with the SQL literal \
                             {literal} instead of a SessionProvenance value bound as a \
                             parameter — this is exactly the typo hazard Handoff 054 §5.4 \
                             introduced the type to close at compile time."
                        );
                    }
                    let call_site = rust_call_site_text(&content, start);
                    assert!(
                        call_site.contains("SessionProvenance::"),
                        "{rel}'s INSERT INTO sessions / .bind(...) call does not reference \
                         `SessionProvenance::` anywhere — provenance must be written through \
                         the type (Handoff 054 §5.4), not assembled from an untyped string \
                         built elsewhere."
                    );
                }
                None => unexpected.push(rel.clone()),
            }
            search_from = start + "INSERT INTO sessions".len();
        }
    }

    assert!(
        unexpected.is_empty(),
        "an INSERT INTO sessions occurs in a file not named in KNOWN_SESSION_MINTING_SITES: \
         {}\n\
         RFC-081 §2.1a's whole community-binding design assumes exactly two minting sites \
         (Handoff 048 §3). If this is a genuine third one, that is a Handoff 048 §17 stop \
         condition — it needs its own provenance/scope decision, not a silent pass. If it is a \
         false positive (e.g. a comment or test fixture containing the literal text), narrow \
         this gate's search rather than adding an exception for something that never mints a \
         session.",
        unexpected.join("\n")
    );

    for site in KNOWN_SESSION_MINTING_SITES {
        assert!(
            seen_files.contains(site.file),
            "KNOWN_SESSION_MINTING_SITES names {} ({}) but the walk never found an INSERT INTO \
             sessions there — stale table entry?",
            site.file,
            site.reason
        );
    }
}

fn scripts_smoke_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/smoke")
}

/// The JS template-literal string for one `INSERT INTO sessions` statement,
/// from `start` up to the closing backtick — every fixture in
/// `scripts/smoke/*.mjs` writes its statement as a single template literal,
/// so this reliably captures just that one statement (the `.bind(` analogue
/// used by `sql_statement_text` above doesn't apply here — these are raw D1
/// `execute` calls, not `db.prepare`/`.bind` pairs).
fn mjs_statement_text(content: &str, start: usize) -> &str {
    let after = &content[start..];
    let end = after.find('`').unwrap_or(after.len());
    &after[..end]
}

/// Handoff 049 §4.5: the smoke-fixture counterpart to
/// `rfc081_session_minting_sites_are_enumerated_and_set_a_provenance`
/// above. That gate pins the two *application* minting sites; this one
/// pins every *fixture* one. Deliberately a directory walk, not a curated
/// list of filenames — a curated list is exactly the shape that let 18 of
/// 19 fixtures go stale when migration 0012 landed (Handoff 048 §7, review
/// §7): nobody had to update a list, so nobody did. A future "fixture
/// twenty" is caught automatically because every `.mjs` file under
/// `scripts/smoke/` is walked, not just the ones named here.
#[test]
fn handoff049_smoke_session_fixtures_all_set_a_provenance() {
    let dir = scripts_smoke_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read directory {}: {e}", dir.display()));
    let mut checked = 0usize;
    let mut missing: Vec<String> = Vec::new();

    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("failed to read directory entry: {e}"))
            .path();
        if path.is_dir() || !path.extension().is_some_and(|ext| ext == "mjs") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let mut search_from = 0;
        while let Some(rel_idx) = content[search_from..].find("INSERT INTO sessions") {
            let start = search_from + rel_idx;
            checked += 1;
            let statement = mjs_statement_text(&content, start);
            if !statement.contains("provenance") {
                missing.push(format!("{name} (byte offset {start})"));
            }
            search_from = start + "INSERT INTO sessions".len();
        }
    }

    assert!(
        checked > 0,
        "found zero `INSERT INTO sessions` occurrences under scripts/smoke/ — has the directory \
         moved, or did every smoke fixture stop seeding sessions directly? This gate expects at \
         least the existing fixtures to still be there; an empty result likely means the gate \
         itself is broken, not that there is nothing left to check."
    );
    assert!(
        missing.is_empty(),
        "these smoke fixtures INSERT a session without setting provenance — after migration \
         0012 (Handoff 048 §7.1) every session must have one, or authorization's fail-closed \
         refusal (Handoff 048 §7.3 / Handoff 049 §4.2) rejects it at the first scope-checked \
         route it touches, silently, at smoke-run time rather than here. Set \
         provenance = 'invite_redemption' on the fixture (these all simulate a member who \
         joined by redeeming an invite):\n{}",
        missing.join("\n")
    );
}

/// Handoff 063 (F2 of the RFC-083 Slice D1a review): a documented exception
/// to the smoke-coverage gate below. `reason` must say why the script
/// genuinely cannot or should not be reachable by any `package.json` name —
/// never used to make an oversight pass quietly.
struct SmokeCoverageException {
    path: &'static str,
    reason: &'static str,
}

/// Empty by design: Handoff 063 gave every `scripts/smoke/*.mjs` file a
/// runnable name (including the three that had none —
/// `admin-role-transfer.mjs`, `help-signin.mjs`, `member-management.mjs`)
/// rather than excepting them. A future addition here should be rare and
/// should say why running the script by name isn't possible, not why nobody
/// got around to it.
const SMOKE_COVERAGE_EXCEPTIONS: &[SmokeCoverageException] = &[];

/// Handoff 063, origin F2 of the RFC-083 Slice D1a review: eight of
/// twenty-four smoke scripts were not running, and nobody knew, because the
/// run set was carried by hand from package to package — the same shape
/// `LOCALIZATION_EXCEPTIONS`' own comment describes for its predecessor,
/// "only checked a file if someone remembered to add it." This gate walks
/// every `scripts/smoke/*.mjs` file and fails on anything neither referenced
/// by some `package.json` script value nor listed in
/// `SMOKE_COVERAGE_EXCEPTIONS` with a written reason — an unlisted,
/// unreferenced file is a failure, not a silent pass.
#[test]
fn every_smoke_script_is_reachable_by_name_or_documented_exception() {
    let dir = scripts_smoke_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read directory {}: {e}", dir.display()));

    let mut checked = 0usize;
    let mut unreferenced: Vec<String> = Vec::new();
    let mut seen_exceptions = std::collections::HashSet::new();

    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("failed to read directory entry: {e}"))
            .path();
        if path.is_dir() || !path.extension().is_some_and(|ext| ext == "mjs") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        checked += 1;
        let referenced = PACKAGE_JSON_SRC.contains(&format!("scripts/smoke/{name}"));

        match SMOKE_COVERAGE_EXCEPTIONS.iter().find(|e| e.path == name) {
            Some(exc) => {
                seen_exceptions.insert(exc.path);
                assert!(
                    !referenced,
                    "{name} is both referenced by a package.json script AND listed in \
                     SMOKE_COVERAGE_EXCEPTIONS ({}) — remove the now-stale exception entry.",
                    exc.reason
                );
            }
            None => {
                if !referenced {
                    unreferenced.push(name);
                }
            }
        }
    }

    assert!(
        checked > 0,
        "found zero .mjs files under scripts/smoke/ — has the directory moved? This gate \
         expects the existing scripts to still be there; an empty result likely means the gate \
         itself is broken, not that there is nothing left to check."
    );
    assert!(
        unreferenced.is_empty(),
        "these scripts/smoke/*.mjs files are not referenced by any package.json script value \
         and are not in SMOKE_COVERAGE_EXCEPTIONS: {} — add a package.json script entry so the \
         file can be run by name, or add a pinned exception with a written reason. This is the \
         defect that let eight of twenty-four smoke scripts go unrun without anyone noticing.",
        unreferenced.join(", ")
    );
    for exc in SMOKE_COVERAGE_EXCEPTIONS {
        assert!(
            seen_exceptions.contains(exc.path),
            "SMOKE_COVERAGE_EXCEPTIONS names {} ({}) but no file with that name exists under \
             scripts/smoke/ — stale table entry?",
            exc.path,
            exc.reason
        );
    }
}

fn scripts_top_level_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts")
}

/// RFC-037 replay-test restoration (Handoff, 2026-08-26), Part D: a
/// documented exception to the top-level script coverage gate below.
/// `reason` must say why the script genuinely cannot or should not be
/// reachable by any `package.json` name — never used to make an oversight
/// pass quietly.
struct TopLevelScriptCoverageException {
    path: &'static str,
    reason: &'static str,
}

/// Empty by design, same as `SMOKE_COVERAGE_EXCEPTIONS` above: every
/// `scripts/*.mjs` file already has a `package.json` name today. This gate
/// exists so a *future* addition keeps that property, not because one is
/// missing now.
const TOP_LEVEL_SCRIPT_COVERAGE_EXCEPTIONS: &[TopLevelScriptCoverageException] = &[];

/// RFC-037 replay-test restoration (Handoff, 2026-08-26), Part D:
/// `every_smoke_script_is_reachable_by_name_or_documented_exception` above
/// only ever scanned `scripts/smoke/`. `scripts/test-form-token-replay-rejected.mjs`
/// and `scripts/collect-evidence-e4-concurrency.mjs` — the two scripts this
/// package restored — live one directory up, and nothing checked that they
/// (or any other top-level script) are even nameable, let alone run. A
/// directory-shaped blind spot, the same shape as the smoke-coverage gate's
/// own predecessor.
///
/// Walks every `scripts/*.mjs` file **directly under `scripts/`** —
/// `scripts/smoke/` (its own gate, above) and `scripts/lib/` (import-only
/// helper modules, never meant to be run by name at all — an exception
/// entry per file would be the wrong shape, so the directory itself is
/// excluded by construction via `path.is_dir()`) are both out of scope —
/// and fails on anything neither referenced by some `package.json` script
/// value nor listed in `TOP_LEVEL_SCRIPT_COVERAGE_EXCEPTIONS` with a
/// written reason.
///
/// This proves every top-level script is **nameable**. It deliberately does
/// not assert any of them belong in `smoke-all.mjs`'s routine sweep —
/// several top-level scripts (`bootstrap-cloudflare.mjs`,
/// `recover-community-access.mjs`) are operator tools against real
/// infrastructure, not tests, and folding every `test:`/`evidence:` name
/// into a routine local run would be a different, larger decision than
/// this gate makes. See this package's review request for that question,
/// raised rather than decided here.
#[test]
fn every_top_level_script_is_reachable_by_name_or_documented_exception() {
    let dir = scripts_top_level_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read directory {}: {e}", dir.display()));

    let mut checked = 0usize;
    let mut unreferenced: Vec<String> = Vec::new();
    let mut seen_exceptions = std::collections::HashSet::new();

    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("failed to read directory entry: {e}"))
            .path();
        if path.is_dir() || !path.extension().is_some_and(|ext| ext == "mjs") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        checked += 1;
        let referenced = PACKAGE_JSON_SRC.contains(&format!("scripts/{name}"));

        match TOP_LEVEL_SCRIPT_COVERAGE_EXCEPTIONS
            .iter()
            .find(|e| e.path == name)
        {
            Some(exc) => {
                seen_exceptions.insert(exc.path);
                assert!(
                    !referenced,
                    "{name} is both referenced by a package.json script AND listed in \
                     TOP_LEVEL_SCRIPT_COVERAGE_EXCEPTIONS ({}) — remove the now-stale exception \
                     entry.",
                    exc.reason
                );
            }
            None => {
                if !referenced {
                    unreferenced.push(name);
                }
            }
        }
    }

    assert!(
        checked > 0,
        "found zero .mjs files directly under scripts/ — has the directory moved? This gate \
         expects the existing top-level scripts to still be there; an empty result likely means \
         the gate itself is broken, not that there is nothing left to check."
    );
    assert!(
        unreferenced.is_empty(),
        "these scripts/*.mjs files are not referenced by any package.json script value and are \
         not in TOP_LEVEL_SCRIPT_COVERAGE_EXCEPTIONS: {} — add a package.json script entry so \
         the file can be run by name, or add a pinned exception with a written reason. This is \
         the sibling defect to Handoff 063's smoke-coverage gate, one directory up.",
        unreferenced.join(", ")
    );
    for exc in TOP_LEVEL_SCRIPT_COVERAGE_EXCEPTIONS {
        assert!(
            seen_exceptions.contains(exc.path),
            "TOP_LEVEL_SCRIPT_COVERAGE_EXCEPTIONS names {} ({}) but no file with that name \
             exists directly under scripts/ — stale table entry?",
            exc.path,
            exc.reason
        );
    }
}

/// Handoff 076 (F1 of the Handoff 076 review): a documented exception to the
/// Accept-Language pin gate below. `reason` must say why the script
/// genuinely does not need the pin (e.g. it never opens a page at all) —
/// never used to make a missed import pass quietly.
struct SmokeLanguagePinException {
    path: &'static str,
    reason: &'static str,
}

/// Empty by design: every `scripts/smoke/*.mjs` that launches a sandboxed
/// Chromium was given the pin (Handoff 076) rather than excepted. A future
/// addition here should be rare and should say why the pin genuinely
/// doesn't apply, not why nobody got around to it.
const SMOKE_LANGUAGE_PIN_EXCEPTIONS: &[SmokeLanguagePinException] = &[];

/// Handoff 076, origin F1 of its own review
/// (`.git-exclude/reviewed/zinnias-ciao-main-2026-08-16-pin-smoke-accept-language-and-prove-rung-2-review.md`):
/// a hand-executed sweep (`grep -l "headless=new"` cross-referenced against
/// `grep -l "smoke-locale"`) missed `account-recovery-and-unlink.mjs` —
/// the sixth time in this series a manually-checked population went stale
/// (`LOCALIZATION_EXCEPTIONS`, the smoke run set, the parity stem list, the
/// identical-pair array, the locale-blind helper check, and now this).
/// The fix is the same each time: derive the population instead of sweeping
/// it by hand. This gate walks every `scripts/smoke/*.mjs` file and fails on
/// any that launches Chromium (`--headless=new`) without also importing
/// `scripts/lib/smoke-locale.mjs` (the shared Accept-Language pin,
/// `SMOKE_ACCEPT_LANGUAGE`) — unless the file is listed in
/// `SMOKE_LANGUAGE_PIN_EXCEPTIONS` with a written reason. A smoke that opens
/// a page without the pin lets that page's rendered language, on an
/// anonymous or re-authenticating route, depend on the developer machine's
/// `LANG` rather than a fixed value — exactly the defect this whole package
/// exists to remove.
#[test]
fn every_chromium_smoke_pins_accept_language_or_documented_exception() {
    let dir = scripts_smoke_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read directory {}: {e}", dir.display()));

    let mut checked = 0usize;
    let mut launches_chromium = 0usize;
    let mut unpinned: Vec<String> = Vec::new();
    let mut seen_exceptions = std::collections::HashSet::new();

    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("failed to read directory entry: {e}"))
            .path();
        if path.is_dir() || !path.extension().is_some_and(|ext| ext == "mjs") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        checked += 1;
        if !content.contains("headless=new") {
            continue;
        }
        launches_chromium += 1;
        let pinned = content.contains("smoke-locale.mjs");

        match SMOKE_LANGUAGE_PIN_EXCEPTIONS
            .iter()
            .find(|e| e.path == name)
        {
            Some(exc) => {
                seen_exceptions.insert(exc.path);
                assert!(
                    !pinned,
                    "{name} both imports scripts/lib/smoke-locale.mjs AND is listed in \
                     SMOKE_LANGUAGE_PIN_EXCEPTIONS ({}) — remove the now-stale exception entry.",
                    exc.reason
                );
            }
            None => {
                if !pinned {
                    unpinned.push(name);
                }
            }
        }
    }

    assert!(
        checked > 0,
        "found zero .mjs files under scripts/smoke/ — has the directory moved? This gate \
         expects the existing scripts to still be there; an empty result likely means the gate \
         itself is broken, not that there is nothing left to check."
    );
    assert!(
        launches_chromium > 0,
        "found zero scripts/smoke/*.mjs files launching Chromium (`--headless=new`) — has every \
         smoke stopped using a real browser, or is this gate's own marker string stale?"
    );
    assert!(
        unpinned.is_empty(),
        "these scripts/smoke/*.mjs files launch Chromium without importing \
         scripts/lib/smoke-locale.mjs and are not in SMOKE_LANGUAGE_PIN_EXCEPTIONS: {} — merge \
         SMOKE_ACCEPT_LANGUAGE into every Network.setExtraHTTPHeaders call the script makes, or \
         add a pinned exception with a written reason. This is the defect that let \
         account-recovery-and-unlink.mjs go unpinned without anyone noticing.",
        unpinned.join(", ")
    );
    for exc in SMOKE_LANGUAGE_PIN_EXCEPTIONS {
        assert!(
            seen_exceptions.contains(exc.path),
            "SMOKE_LANGUAGE_PIN_EXCEPTIONS names {} ({}) but no file with that name exists under \
             scripts/smoke/, or it does not launch Chromium — stale table entry?",
            exc.path,
            exc.reason
        );
    }
}

/// Handoff 078: a documented exception to the fixture-locale-pin gate
/// below. `reason` must say why the script genuinely does not need the pin
/// (e.g. it asserts nothing locale-dependent despite the pattern match) —
/// never used to make a missed import pass quietly.
struct SmokeFixtureLocalePinException {
    path: &'static str,
    reason: &'static str,
}

/// One entry, found while proving the shared pin's placement (RFC-085 §10
/// / Handoff 078 §10's own named risk): `language-preference.mjs`'s
/// `otherMembershipUnaffectedThroughout` check proves the language switch
/// is membership-scoped by asserting a *second*, deliberately-untouched
/// membership's `ui_language` stays `NULL` forever. The shared blanket
/// pin (`WHERE ui_language IS NULL`) would set that row to `'ja'` too,
/// making the check pass by construction and proving nothing — exactly
/// the failure mode this handoff's §10 warned about. That file pins only
/// its own membership under test, by id, inline, instead of importing the
/// shared helper. Confirmed by running the temporary `PRODUCT_DEFAULT`
/// flip (Handoff 078 §5): without this exception the gate would demand an
/// import this file must not add.
const SMOKE_FIXTURE_LOCALE_PIN_EXCEPTIONS: &[SmokeFixtureLocalePinException] = &[
    SmokeFixtureLocalePinException {
        path: "language-preference.mjs",
        reason: "manages ui_language directly with its own scoped UPDATE (memberMembershipId only) — the shared blanket pin would overwrite otherMembershipId's deliberate NULL, which otherMembershipUnaffectedThroughout depends on to prove per-membership scoping",
    },
];

/// The same codepoint ranges every Rust render test in this codebase
/// already uses (e.g. `handlers/account/tests.rs`'s local copy) — Hiragana/
/// Katakana, CJK Unified Ideographs, CJK punctuation, and fullwidth forms.
fn contains_japanese_codepoint(s: &str) -> bool {
    s.chars().any(|c| {
        let cp = c as u32;
        (0x3040..=0x30FF).contains(&cp)
            || (0x4E00..=0x9FFF).contains(&cp)
            || (0x3000..=0x303F).contains(&cp)
            || (0xFF00..=0xFFEF).contains(&cp)
    })
}

/// Handoff 078: a smoke depends on the fixture-locale pin if it asserts
/// **either** a literal Japanese codepoint (a rendered string) **or** a
/// hardcoded `=== 'ja'` / `=== "ja"` comparison (an `html lang` check) —
/// checked against `rfc075-slice4/5/6/7-*.mjs`, which assert
/// `htmlLangJa: observed.htmlLang === 'ja'` with **zero** Japanese
/// codepoints anywhere in the file, so the codepoint check alone misses a
/// real dependency. Comments stripped first, so an explanatory comment
/// mentioning either shape (this function's own doc comment, for one)
/// cannot make a file look like it needs the pin when it does not.
fn asserts_japanese_locale(content: &str) -> bool {
    let production = strip_line_comments(content);
    contains_japanese_codepoint(&production)
        || production.contains("=== 'ja'")
        || production.contains("=== \"ja\"")
}

/// Handoff 078 (Handoff 076's own precedent, derived not swept): no fixture
/// sets `ui_language`, and no application insert path backfills it either,
/// so every seeded membership is `NULL` and every signed-in page resolves
/// through `Locale::PRODUCT_DEFAULT` — Japanese at the time this gate was
/// written, English since Handoff 079 took ROADMAP.md's English-default
/// decision. Flipping that one line (RFC-085 reduced the decision to
/// exactly that) would have flipped every Japanese-asserting smoke with
/// it, for a reason that had nothing to do with the product — the same
/// ambient-state dependence Handoff 076 removed for `Accept-Language`.
/// This gate's own assertions never depended on which value
/// `PRODUCT_DEFAULT` held (it checks import presence, not rendered
/// language), so the flip changed nothing here but this comment's tense.
/// This gate walks
/// every `scripts/smoke/*.mjs` file and fails on any that asserts Japanese
/// locale (per [`asserts_japanese_locale`]) without importing
/// `scripts/lib/smoke-fixture-locale.mjs` (`PIN_FIXTURE_UI_LANGUAGE_TO_JAPANESE_SQL`),
/// unless the file is listed in `SMOKE_FIXTURE_LOCALE_PIN_EXCEPTIONS` with a
/// written reason.
#[test]
fn every_japanese_asserting_smoke_pins_fixture_ui_language_or_documented_exception() {
    let dir = scripts_smoke_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read directory {}: {e}", dir.display()));

    let mut checked = 0usize;
    let mut asserts_japanese = 0usize;
    let mut unpinned: Vec<String> = Vec::new();
    let mut seen_exceptions = std::collections::HashSet::new();

    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("failed to read directory entry: {e}"))
            .path();
        if path.is_dir() || !path.extension().is_some_and(|ext| ext == "mjs") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        checked += 1;
        if !asserts_japanese_locale(&content) {
            continue;
        }
        asserts_japanese += 1;
        let pinned = content.contains("smoke-fixture-locale.mjs");

        match SMOKE_FIXTURE_LOCALE_PIN_EXCEPTIONS
            .iter()
            .find(|e| e.path == name)
        {
            Some(exc) => {
                seen_exceptions.insert(exc.path);
                assert!(
                    !pinned,
                    "{name} both imports scripts/lib/smoke-fixture-locale.mjs AND is listed in \
                     SMOKE_FIXTURE_LOCALE_PIN_EXCEPTIONS ({}) — remove the now-stale exception \
                     entry.",
                    exc.reason
                );
            }
            None => {
                if !pinned {
                    unpinned.push(name);
                }
            }
        }
    }

    assert!(
        checked > 0,
        "found zero .mjs files under scripts/smoke/ — has the directory moved? This gate \
         expects the existing scripts to still be there; an empty result likely means the gate \
         itself is broken, not that there is nothing left to check."
    );
    assert!(
        asserts_japanese > 0,
        "found zero scripts/smoke/*.mjs files asserting Japanese locale — has every Japanese \
         assertion been removed, or is this gate's own detection stale?"
    );
    assert!(
        unpinned.is_empty(),
        "these scripts/smoke/*.mjs files assert Japanese locale without importing \
         scripts/lib/smoke-fixture-locale.mjs and are not in \
         SMOKE_FIXTURE_LOCALE_PIN_EXCEPTIONS: {} — call \
         sql(PIN_FIXTURE_UI_LANGUAGE_TO_JAPANESE_SQL) after every step that can create a \
         membership (fixture seeding, and any application-created membership mid-scenario), or \
         add a pinned exception with a written reason. Flipping Locale::PRODUCT_DEFAULT would \
         flip this smoke's language with it otherwise.",
        unpinned.join(", ")
    );
    for exc in SMOKE_FIXTURE_LOCALE_PIN_EXCEPTIONS {
        assert!(
            seen_exceptions.contains(exc.path),
            "SMOKE_FIXTURE_LOCALE_PIN_EXCEPTIONS names {} ({}) but no file with that name exists \
             under scripts/smoke/, or it no longer asserts Japanese locale — stale table entry?",
            exc.path,
            exc.reason
        );
    }
}

/// Handoff 063 §3.3's cross-language pin: `recurrence-v2.mjs` cannot import
/// `RECURRENCE_MATERIALIZATION_MONTHS_AHEAD` from Rust, so it carries its own
/// literal copy, used to derive a "definitely outside the horizon" Calendar
/// month at smoke-run time. This reads the *live* constant (not a duplicated
/// number on this side) and fails if the JS literal drifts from it — the
/// exact way a hardcoded `?month=2027-02` silently stopped meaning "far
/// future" once real time caught up to it.
#[test]
fn rfc065_recurrence_smoke_pins_the_materialization_horizon_constant() {
    let expected = zinnias_ciao_domain::event_admin::RECURRENCE_MATERIALIZATION_MONTHS_AHEAD;
    let needle = format!("const RECURRENCE_MATERIALIZATION_MONTHS_AHEAD = {expected};");
    assert!(
        RECURRENCE_V2_SMOKE_SRC.contains(&needle),
        "scripts/smoke/recurrence-v2.mjs does not contain `{needle}` — its far-future Calendar \
         month is derived from this literal, and it must track \
         packages/domain/src/event_admin.rs's RECURRENCE_MATERIALIZATION_MONTHS_AHEAD (currently \
         {expected}) exactly, or the smoke's \"beyond the horizon\" assertion silently stops \
         meaning that."
    );
}

/// RFC-080 §3.2 (Handoff 050 §6): namespaces are created by migration or
/// reviewed configuration, **never at runtime from a token** — that is the
/// whole point of a namespace being a reviewed provider registration
/// rather than something a callback can mint on the fly. Unlike the two
/// session-minting gates above, there is no legitimate application-code
/// site to name: every occurrence of `INSERT INTO identity_namespaces`
/// under `workers/ssr/src` is unconditionally wrong. Default-fail with no
/// exceptions table.
#[test]
fn rfc080_identity_namespaces_are_never_created_outside_a_migration() {
    let mut files = Vec::new();
    walk_rs_files(&workers_ssr_src_dir(), &mut files);
    let src_dir = workers_ssr_src_dir();
    let mut offenders: Vec<String> = Vec::new();

    for path in &files {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        if content.contains("INSERT INTO identity_namespaces") {
            let rel = path
                .strip_prefix(&src_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            offenders.push(rel);
        }
    }

    assert!(
        offenders.is_empty(),
        "found `INSERT INTO identity_namespaces` under workers/ssr/src in: {} — RFC-080 §3.2 \
         requires every namespace to come from a migration or reviewed configuration, never \
         from application code at request time. A namespace minted from a token would let a \
         request forge the reviewed-registration guarantee this table exists to provide. If \
         this is genuinely needed, that is a stop condition (Handoff 050 §14), not something \
         to except here.",
        offenders.join(", ")
    );
}

fn identity_module_dir() -> std::path::PathBuf {
    workers_ssr_src_dir().join("identity")
}

/// Handoff 053 §6.1: 4a performs no HTTP of any kind — a stray `fetch` is
/// the difference between a testable, no-network boundary and one that
/// silently needs a live provider. Default-fail: every file under
/// `workers/ssr/src/identity/` is scanned for the substring `fetch`
/// (case-insensitive, so a `Fetch`-typed network call can't slip in under
/// a differently-cased spelling either), and none may contain it.
#[test]
fn identity_module_makes_no_network_call() {
    let mut files = Vec::new();
    walk_rs_files(&identity_module_dir(), &mut files);
    assert!(
        !files.is_empty(),
        "found zero .rs files under workers/ssr/src/identity/ — has the module moved? This \
         gate expects the module this handoff added to still be there."
    );

    let mut offenders: Vec<String> = Vec::new();
    for path in &files {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        if content.to_ascii_lowercase().contains("fetch") {
            offenders.push(
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
        }
    }

    assert!(
        offenders.is_empty(),
        "found the substring `fetch` (case-insensitive) in: {} — Handoff 053 §2/§6.1 requires \
         the identity module to make no network call of any kind. If a real provider adapter \
         genuinely needs one, that is Slice 4b's job in a route-reachable file, not this \
         module's.",
        offenders.join(", ")
    );
}

/// Handoff 053 §6.2: the fake issuer (RFC-080 §10) is a required
/// deliverable, but only as a test-only mechanism — it must not be
/// reachable from a non-test build. The guarantee is structural, not a
/// convention to remember: `identity/mod.rs` declares the module behind
/// `#[cfg(test)]`, so `fake_issuer.rs` is entirely absent from a release
/// build's compiled output, not merely unreferenced by it. This gate
/// checks that declaration is actually there, in the specific two-line
/// shape that makes it true.
#[test]
fn identity_fake_issuer_is_test_only() {
    let mod_rs = identity_module_dir().join("mod.rs");
    let content = std::fs::read_to_string(&mod_rs)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", mod_rs.display()));

    assert!(
        content.contains("fake_issuer.rs") || identity_module_dir().join("fake_issuer.rs").exists(),
        "workers/ssr/src/identity/fake_issuer.rs is missing — has the fake issuer (RFC-080 §10, \
         a required deliverable) been removed?"
    );

    let guarded = content
        .lines()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair[0].trim() == "#[cfg(test)]" && pair[1].trim() == "mod fake_issuer;");

    assert!(
        guarded,
        "identity/mod.rs must declare `mod fake_issuer;` immediately behind its own \
         `#[cfg(test)]` line — found the declaration, but not in that exact guarded shape. A \
         fake_issuer reachable from a non-test build is the difference between a test-only \
         mechanism and a live one; do not weaken this to a doc comment or a runtime check."
    );
}

/// Handoff 056 §6 gate 1, **relaxed (not deleted) by Handoff 057 §6 gate
/// 1**: linking must be additive by construction everywhere except the one
/// legitimate unlink path Handoff 057 adds. Default-fail, same
/// exceptions-table shape as `KNOWN_SESSION_MINTING_SITES` /
/// `ADMIN_CLASS_LEAK_EXCEPTIONS`: every `UPDATE user_identities SET status`
/// anywhere under `workers/ssr/src` must be named here with a written
/// reason, or the walk fails — a second, un-reviewed unlink path appearing
/// later is exactly what this exists to catch. `DELETE FROM
/// user_identities` stays unconditionally forbidden with **no** exception
/// ever granted — a hard delete is never legitimate, matching this
/// schema's established revoke-not-delete discipline everywhere else.
struct UnlinkExceptionSite {
    file: &'static str,
    reason: &'static str,
}

const USER_IDENTITIES_UNLINK_EXCEPTIONS: &[UnlinkExceptionSite] = &[UnlinkExceptionSite {
    file: "db/identity.rs",
    reason: "the one legitimate unlink path (RFC-081 §3.3, Handoff 057 §5.3) — \
             unlink_required's claim is a single, EXISTS-guarded UPDATE that can only ever \
             affect a row when at least one other verified usable method remains",
}];

#[test]
fn no_unlink_path_exists_for_user_identities() {
    let mut files = Vec::new();
    walk_rs_files(&workers_ssr_src_dir(), &mut files);
    let src_dir = workers_ssr_src_dir();
    let mut seen_files = std::collections::HashSet::new();
    let mut offenders: Vec<String> = Vec::new();

    for path in &files {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let rel = path
            .strip_prefix(&src_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if content.contains("DELETE FROM user_identities") {
            offenders.push(format!(
                "{rel}: DELETE FROM user_identities (never permitted — no exception exists for \
                 a hard delete)"
            ));
        }
        if content.contains("UPDATE user_identities SET status") {
            match USER_IDENTITIES_UNLINK_EXCEPTIONS
                .iter()
                .find(|s| s.file == rel)
            {
                Some(site) => {
                    seen_files.insert(site.file);
                }
                None => offenders.push(format!("{rel}: UPDATE user_identities SET status")),
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "found an unlink-capable SQL statement against user_identities not named in \
         USER_IDENTITIES_UNLINK_EXCEPTIONS: {} — RFC-081 §4 / §3.3 requires linking to stay \
         additive by construction everywhere except the one reviewed unlink path; a genuine \
         second unlink path needs its own review and a table entry with a written reason, not a \
         silent pass.",
        offenders.join(", ")
    );

    for site in USER_IDENTITIES_UNLINK_EXCEPTIONS {
        assert!(
            seen_files.contains(site.file),
            "USER_IDENTITIES_UNLINK_EXCEPTIONS names {} ({}) but the walk never found an UPDATE \
             user_identities SET status there — stale table entry?",
            site.file,
            site.reason
        );
    }

    // Defence in depth, matching `rfc081_session_minting_sites_are_enumerated_and_set_a_provenance`'s
    // own per-site assertions: the named exception's own statement must
    // actually be guarded, not merely present. A regression here (the
    // `EXISTS` guard silently dropped) would still pass the exceptions-
    // table check above, since that check only looks for the statement's
    // fixed prefix — this is what catches the guard itself disappearing.
    let unlink_statement_start = IDENTITY_DB_SRC
        .find("UPDATE user_identities SET status = 'revoked'")
        .expect("db/identity.rs must still contain the named unlink statement");
    let unlink_statement = rust_call_site_text(IDENTITY_DB_SRC, unlink_statement_start);
    assert!(
        unlink_statement.contains("status = 'active'")
            && unlink_statement.contains("usable_method_exists_sql("),
        "db/identity.rs's unlink UPDATE must stay guarded by both the current-status check and \
         a call to usable_method_exists_sql — an unguarded version of this exact statement is \
         the final-credential-lockout bug RFC-081 §3.3 exists to prevent"
    );
    assert!(
        RECOVERY_DB_SRC.contains("account_recovery_credentials")
            && RECOVERY_DB_SRC.contains("user_identities"),
        "usable_method_exists_sql must still check both halves of the usable-method definition"
    );
}

/// Handoff 056 §6 gate 2: `prompt=login` must be sent for every `link` \
/// authorization request and for a `sign_in` re-authentication — never a \
/// hardcoded bypass of the one decision function that determines this \
/// (`should_send_prompt_login`, exhaustively unit-tested in \
/// `handlers/identity/tests.rs`). Checks both real call sites of \
/// `start_oidc_transaction`: `get_start`'s own body must reference the \
/// decision function directly (not a literal `true`/`false` in its place); \
/// `handlers/account/link.rs::post_link`'s call must pass the literal \
/// `true` its own function's unconditional `link` case computes to —
/// proven equivalent to calling the decision function by
/// `link_always_sends_prompt_login_regardless_of_session_state`, not
/// merely assumed.
#[test]
fn prompt_login_is_sent_for_link_and_reauthentication() {
    let identity_mod_path = workers_ssr_src_dir().join("handlers/identity/mod.rs");
    let identity_mod = std::fs::read_to_string(&identity_mod_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", identity_mod_path.display()));
    let get_start_body = compact_brace_block(&identity_mod, "pub async fn get_start");
    assert!(
        get_start_body.contains("should_send_prompt_login("),
        "get_start must decide whether to send prompt=login through \
         should_send_prompt_login, not a hardcoded bypass"
    );

    let start_oidc_transaction_body =
        compact_brace_block(&identity_mod, "pub(crate) async fn start_oidc_transaction");
    assert!(
        start_oidc_transaction_body.contains("prompt=login")
            && start_oidc_transaction_body.contains("send_prompt_login"),
        "start_oidc_transaction must build the prompt=login query fragment conditionally on \
         its own send_prompt_login parameter, not unconditionally"
    );

    let link_path = workers_ssr_src_dir().join("handlers/account/link.rs");
    let link_source = std::fs::read_to_string(&link_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", link_path.display()));
    // Deliberately the *raw* source, not `compact_brace_block`'s output —
    // that helper collapses all whitespace (`split_whitespace().collect()`,
    // built for exact-shape matching elsewhere in this file), which would
    // destroy the argument-list structure this check needs to parse.
    assert!(
        link_source.contains("\"link\","),
        "post_link must start a transaction with action \"link\""
    );
    // The call's own argument list, from `start_oidc_transaction(` through
    // its matching close paren — bounded rather than exact-formatted, so
    // this survives an ordinary `cargo fmt` reflow.
    let call_start = link_source
        .find("start_oidc_transaction(")
        .expect("post_link must call start_oidc_transaction");
    let call_site = &link_source[call_start..];
    let call_end = call_site
        .find(")\n")
        .map(|offset| offset + 1)
        .unwrap_or(call_site.len());
    let arguments = &call_site[..call_end];
    // Handoff 075 added an eighth argument (the locale — see below), so
    // `send_prompt_login` is now the second-from-last segment, not the
    // last. Collected in reverse (last argument first) rather than
    // re-anchored to a fixed count, so this keeps working if another
    // trailing argument is ever added the same way.
    let trailing_segments: Vec<&str> = arguments
        .trim_end_matches([')', '\n', ' '])
        .rsplit(',')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect();
    let last_argument = trailing_segments.first().copied().unwrap_or("");
    let send_prompt_login_argument = trailing_segments.get(1).copied().unwrap_or("");
    assert_eq!(
        send_prompt_login_argument, "true",
        "post_link's start_oidc_transaction call must pass true for send_prompt_login (found \
         {send_prompt_login_argument:?}) — link's own case is unconditional (proven by \
         link_always_sends_prompt_login_regardless_of_session_state), so a literal true here is \
         the correct, equivalent value, not a bypass of a different decision"
    );
    // RFC-084 (Handoff 084) discharged D2b: account/link.rs now resolves a
    // real account-tier locale (`authz::resolve_account_locale`) instead of
    // the literal `Locale::Ja` placeholder Handoff 075 pinned here. The call
    // site must pass a resolved value, never a literal — a literal here
    // would mean the resolution call was removed or never wired to this
    // argument, exactly the regression this assertion now exists to catch.
    assert_ne!(
        last_argument, "Locale::Ja",
        "post_link's start_oidc_transaction call still passes the literal Locale::Ja — RFC-084 \
         replaced this D2b placeholder with a real resolution; a literal here means the \
         resolution call was removed or never wired to this call site"
    );
    assert_ne!(
        last_argument, "Locale::En",
        "post_link's start_oidc_transaction call passes a literal Locale::En — RFC-084 requires \
         a resolved value from authz::resolve_account_locale, never a hardcoded one"
    );
    assert!(
        link_source.contains("resolve_account_locale("),
        "post_link must resolve its locale through authz::resolve_account_locale, not compute \
         it ad hoc or hardcode one"
    );
}

/// RFC-082 §3 / Handoff 058 §7: the structural control behind the
/// two-predicate rule. `MEMBERSHIP_ACTIVE` and `MEMBERSHIP_PRESENT`
/// (`db/membership.rs`) are defined exactly once; every one of the 54
/// pre-existing call sites (plus every new one this package adds) must
/// interpolate one of the two, never spell `removed_at IS NULL` inline.
/// Default-fail, comments stripped first — the third occurrence of the
/// "gate matches its own explanatory prose" failure mode in this project
/// (after the flash gate and Handoff 057's abuse-limiter gate) is treated
/// as the standing rule, not rediscovered here: this gate strips comments
/// from the start rather than finding the need the hard way a fourth time.
/// The gate is the control; the per-site classification in the review
/// request is the evidence — neither alone proves every site chose
/// correctly, only that no site spells the predicate inline.
#[test]
fn rfc082_no_inline_membership_active_predicate_outside_the_two_constants() {
    let src_dir = workers_ssr_src_dir();
    let mut files = Vec::new();
    walk_rs_files(&src_dir, &mut files);
    let files: Vec<_> = files
        .into_iter()
        .filter(|p| p.file_name().and_then(|n| n.to_str()) != Some("tests.rs"))
        .collect();
    assert!(
        files.len() > 80,
        "expected many .rs files under workers/ssr/src, found only {} — directory walk is \
         probably broken, not the codebase actually shrinking",
        files.len()
    );

    let mut violations: Vec<String> = Vec::new();
    for path in &files {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let stripped = strip_line_comments(&content);
        let rel = path
            .strip_prefix(&src_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        for (line_no, line) in stripped.lines().enumerate() {
            if !line.contains("removed_at IS NULL") {
                continue;
            }
            let is_active_constant_definition = rel == "db/membership.rs"
                && line.contains("const MEMBERSHIP_ACTIVE")
                && line.contains("removed_at IS NULL AND suspended_at IS NULL");
            let is_present_constant_definition =
                rel == "db/membership.rs" && line.contains("const MEMBERSHIP_PRESENT");
            if is_active_constant_definition || is_present_constant_definition {
                continue;
            }
            violations.push(format!("{rel}:{}: {}", line_no + 1, line.trim()));
        }
    }

    assert!(
        violations.is_empty(),
        "removed_at IS NULL spelled inline outside the two named predicates in \
         db/membership.rs (MEMBERSHIP_ACTIVE / MEMBERSHIP_PRESENT) — every activeness query \
         must interpolate one of the two via format!(\"... {{MEMBERSHIP_ACTIVE}} ...\") instead, \
         never spell the condition out itself:\n{}",
        violations.join("\n")
    );
}

/// RFC-082 §2 / §5.1: the migration is additive only — two nullable
/// columns, no table rebuild — and the partial-index comment in migration
/// 0001 already anticipated a suspended row occupying the
/// (community_id, user_id) pair.
#[test]
fn rfc082_migration_0018_is_additive_and_index_compatible() {
    assert!(
        MIGRATION_0018_SRC.contains("ALTER TABLE community_memberships ADD COLUMN suspended_at")
            && MIGRATION_0018_SRC.contains(
                "ALTER TABLE community_memberships ADD COLUMN suspended_by_membership_id"
            )
            && MIGRATION_0018_SRC.contains("REFERENCES community_memberships(id)"),
        "migration 0018 must add exactly the two nullable columns RFC-082 §2 specifies"
    );
    for forbidden in ["DROP TABLE", "CREATE TABLE", "RENAME TABLE"] {
        assert!(
            !MIGRATION_0018_SRC.contains(forbidden),
            "migration 0018 must be purely additive — found {forbidden:?}, which implies a \
             rebuild RFC-082 §2 says is deliberately avoided"
        );
    }

    const MIGRATION_0001_SRC: &str = include_str!("../../../migrations/0001_initial.sql");
    assert!(
        MIGRATION_0001_SRC.contains("idx_memberships_one_active_per_user")
            && MIGRATION_0001_SRC.contains("WHERE removed_at IS NULL")
            && MIGRATION_0001_SRC.to_ascii_lowercase().contains("suspend"),
        "the partial unique index (migration 0001) must remain WHERE removed_at IS NULL — a \
         suspended row is not removed and must still occupy the (community_id, user_id) pair — \
         and its own comment must already name suspension, confirmed rather than assumed \
         (RFC-082 §5.1)"
    );
}

/// RFC-082 §5.2 / §1: `suspend_required` and `unsuspend_required` must
/// scope their guarded UPDATE the same way every other role-changing
/// mutation in this file does — actor re-checked `MEMBERSHIP_ACTIVE` and
/// `role = 'admin'` in the same statement, target state re-checked to
/// match the RFC-082 §1 transition being attempted, and (for suspend) the
/// at-least-one-admin invariant preserved.
#[test]
fn rfc082_suspend_and_unsuspend_writes_are_scoped_and_guarded() {
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn suspend_required")
            && MEMBERSHIP_DB_SRC.contains("SET suspended_at = ?1, suspended_by_membership_id = ?4")
            && MEMBERSHIP_DB_SRC.contains("AuditAction::MembershipSuspended"),
        "suspend_required must exist, set both suspension columns together, and audit as \
         MembershipSuspended"
    );
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn unsuspend_required")
            && MEMBERSHIP_DB_SRC
                .contains("SET suspended_at = NULL, suspended_by_membership_id = NULL")
            && MEMBERSHIP_DB_SRC.contains("AuditAction::MembershipUnsuspended"),
        "unsuspend_required must exist, clear both suspension columns together, and audit as \
         MembershipUnsuspended"
    );

    // active -> suspended: the target's *current* state must be ACTIVE
    // (removed_at IS NULL AND suspended_at IS NULL) before suspending it —
    // MEMBERSHIP_ACTIVE covers both terminal-removal and already-suspended
    // as refused starting states in one predicate.
    let suspend_start = MEMBERSHIP_DB_SRC
        .find("pub async fn suspend_required")
        .expect("suspend_required should exist");
    let suspend_end = MEMBERSHIP_DB_SRC[suspend_start..]
        .find("pub async fn unsuspend_required")
        .map(|offset| suspend_start + offset)
        .expect("unsuspend_required should follow suspend_required");
    let suspend_fn = &MEMBERSHIP_DB_SRC[suspend_start..suspend_end];
    // Comments stripped first (the standing rule this project now applies
    // to every source-scanning gate, per Handoff 058 §7): suspend_required's
    // own doc comment mentions `MEMBERSHIP_ACTIVE` in prose, which would
    // otherwise inflate this count without the SQL itself changing.
    let suspend_fn_code = strip_line_comments(suspend_fn);
    assert_eq!(
        suspend_fn_code.matches("MEMBERSHIP_ACTIVE").count(),
        3,
        "suspend_required's mutation must re-check MEMBERSHIP_ACTIVE three times: the target's \
         own current state, the actor's admin membership, and the admin-count subquery"
    );
    assert!(
        suspend_fn.contains("role != 'admin' OR")
            && suspend_fn.contains("SELECT COUNT(*) FROM community_memberships")
            && suspend_fn.contains("> 1"),
        "suspend_required must preserve the at-least-one-admin invariant the same way \
         soft_remove_guarded_required does — suspending a community's last active admin would \
         leave nobody who could ever unsuspend anyone"
    );
    assert!(
        suspend_fn.contains("id != ?4"),
        "suspend_required must deny self-targeting in SQL, not only in the handler"
    );

    // suspended -> active: the target's *current* state must be PRESENT
    // and specifically suspended (suspended_at IS NOT NULL) — PRESENT
    // alone would also match an already-active row.
    let unsuspend_fn = &MEMBERSHIP_DB_SRC[suspend_end..];
    assert!(
        unsuspend_fn.contains("MEMBERSHIP_PRESENT")
            && unsuspend_fn.contains("suspended_at IS NOT NULL"),
        "unsuspend_required's target check must be MEMBERSHIP_PRESENT AND suspended_at IS NOT \
         NULL — PRESENT alone would also match an already-active membership, which must not be \
         accepted by an unsuspend"
    );
    // Comments stripped first, same reason as suspend_fn_code above —
    // unsuspend_required's own doc comment also mentions MEMBERSHIP_ACTIVE.
    let unsuspend_fn_code = strip_line_comments(unsuspend_fn);
    assert_eq!(
        unsuspend_fn_code.matches("MEMBERSHIP_ACTIVE").count(),
        1,
        "unsuspend_required's actor check must be MEMBERSHIP_ACTIVE — a suspended admin (who \
         could not have suspended themselves, since suspend denies self-targeting) must not be \
         able to unsuspend anyone either"
    );

    // active|suspended -> removed: soft_remove_guarded_required's target
    // check is MEMBERSHIP_PRESENT (RFC-082 §1's one deliberate exception to
    // the fail-closed default) so an already-suspended member remains
    // removable, while its actor check stays ACTIVE.
    let remove_start = MEMBERSHIP_DB_SRC
        .find("pub async fn soft_remove_guarded_required")
        .expect("soft_remove_guarded_required should exist");
    let remove_fn = &MEMBERSHIP_DB_SRC[remove_start..];
    let remove_fn = &remove_fn[..remove_fn
        .find("\n}\n")
        .map(|offset| offset + 3)
        .unwrap_or(remove_fn.len())];
    assert!(
        remove_fn.contains("MEMBERSHIP_PRESENT"),
        "soft_remove_guarded_required's target check must be MEMBERSHIP_PRESENT, not \
         MEMBERSHIP_ACTIVE — RFC-082 §1 requires suspended -> removed to remain a valid \
         transition"
    );
    assert!(
        remove_fn.contains("MEMBERSHIP_ACTIVE"),
        "soft_remove_guarded_required's actor check must stay MEMBERSHIP_ACTIVE — a suspended \
         admin must not be able to remove anyone"
    );

    // removed -> anything: no function anywhere in this file clears
    // removed_at or otherwise treats it as reversible — asserted by
    // absence, matching rfc063_removal_only_policy_is_locked's own
    // "reactivate"/"restore" lock.
    assert!(
        !MEMBERSHIP_DB_SRC.contains("SET removed_at = NULL")
            && !MEMBERSHIP_DB_SRC.to_ascii_lowercase().contains("unremove")
            && !MEMBERSHIP_DB_SRC
                .to_ascii_lowercase()
                .contains("reactivate"),
        "removed must stay terminal — no function may clear removed_at (RFC-082 §1: \
         removed -> anything is refused for everyone, no exception)"
    );
}

/// RFC-082 §6: the two Class A audit actions exist, are wired into the
/// closed model the same way every other membership-lifecycle action is,
/// and record no more than the standard (community, actor, target) triple.
#[test]
fn rfc082_audit_actions_are_wired() {
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("MembershipSuspended,")
            && RFC079_AUDIT_CORE_SRC.contains("MembershipUnsuspended,")
            && RFC079_AUDIT_CORE_SRC.contains("\"membership.suspended\"")
            && RFC079_AUDIT_CORE_SRC.contains("\"membership.unsuspended\""),
        "MembershipSuspended/MembershipUnsuspended must exist with the expected canonical names"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("pub(crate) const ALL: [Self; 37]"),
        "AuditAction::ALL must be re-pinned to 37 after adding the two RFC-082 actions (was 35)"
    );
    let all_start = RFC079_AUDIT_CORE_SRC
        .find("pub(crate) const ALL")
        .expect("ALL should exist");
    let all_end = RFC079_AUDIT_CORE_SRC[all_start..]
        .find("];")
        .map(|offset| all_start + offset)
        .expect("ALL array should close");
    let all_array = &RFC079_AUDIT_CORE_SRC[all_start..all_end];
    assert!(
        all_array.contains("Self::MembershipSuspended")
            && all_array.contains("Self::MembershipUnsuspended"),
        "both new actions must be listed in AuditAction::ALL, not only defined on the enum"
    );
}

/// RFC-082 §4 / Handoff 058: the "access is paused" mechanism — a
/// sentinel error parallel to `authz::not_found`, caught by `lib.rs`'s
/// top-level dispatch the same way, rendered by a dedicated
/// `render::suspended()` — chosen specifically so
/// `require_membership`'s signature and every one of its call sites stay
/// unchanged.
#[test]
fn rfc082_paused_page_mechanism_is_wired() {
    assert!(
        AUTHZ_SRC.contains("pub(crate) fn suspended() -> worker::Error")
            && AUTHZ_SRC.contains("\"Suspended.\""),
        "authz.rs must define a suspended() sentinel parallel to not_found()"
    );
    assert!(
        AUTHZ_SRC.contains("membership_db::exists_present(&db, &auth.user_id, community_id)")
            && AUTHZ_SRC.contains("return Err(suspended())"),
        "require_membership must distinguish a present-but-suspended membership from a \
         genuinely absent one and return the suspended() sentinel for the former"
    );
    assert!(
        LIB_SRC.contains("fn is_suspended_error")
            && LIB_SRC.contains("\"Suspended.\"")
            && LIB_SRC.contains("if is_suspended_error(&error)")
            && LIB_SRC.contains("render::suspended()"),
        "lib.rs's top-level dispatch must catch the suspended() sentinel the same way it \
         catches not_found(), rendering render::suspended() instead of a generic 500"
    );
    assert!(
        RENDER_SRC.contains("pub fn suspended() -> Result<Response>")
            && RENDER_SRC.contains("i18n::JA_MEMBERSHIP_SUSPENDED")
            && RENDER_SRC.contains(".with_status(403)"),
        "render::suspended() must exist, use the explicit paused-access copy, and return 403"
    );
    assert!(
        !RENDER_SRC.contains("suspended() -> Result<Response> {\n    let body = format!(\n        \"<main class=\\\"cz-anon-main\\\">\\\n         <p>{}</p>{}</main>\",\n        i18n::JA_MEMBERSHIP_SUSPENDED,\n        recovery_links()"),
        "the paused page must not reuse recovery_links() (which offers /join) — a suspended \
         member is already a member, and /join is not their path back"
    );
}

/// RFC-082 §5 / §8.6: the admin surface. A suspended member must appear in
/// the member list (via the PRESENT-based listing, not the ACTIVE one),
/// marked suspended, with an unsuspend action; self-targeting is denied in
/// the handler the same way promote/demote/remove already deny it.
#[test]
fn rfc082_suspension_handlers_are_registered_and_self_target_denied() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("\"suspend\" => {")
            && COMMUNITY_HANDLER_SRC.contains("super::admin::get_suspend_member")
            && COMMUNITY_HANDLER_SRC.contains("super::admin::post_suspend_member")
            && COMMUNITY_HANDLER_SRC.contains("\"unsuspend\" => {")
            && COMMUNITY_HANDLER_SRC.contains("super::admin::get_unsuspend_member")
            && COMMUNITY_HANDLER_SRC.contains("super::admin::post_unsuspend_member"),
        "suspend/unsuspend GET and POST routes must be registered under /c/:cid/admin/members/:mid/"
    );
    assert!(
        SUSPENSION_HANDLER_SRC.contains("token_purpose::SUSPEND_MEMBER")
            && SUSPENSION_HANDLER_SRC.contains("token_purpose::UNSUSPEND_MEMBER")
            && SUSPENSION_HANDLER_SRC.contains("target_membership_id == membership.membership_id"),
        "suspension handlers must use dedicated token purposes and deny self-targeting \
         server-side, matching role_transfer.rs's own discipline"
    );
    assert!(
        SUSPENSION_HANDLER_SRC.contains("membership_db::find_present_summary")
            && SUSPENSION_HANDLER_SRC.contains("membership_db::suspend_required")
            && SUSPENSION_HANDLER_SRC.contains("membership_db::unsuspend_required"),
        "suspension handlers must target present (not only active) memberships and call the \
         guarded RFC-082 mutations"
    );
    assert!(
        MEMBERS_HANDLER_SRC.contains("membership_db::list_present_for_admin")
            && MEMBERS_HANDLER_SRC.contains("i18n::ADMIN_SUSPENDED_BADGE") // RFC-072/Handoff 072 locale-aware accessor
            && MEMBERS_HANDLER_SRC.contains("/suspend\\\"")
            && MEMBERS_HANDLER_SRC.contains("/unsuspend\\\""),
        "the admin member list must use the PRESENT-based listing (RFC-082 §5), render the \
         suspended badge, and link both the suspend and unsuspend actions"
    );
    assert!(
        MEMBER_REMOVE_HANDLER_SRC.contains("membership_db::find_present_summary"),
        "member_remove.rs's confirmation page must target present (not only active) \
         memberships — RFC-082 §1 requires suspended -> removed to remain reachable"
    );
}
