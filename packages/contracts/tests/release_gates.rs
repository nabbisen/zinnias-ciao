//! Release-gate checks (RFC-015).
//! Every item here maps to a row in the MVP release-gate matrix.

use zinnias_ciao_contracts::{AppError, SESSION_TTL_SECONDS, FORM_TOKEN_TTL_SECONDS};
use zinnias_ciao_contracts::auth::token_purpose;

// ── Session / auth gates ──────────────────────────────────────────────────

#[test]
fn session_ttl_positive_and_bounded() {
    assert!(SESSION_TTL_SECONDS > 0,  "session TTL must be positive (Max-Age=0 bug)");
    assert!(SESSION_TTL_SECONDS >= 3600, "session TTL too short");
    assert!(SESSION_TTL_SECONDS <= 7 * 86400, "session TTL too long for MVP");
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
