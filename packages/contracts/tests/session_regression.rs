//! Regression tests for the session-TTL-decoupling rule (RFC-003 §8).

use zinnias_ciao_contracts::{FORM_TOKEN_TTL_SECONDS, SESSION_TTL_SECONDS};

#[test]
fn session_ttl_is_constant_and_positive() {
    assert!(
        SESSION_TTL_SECONDS > 0,
        "SESSION_TTL_SECONDS must be positive; Max-Age=0 discards the cookie"
    );
    assert!(SESSION_TTL_SECONDS >= 3_600);
}

#[test]
fn form_token_ttl_is_shorter_than_session_ttl() {
    assert!(FORM_TOKEN_TTL_SECONDS < SESSION_TTL_SECONDS);
}

#[test]
fn session_ttl_simulated_leeway_edge() {
    // Documents the original bug: token exp - now can be <=0 at the leeway edge.
    let token_exp_unix: i64 = 1_000_000_000;
    let now_unix: i64 = 1_000_000_055; // 55s past exp, within 60s leeway
    let derived_ttl = token_exp_unix - now_unix;
    assert!(
        derived_ttl <= 0,
        "derived_ttl={derived_ttl} — demonstrates the bug"
    );
    let correct_ttl = SESSION_TTL_SECONDS as i64;
    assert!(correct_ttl > 0);
}
