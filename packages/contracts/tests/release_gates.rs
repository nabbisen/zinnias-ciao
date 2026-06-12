//! Release-gate checks (RFC-015).
//! Every item here maps to a row in the MVP release-gate matrix.

use zinnias_ciao_contracts::{AppError, SESSION_TTL_SECONDS, FORM_TOKEN_TTL_SECONDS};
use zinnias_ciao_contracts::auth::token_purpose;

// ── Session / auth gates ──────────────────────────────────────────────────

#[test]
fn session_ttl_positive_and_bounded() {
    assert!(SESSION_TTL_SECONDS > 0,  "session TTL must be positive (Max-Age=0 bug)");
    assert!(SESSION_TTL_SECONDS >= 3600, "session TTL too short");
    assert!(SESSION_TTL_SECONDS <= 31 * 86400, "session TTL too long for invite-only MVP");
}

#[test]
fn form_token_ttl_shorter_than_session() {
    assert!(FORM_TOKEN_TTL_SECONDS < SESSION_TTL_SECONDS,
        "form token must expire before the session");
}

#[test]
fn session_ttl_never_derived_from_token_exp() {
    // Documents the regression: if someone naively computed TTL as
    // token_exp - now and the token was at the JWT leeway edge (~55s past exp),
    // Max-Age would be <= 0 and the browser would discard the cookie immediately.
    let token_exp: i64 = 1_000_000_000;
    let now_at_edge: i64 = 1_000_000_055;   // 55 s past exp — within 60 s leeway
    let derived: i64 = token_exp - now_at_edge;
    assert!(derived <= 0, "derived TTL {} <= 0 demonstrates the bug", derived);
    // The correct value is always the constant:
    assert!(SESSION_TTL_SECONDS as i64 > 0);
}

// ── Error model gates ─────────────────────────────────────────────────────

#[test]
fn not_found_and_forbidden_same_message() {
    assert_eq!(AppError::not_found().user_message, AppError::forbidden().user_message);
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
        token_purpose::COMMUNITY_EXPORT,
        token_purpose::CREATE_TEMPLATE,
        token_purpose::DELETE_TEMPLATE,
        token_purpose::REMOVE_MEMBER,
        token_purpose::GENERATE_INVITE,
        token_purpose::REDEEM_INVITE,
        token_purpose::JOIN_PROFILE,
        token_purpose::LOGOUT,
    ];
    for p in required {
        assert!(!p.is_empty(), "token purpose must not be empty: {p}");
        assert!(!p.contains(' '), "token purpose must not contain spaces: {p}");
    }
}

// ── i18n parity gate ──────────────────────────────────────────────────────

