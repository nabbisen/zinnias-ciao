//! Release-gate checks (RFC-015).
//! Every item here maps to a row in the MVP release-gate matrix.

#![allow(clippy::assertions_on_constants)]

use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::{AppError, FORM_TOKEN_TTL_SECONDS, SESSION_TTL_SECONDS};

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
        token_purpose::CREATE_COMMUNITY,
    ];
    for p in required {
        assert!(!p.is_empty(), "token purpose must not be empty: {p}");
        assert!(
            !p.contains(' '),
            "token purpose must not contain spaces: {p}"
        );
    }
}

// ── i18n parity gate ──────────────────────────────────────────────────────
// Every EN_* constant must have a non-empty JA_* counterpart.
// This test registers every member-facing string pair so a JA string going
// empty or missing causes `cargo test` to fail immediately.
// To add a new string: add EN_FOO and JA_FOO in i18n.rs, then add the pair below.

#[test]
fn i18n_en_ja_parity_count() {
    use zinnias_ciao_contracts::i18n::*;
    let pairs = [
        (EN_JOIN_HEADING, JA_JOIN_HEADING),
        (EN_JOIN_SUBHEADING, JA_JOIN_SUBHEADING),
        (EN_JOIN_CODE_LABEL, JA_JOIN_CODE_LABEL),
        (EN_JOIN_CODE_HINT, JA_JOIN_CODE_HINT),
        (EN_JOIN_SUBMIT, JA_JOIN_SUBMIT),
        (EN_JOIN_PROFILE_HEADING, JA_JOIN_PROFILE_HEADING),
        (EN_JOIN_PROFILE_HINT, JA_JOIN_PROFILE_HINT),
        (EN_JOIN_PROFILE_LABEL, JA_JOIN_PROFILE_LABEL),
        (EN_JOIN_PROFILE_SUBMIT, JA_JOIN_PROFILE_SUBMIT),
        (EN_STATUS_GOING, JA_STATUS_GOING),
        (EN_STATUS_NOT_GOING, JA_STATUS_NOT_GOING),
        (EN_STATUS_ATTENDED, JA_STATUS_ATTENDED),
        (EN_STATUS_NO_ANSWER, JA_STATUS_NO_ANSWER),
        (EN_STATUS_ATTENDED_DISABLED, JA_STATUS_ATTENDED_DISABLED),
        (EN_NOTE_SAVE, JA_NOTE_SAVE),
        (EN_NOTE_DELETE, JA_NOTE_DELETE),
        (EN_NOTE_SAVED, JA_NOTE_SAVED),
        (EN_NOTE_TOO_LONG, JA_NOTE_TOO_LONG),
        (EN_SESSION_EXPIRED, JA_SESSION_EXPIRED),
        (EN_LOGOUT, JA_LOGOUT),
        (EN_LOGOUT_CONFIRM, JA_LOGOUT_CONFIRM),
        (EN_GENERAL_ERROR, JA_GENERAL_ERROR),
        (EN_OFFLINE_BANNER, JA_OFFLINE_BANNER),
        (EN_EMPTY_EVENTS, JA_EMPTY_EVENTS),
        (EN_EMPTY_EVENTS_HINT, JA_EMPTY_EVENTS_HINT),
        (EN_EMPTY_EVENTS_ADMIN, JA_EMPTY_EVENTS_ADMIN),
        (EN_NAV_HOME, JA_NAV_HOME),
        (EN_NAV_COMMUNITIES, JA_NAV_COMMUNITIES),
        (EN_NAV_ME, JA_NAV_ME),
        (EN_HOME_TODAY, JA_HOME_TODAY),
        (EN_HOME_THIS_WEEK, JA_HOME_THIS_WEEK),
        (EN_HOME_LATER, JA_HOME_LATER),
        (EN_HOME_CREATE_EVENT, JA_HOME_CREATE_EVENT),
        (EN_HOME_INVITE_MEMBERS, JA_HOME_INVITE_MEMBERS),
        (EN_HOME_CALENDAR_TITLE, JA_HOME_CALENDAR_TITLE),
        (EN_HOME_CALENDAR_HELPER, JA_HOME_CALENDAR_HELPER),
        (EN_HOME_CALENDAR_EMPTY, JA_HOME_CALENDAR_EMPTY),
        (EN_HOME_CALENDAR_COUNT_SUFFIX, JA_HOME_CALENDAR_COUNT_SUFFIX),
        (EN_HOME_AGENDA_TITLE, JA_HOME_AGENDA_TITLE),
        (EN_CALENDAR_MONTH_TITLE, JA_CALENDAR_MONTH_TITLE),
        (EN_CALENDAR_PREV_MONTH, JA_CALENDAR_PREV_MONTH),
        (EN_CALENDAR_NEXT_MONTH, JA_CALENDAR_NEXT_MONTH),
        (EN_CALENDAR_THIS_MONTH, JA_CALENDAR_THIS_MONTH),
        (EN_CALENDAR_ALL_DAYS, JA_CALENDAR_ALL_DAYS),
        (EN_CALENDAR_EMPTY_MONTH, JA_CALENDAR_EMPTY_MONTH),
        (EN_CALENDAR_EMPTY_DAY, JA_CALENDAR_EMPTY_DAY),
        (EN_CALENDAR_CREATE_ON_DAY, JA_CALENDAR_CREATE_ON_DAY),
        (EN_STATUS_CLEAR, JA_STATUS_CLEAR),
        (EN_STATUS_CLEAR_LABEL, JA_STATUS_CLEAR_LABEL),
        (EN_NOTE_SECTION_LABEL, JA_NOTE_SECTION_LABEL),
        (EN_NOTE_PLACEHOLDER_LABEL, JA_NOTE_PLACEHOLDER_LABEL),
        (EN_NOTE_CHAR_HINT, JA_NOTE_CHAR_HINT),
        (EN_NOTE_VISIBILITY, JA_NOTE_VISIBILITY),
        (EN_ME_SECTION_NAME, JA_ME_SECTION_NAME),
        (EN_ME_SECTION_COMMUNITY, JA_ME_SECTION_COMMUNITY),
        (EN_ME_SECTION_HELP, JA_ME_SECTION_HELP),
        (EN_ME_HELP_BODY, JA_ME_HELP_BODY),
        (EN_ADMIN_CREATE_EVENT_TITLE, JA_ADMIN_CREATE_EVENT_TITLE),
        (EN_ADMIN_CREATE_EVENT_SUBMIT, JA_ADMIN_CREATE_EVENT_SUBMIT),
        (EN_ADMIN_EDIT_EVENT_TITLE, JA_ADMIN_EDIT_EVENT_TITLE),
        (EN_ADMIN_EDIT_EVENT_SUBMIT, JA_ADMIN_EDIT_EVENT_SUBMIT),
        (EN_ADMIN_EDIT_EVENT_HINT, JA_ADMIN_EDIT_EVENT_HINT),
        (EN_ADMIN_CANCEL_EVENT_TITLE, JA_ADMIN_CANCEL_EVENT_TITLE),
        (EN_ADMIN_CANCEL_EVENT_BODY, JA_ADMIN_CANCEL_EVENT_BODY),
        (EN_ADMIN_CANCEL_EVENT_KEEP, JA_ADMIN_CANCEL_EVENT_KEEP),
        (EN_ADMIN_CANCEL_EVENT_CONFIRM, JA_ADMIN_CANCEL_EVENT_CONFIRM),
        (
            EN_ADMIN_CANNOT_EDIT_CANCELLED,
            JA_ADMIN_CANNOT_EDIT_CANCELLED,
        ),
        (EN_ADMIN_CANNOT_EDIT_STARTED, JA_ADMIN_CANNOT_EDIT_STARTED),
        (
            EN_ADMIN_CANNOT_ATTEND_CANCELLED,
            JA_ADMIN_CANNOT_ATTEND_CANCELLED,
        ),
        (EN_ADMIN_ATTEND_TITLE, JA_ADMIN_ATTEND_TITLE),
        (EN_ADMIN_ATTEND_SUBMIT, JA_ADMIN_ATTEND_SUBMIT),
        (EN_ADMIN_INVITES_TITLE, JA_ADMIN_INVITES_TITLE),
        (EN_ADMIN_INVITES_BODY, JA_ADMIN_INVITES_BODY),
        (EN_ADMIN_INVITES_GENERATE, JA_ADMIN_INVITES_GENERATE),
        (EN_ADMIN_INVITES_ACTIVE, JA_ADMIN_INVITES_ACTIVE),
        (EN_ADMIN_INVITES_NONE, JA_ADMIN_INVITES_NONE),
        (
            EN_ADMIN_INVITES_NEW_CODE_HINT,
            JA_ADMIN_INVITES_NEW_CODE_HINT,
        ),
        (EN_ADMIN_INVITES_REVOKE, JA_ADMIN_INVITES_REVOKE),
        (EN_ADMIN_INVITES_REVOKED, JA_ADMIN_INVITES_REVOKED),
        (EN_ADMIN_MEMBERS_TITLE, JA_ADMIN_MEMBERS_TITLE),
        (
            EN_ADMIN_MEMBERS_GENERATE_INVITE,
            JA_ADMIN_MEMBERS_GENERATE_INVITE,
        ),
        (EN_ADMIN_REMOVE_TITLE, JA_ADMIN_REMOVE_TITLE),
        (EN_ADMIN_REMOVE_KEEP, JA_ADMIN_REMOVE_KEEP),
        (EN_ADMIN_REMOVE_CONFIRM, JA_ADMIN_REMOVE_CONFIRM),
        (EN_ADMIN_REMOVE_CONSEQUENCE, JA_ADMIN_REMOVE_CONSEQUENCE),
        (EN_ADMIN_LAST_ADMIN, JA_ADMIN_LAST_ADMIN),
        (EN_COMMUNITIES_JOIN_ANOTHER, JA_COMMUNITIES_JOIN_ANOTHER),
        (EN_COMMUNITY_CREATE_LINK, JA_COMMUNITY_CREATE_LINK),
        (EN_COMMUNITY_CREATE_TITLE, JA_COMMUNITY_CREATE_TITLE),
        (EN_COMMUNITY_CREATE_BODY, JA_COMMUNITY_CREATE_BODY),
        (
            EN_COMMUNITY_CREATE_NAME_LABEL,
            JA_COMMUNITY_CREATE_NAME_LABEL,
        ),
        (
            EN_COMMUNITY_CREATE_DISPLAY_NAME_LABEL,
            JA_COMMUNITY_CREATE_DISPLAY_NAME_LABEL,
        ),
        (
            EN_COMMUNITY_CREATE_TIMEZONE_LABEL,
            JA_COMMUNITY_CREATE_TIMEZONE_LABEL,
        ),
        (
            EN_COMMUNITY_CREATE_TIMEZONE_JAPAN,
            JA_COMMUNITY_CREATE_TIMEZONE_JAPAN,
        ),
        (EN_COMMUNITY_CREATE_SUBMIT, JA_COMMUNITY_CREATE_SUBMIT),
        (EN_COMMUNITY_CREATE_CANCEL, JA_COMMUNITY_CREATE_CANCEL),
        (EN_COMMUNITY_CREATE_DISABLED, JA_COMMUNITY_CREATE_DISABLED),
        (
            EN_COMMUNITY_CREATE_RATE_LIMITED,
            JA_COMMUNITY_CREATE_RATE_LIMITED,
        ),
        (
            EN_COMMUNITY_CREATE_NAME_ERROR,
            JA_COMMUNITY_CREATE_NAME_ERROR,
        ),
        (
            EN_COMMUNITY_CREATE_NAME_TOO_LONG,
            JA_COMMUNITY_CREATE_NAME_TOO_LONG,
        ),
        (
            EN_COMMUNITY_CREATE_NAME_INVALID,
            JA_COMMUNITY_CREATE_NAME_INVALID,
        ),
        (
            EN_COMMUNITY_CREATE_DISPLAY_NAME_ERROR,
            JA_COMMUNITY_CREATE_DISPLAY_NAME_ERROR,
        ),
        (
            EN_COMMUNITY_CREATE_TIMEZONE_ERROR,
            JA_COMMUNITY_CREATE_TIMEZONE_ERROR,
        ),
        (EN_ROLE_ADMIN, JA_ROLE_ADMIN),
        (EN_ROLE_MEMBER, JA_ROLE_MEMBER),
        (EN_HOME_FIRST_RUN_WELCOME, JA_HOME_FIRST_RUN_WELCOME),
        (EN_HOME_FIRST_RUN_NO_EVENTS, JA_HOME_FIRST_RUN_NO_EVENTS),
        (EN_HOME_FIRST_RUN_CREATE, JA_HOME_FIRST_RUN_CREATE),
        (EN_HOME_FIRST_RUN_INVITE_HINT, JA_HOME_FIRST_RUN_INVITE_HINT),
        (EN_REPEAT_LABEL, JA_REPEAT_LABEL),
        (EN_REPEAT_NONE, JA_REPEAT_NONE),
        (EN_REPEAT_WEEKLY, JA_REPEAT_WEEKLY),
        (EN_REPEAT_BIWEEKLY, JA_REPEAT_BIWEEKLY),
        (EN_REPEAT_MONTHLY, JA_REPEAT_MONTHLY),
        (EN_REPEAT_COUNT_UNIT, JA_REPEAT_COUNT_UNIT),
        (EN_REPEAT_COUNT_HINT, JA_REPEAT_COUNT_HINT),
        (EN_TEMPLATES_TITLE, JA_TEMPLATES_TITLE),
        (EN_TEMPLATES_DESCRIPTION, JA_TEMPLATES_DESCRIPTION),
        (EN_TEMPLATES_EMPTY, JA_TEMPLATES_EMPTY),
        (EN_TEMPLATES_SAVE_SECTION, JA_TEMPLATES_SAVE_SECTION),
        (EN_TEMPLATES_TITLE_LABEL, JA_TEMPLATES_TITLE_LABEL),
        (EN_TEMPLATES_LOC_LABEL, JA_TEMPLATES_LOC_LABEL),
        (EN_TEMPLATES_DUR_LABEL, JA_TEMPLATES_DUR_LABEL),
        (EN_TEMPLATES_SAVE_BTN, JA_TEMPLATES_SAVE_BTN),
        (EN_TEMPLATES_USE_BTN, JA_TEMPLATES_USE_BTN),
        (EN_TEMPLATES_DELETE_BTN, JA_TEMPLATES_DELETE_BTN),
        (EN_TEMPLATES_USE_LINK, JA_TEMPLATES_USE_LINK),
        (EN_EXPORT_TITLE, JA_EXPORT_TITLE),
        (EN_EXPORT_DESCRIPTION, JA_EXPORT_DESCRIPTION),
        (EN_EXPORT_PRIVACY_NOTE, JA_EXPORT_PRIVACY_NOTE),
        (EN_EXPORT_DOWNLOAD_BTN, JA_EXPORT_DOWNLOAD_BTN),
        (EN_EXPORT_SINGLE_USE, JA_EXPORT_SINGLE_USE),
        (EN_ME_SECTION_ABOUT, JA_ME_SECTION_ABOUT),
        (EN_ME_VERSION_LABEL, JA_ME_VERSION_LABEL),
        (EN_ME_REF_LABEL, JA_ME_REF_LABEL),
        (EN_ME_SECTION_DATA, JA_ME_SECTION_DATA),
        (EN_ME_EXPORT_LINK, JA_ME_EXPORT_LINK),
        (EN_CALENDAR_TITLE, JA_CALENDAR_TITLE),
        (EN_CALENDAR_DESCRIPTION, JA_CALENDAR_DESCRIPTION),
        (EN_CALENDAR_GENERATE, JA_CALENDAR_GENERATE),
        (EN_CALENDAR_DISABLE, JA_CALENDAR_DISABLE),
        (EN_CALENDAR_REGENERATE, JA_CALENDAR_REGENERATE),
        (EN_CALENDAR_PRIVACY_NOTE, JA_CALENDAR_PRIVACY_NOTE),
        (EN_EVENT_TITLE_HEADER, JA_EVENT_TITLE_HEADER),
        (EN_EVENT_ATTENDED_UNAVAILABLE, JA_EVENT_ATTENDED_UNAVAILABLE),
        (EN_EVENT_ATTENDED_ADMIN_ONLY, JA_EVENT_ATTENDED_ADMIN_ONLY),
        (EN_EVENT_MEMBER_FALLBACK, JA_EVENT_MEMBER_FALLBACK),
        (EN_JOIN_PAGE_TITLE, JA_JOIN_PAGE_TITLE),
        (EN_JOIN_PROFILE_PAGE_TITLE, JA_JOIN_PROFILE_PAGE_TITLE),
        // Added in v0.33.x — EN→JA inline string sweep
        (EN_NOT_FOUND, JA_NOT_FOUND),
        (EN_INTERNAL_ERROR, JA_INTERNAL_ERROR),
        (EN_ADMIN_ATTEND_CANCELLED, JA_ADMIN_ATTEND_CANCELLED),
        (EN_GENERAL_BACK, JA_GENERAL_BACK),
        (EN_ADMIN_EDIT_CANCELLED, JA_ADMIN_EDIT_CANCELLED),
        (EN_ADMIN_EDIT_STARTED, JA_ADMIN_EDIT_STARTED),
        (EN_NAV_BACK, JA_NAV_BACK),
        (EN_NAV_SWITCH_GO, JA_NAV_SWITCH_GO),
        (EN_NOTE_DELETE_BODY, JA_NOTE_DELETE_BODY),
        (EN_NOTE_KEEP_ACTION, JA_NOTE_KEEP_ACTION),
        (EN_FORM_FIELD_TITLE, JA_FORM_FIELD_TITLE),
        (EN_FORM_FIELD_DATE, JA_FORM_FIELD_DATE),
        (EN_FORM_FIELD_START, JA_FORM_FIELD_START),
        (EN_FORM_FIELD_END, JA_FORM_FIELD_END),
        (EN_FORM_FIELD_LOCATION, JA_FORM_FIELD_LOCATION),
        (EN_FORM_FIELD_DESC, JA_FORM_FIELD_DESC),
        (EN_EVENT_CANCELLED_BADGE, JA_EVENT_CANCELLED_BADGE),
        (EN_EVENT_WHOS_GOING, JA_EVENT_WHOS_GOING),
        (EN_EVENT_NOTES_SECTION, JA_EVENT_NOTES_SECTION),
        (EN_TZ_ERROR, JA_TZ_ERROR),
        (EN_CURRENT_BADGE, JA_CURRENT_BADGE),
        (EN_ME_CALENDAR_LABEL, JA_ME_CALENDAR_LABEL),
        (EN_ME_DATA_EXPORT, JA_ME_DATA_EXPORT),
    ];
    // Strings that are intentionally identical across languages (product name,
    // numeric units, etc.) are exempted from the identity check.
    const INTENTIONALLY_IDENTICAL: &[&str] = &["ciao.zinnias"];

    for (en, ja) in pairs {
        assert!(!en.is_empty(), "EN string empty");
        assert!(!ja.is_empty(), "JA string empty for EN: {en}");
        if !INTENTIONALLY_IDENTICAL.contains(&en) {
            assert_ne!(en, ja, "EN and JA are identical (likely copy-paste): {en}");
        }
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
const COMMUNITY_CREATE_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/community_create.rs");
const ME_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/me.rs");
const LIB_SRC: &str = include_str!("../../../workers/ssr/src/lib.rs");
const AUTHZ_SRC: &str = include_str!("../../../workers/ssr/src/authz.rs");
const RATE_LIMIT_SRC: &str = include_str!("../../../workers/ssr/src/rate_limit.rs");
const COMMUNITY_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/community.rs");
const WRANGLER_TOML_SRC: &str = include_str!("../../../wrangler.toml");

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
                    found = trimmed
                        .split_once('=')
                        .map(|(_, v)| v.trim().trim_matches('"').to_owned());
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

// ── Japanese-only rendered-text gate (RFC-049) ───────────────────────────
//
// The pilot ships Japanese UI only. English words leaked into rendered link
// and button text twice in v0.35.x (event-detail "← Home", communities
// "Invite members" / "Manage members"). These were inline literals, not i18n
// constants, so the i18n parity gate did not catch them.
//
// This gate scans the handler/render sources for the specific regressions that
// occurred and a few obvious English UI words appearing as element text. It is
// deliberately narrow: it matches ">Word</a>" or ">Word</button>" shapes with a
// known English UI vocabulary, not arbitrary English (comments, code, ARIA
// values, and HTTP header literals must remain unflagged).

const COMMUNITIES_SRC: &str = include_str!("../../../workers/ssr/src/handlers/communities.rs");
const COMMUNITY_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/community.rs");
const ADMIN_EVENTS_SRC: &str = include_str!("../../../workers/ssr/src/handlers/admin/events.rs");
const APP_JS_SRC: &str = include_str!("../../../workers/ssr/static/app.js");
const RENDER_SRC: &str = include_str!("../../../workers/ssr/src/render.rs");
const STATIC_FILES_SRC: &str = include_str!("../../../workers/ssr/src/handlers/static_files.rs");

#[test]
fn no_known_english_ui_leaks_in_rendered_text() {
    // Exact regressions that previously shipped — keep them from returning.
    let forbidden: &[&str] = &[
        ">Invite members<",
        ">Manage members<",
        "\u{2190} Home<", // "← Home" — must be "← ホーム"
        ">Home</a>",
        ">Members</a>",
        ">Go</button>", // bare English fallback button (use JA)
    ];
    for src in [
        EVENT_HANDLER_SRC,
        COMMUNITIES_SRC,
        RENDER_SRC,
        HOME_HANDLER_SRC,
        COMMUNITY_CREATE_HANDLER_SRC,
    ] {
        for needle in forbidden {
            assert!(
                !src.contains(needle),
                "English UI text leaked into rendered output: {needle:?}. \
                 Pilot is Japanese-only (RFC-049) — use a JA_* i18n constant."
            );
        }
    }
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
            && COMMUNITY_CREATE_HANDLER_SRC.contains("if let Some(community_id) = replay"),
        "Community creation must use scoped form tokens and replay to the created community"
    );
    assert!(
        RATE_LIMIT_SRC.contains("community_create_user")
            && RATE_LIMIT_SRC.contains("community_create_session")
            && RATE_LIMIT_SRC.contains("community_create_ip")
            && RATE_LIMIT_SRC.contains("COMMUNITY_CREATION_MAX_PER_WINDOW"),
        "Community creation must be rate-limited by user, session, and IP"
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
            && COMMUNITY_DB_SRC.contains("INSERT INTO audit_log")
            && COMMUNITY_DB_SRC.contains("db.batch"),
        "Community creation must batch community, first-admin membership, and audit writes"
    );
    assert!(
        COMMUNITY_DB_SRC.contains("community.created")
            && COMMUNITY_DB_SRC.contains("membership.created_first_admin"),
        "Community creation must emit the reviewed audit events"
    );
    assert!(
        COMMUNITY_DB_SRC.contains("metadata_json")
            && !COMMUNITY_DB_SRC.contains("action, metadata, created_at"),
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
        ME_HANDLER_SRC.contains("JA_COMMUNITY_CREATE_LINK")
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
        HOME_HANDLER_SRC.contains("render::header(i18n::JA_NAV_HOME"),
        "Home must use a simple header without the community switcher"
    );
    assert!(
        !HOME_HANDLER_SRC.contains("header_with_switcher(i18n::JA_NAV_HOME"),
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
            && COMMUNITIES_SRC.contains("JA_CALENDAR_PREV_MONTH")
            && COMMUNITIES_SRC.contains("JA_CALENDAR_NEXT_MONTH")
            && COMMUNITIES_SRC.contains("JA_CALENDAR_THIS_MONTH")
            && COMMUNITIES_SRC.contains("JA_CALENDAR_ALL_DAYS"),
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
        RENDER_SRC.contains("/static/app.js?v=0.43.0-calendar-create-day")
            && STATIC_FILES_SRC.contains("/static/app.js?v=0.43.0-calendar-create-day"),
        "HTML shell must cache-bust app.js so same-version switcher fixes are not hidden by the service worker"
    );
    assert!(
        RENDER_SRC.contains("<button type='submit'")
            && RENDER_SRC.contains("JA_NAV_SWITCH_GO")
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
        COMMUNITIES_SRC.contains("grid-template-columns:repeat(7,minmax(0,1fr))"),
        "Calendar overview must keep a stable seven-column grid"
    );
}

#[test]
fn calendar_overview_contract_is_explicit() {
    let calendar_src = COMMUNITIES_SRC
        .split("fn render_calendar_month")
        .nth(1)
        .expect("Calendar page must keep a dedicated calendar renderer");

    assert!(
        calendar_src.contains("JA_HOME_CALENDAR_HELPER"),
        "Calendar overview must include helper copy explaining that details are in the list below"
    );
    assert!(
        calendar_src.contains("今日"),
        "Today must be identified by visible text, not color alone"
    );
    assert!(
        calendar_src.contains('●'),
        "Event presence must use a visible marker, not color alone"
    );
    assert!(
        calendar_src.contains("<a href=")
            && calendar_src.contains("aria-current=\\\"date\\\"")
            && calendar_src.contains("JA_CALENDAR_ALL_DAYS"),
        "Calendar day cells are interactive in v0.42.0 and must expose selected-day state plus a clear filter"
    );
    assert!(
        !calendar_src.contains("is_selected || is_today")
            && calendar_src.contains("#FAFAFB")
            && calendar_src.contains("let border_width = if is_today && !is_selected")
            && calendar_src.contains("border:{border_width} solid {border}")
            && calendar_src.contains("#6E6E73"),
        "Today styling must stay calmer than selected-day styling and distinct from ordinary event days"
    );
}

#[test]
fn rfc059_calendar_create_from_day_is_route_backed() {
    assert!(
        COMMUNITIES_SRC.contains("membership_db::find_active")
            && COMMUNITIES_SRC.contains("membership.role == \"admin\"")
            && COMMUNITIES_SRC.contains("can_create_event"),
        "Calendar create-from-day action must be rendered only for active admins"
    );
    assert!(
        COMMUNITIES_SRC.contains("/admin/events/new?day={day}")
            && COMMUNITIES_SRC.contains("JA_CALENDAR_CREATE_ON_DAY"),
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
    let mark_used = JOIN_HANDLER_SRC
        .find("crate::db::invite::mark_used(&db, &invite_id)")
        .expect("join profile must atomically mark invite used");
    let insert_user = JOIN_HANDLER_SRC
        .find("membership_db::insert_user(&db, &user_id)")
        .expect("join profile must insert user");
    let insert_membership = JOIN_HANDLER_SRC
        .find("membership_db::insert_membership(")
        .expect("join profile must insert membership");
    let assign_used_membership = JOIN_HANDLER_SRC
        .find("crate::db::invite::assign_used_membership(&db, &invite_id, &membership_id)")
        .expect("join profile must backfill invite used_by_membership_id");

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
    let mark_start = INVITE_DB_SRC
        .find("pub async fn mark_used(")
        .expect("invite::mark_used must exist");
    let assign_start = INVITE_DB_SRC
        .find("pub async fn assign_used_membership(")
        .expect("invite::assign_used_membership must exist");
    let mark_body = &INVITE_DB_SRC[mark_start..assign_start];
    let assign_body = &INVITE_DB_SRC[assign_start..];

    assert!(
        mark_body.contains("SET used_at = ?1"),
        "mark_used should perform the atomic one-winner claim"
    );
    assert!(
        !mark_body.contains("used_by_membership_id"),
        "mark_used must not write used_by_membership_id before the membership FK target exists"
    );
    assert!(
        assign_body.contains("SET used_by_membership_id = ?1"),
        "assign_used_membership should perform the post-membership FK backfill"
    );
}