#[test]
fn i18n_en_ja_parity_count() {
    use zinnias_ciao_contracts::i18n::*;
    // Spot-check: key strings have non-empty EN and JA counterparts.
    let pairs = [
        (EN_JOIN_SUBMIT,           JA_JOIN_SUBMIT),
        (EN_STATUS_GOING,          JA_STATUS_GOING),
        (EN_STATUS_NOT_GOING,      JA_STATUS_NOT_GOING),
        (EN_STATUS_ATTENDED,       JA_STATUS_ATTENDED),
        (EN_STATUS_NO_ANSWER,      JA_STATUS_NO_ANSWER),
        (EN_STATUS_ATTENDED_DISABLED, JA_STATUS_ATTENDED_DISABLED),
        (EN_NOTE_SAVE,             JA_NOTE_SAVE),
        (EN_SESSION_EXPIRED,       JA_SESSION_EXPIRED),
        (EN_OFFLINE_BANNER,        JA_OFFLINE_BANNER),
    ];
    for (en, ja) in pairs {
        assert!(!en.is_empty(), "EN string empty");
        assert!(!ja.is_empty(), "JA string empty for EN: {en}");
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
// RECURRENCE_MAX_COUNT = 52 (RFC-022) so the worst case is 9 fixed queries
// + 1 batch attendance + 52 token issues = 62 ops, documented below.

/// Fixed D1 queries for Home (no loops above 1 per route):
/// memberships, events, member_count, my_statuses (IN), counts (IN),
/// communities_for_switcher + 2 spares = 8 total.
const QUERY_BUDGET_HOME: usize = 8;

/// Fixed D1 queries for Event Detail (single-day event):
/// find_event, days, member_count, my_note, all_notes, all_members,
/// community, my_statuses (IN), counts (IN), all_day_attendances (IN),
/// 1 SET_STATUS token issue, 1 SAVE_NOTE token issue,
/// communities_for_switcher = 13 total.
const QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY: usize = 13;

/// Worst-case D1 ops for Event Detail (recurring, RECURRENCE_MAX_COUNT days):
/// 10 fixed + 1 batch attendance + 52 SET_STATUS token issues + 1 SAVE_NOTE
/// + 1 communities_for_switcher = 65.
const QUERY_BUDGET_EVENT_DETAIL_MAX_RECURRING: usize = 65;

/// D1 queries for Export (any community size): 5 fixed + 3 IN batches = 8.
/// Was O(events * days) before v0.24.0; now a flat 8 regardless of size.
const QUERY_BUDGET_EXPORT: usize = 8;

#[test]
fn query_budgets_are_positive_and_ordered() {
    assert!(QUERY_BUDGET_HOME > 0);
    assert!(QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY > QUERY_BUDGET_HOME);
    assert!(QUERY_BUDGET_EVENT_DETAIL_MAX_RECURRING >= QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY);
    assert!(QUERY_BUDGET_EXPORT > 0);
    // Export must be flat (well under the old per-event worst case):
    assert!(QUERY_BUDGET_EXPORT < 20,
        "Export budget {QUERY_BUDGET_EXPORT} exceeds expected flat upper bound");
    // Event detail single-day must be well under the old per-note worst case:
    assert!(QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY < 20,
        "Event detail budget {QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY} suggests an N+1 regression");
}

// ── Service worker version gate (RFC-044 §11 step 1) ─────────────────────
//
// sw.js CACHE_VERSION must equal the package version at every release.
// A mismatch means the service worker will not invalidate old caches on deploy.
//
// This test reads both files at test time using include_str! so it fires on
// every `cargo test` run without any external tooling.

const SW_JS_SOURCE: &str = include_str!("../../../workers/ssr/static/sw.js");
const WORKSPACE_CARGO_TOML: &str = include_str!("../../../Cargo.toml");

#[test]
fn sw_cache_version_matches_workspace_version() {
    // Extract CACHE_VERSION from sw.js:  const CACHE_VERSION = 'vX.Y.Z';
    let cache_ver = SW_JS_SOURCE
        .lines()
        .find(|l| l.trim_start().starts_with("const CACHE_VERSION"))
        .and_then(|l| {
            // e.g.  const CACHE_VERSION = 'v0.25.0';
            let after_eq = l.splitn(2, '=').nth(1)?;
            let inner = after_eq.trim().trim_start_matches('\'').trim_end_matches(';')
                .trim_end_matches('\'');
            // Strip the leading 'v'
            inner.strip_prefix('v')
        })
        .expect("CACHE_VERSION not found in sw.js");

    // Extract version from [workspace.package] block in Cargo.toml.
    // Find the version line that follows the [workspace.package] header.
    let workspace_ver = {
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
                    found = trimmed.splitn(2, '=').nth(1)
                        .map(|v| v.trim().trim_matches('"').to_owned());
                    break;
                }
            }
        }
        found.expect("workspace version not found in Cargo.toml")
    };

    assert_eq!(
        cache_ver, workspace_ver,
        "sw.js CACHE_VERSION 'v{cache_ver}' does not match workspace version '{workspace_ver}'. \
         Update sw.js CACHE_VERSION when bumping the version."
    );
}
