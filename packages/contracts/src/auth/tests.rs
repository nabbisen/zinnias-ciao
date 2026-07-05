use super::*;

/// Regression guard: session TTL must never be zero or near-zero.
/// If this is set from an IdP token exp in future, this test must be
/// updated to verify the decoupling explicitly (RFC-003 §8).
#[test]
fn session_ttl_is_positive_and_reasonable() {
    let session_ttl_seconds = std::hint::black_box(SESSION_TTL_SECONDS);
    assert!(
        session_ttl_seconds >= 3_600,
        "SESSION_TTL_SECONDS too short"
    );
    assert!(
        session_ttl_seconds <= 31 * 86_400,
        "SESSION_TTL_SECONDS too long for invite-only MVP"
    );
}

#[test]
fn form_token_ttl_shorter_than_session() {
    let form_token_ttl_seconds = std::hint::black_box(FORM_TOKEN_TTL_SECONDS);
    let session_ttl_seconds = std::hint::black_box(SESSION_TTL_SECONDS);
    assert!(form_token_ttl_seconds < session_ttl_seconds);
}

// ── Token consume classification (P0-7 race/idempotency contract) ──────

#[test]
fn consume_winner_proceeds() {
    // The call whose atomic UPDATE changed one row proceeds.
    assert_eq!(
        classify_token_consume(1, true, false, true),
        TokenConsumeOutcome::Proceed
    );
}

#[test]
fn consume_loser_of_race_sees_replay() {
    // Concurrent double-submit: the second call's UPDATE changes 0 rows
    // because consumed_at is now set. It must replay, not re-execute.
    assert_eq!(
        classify_token_consume(0, true, true, true),
        TokenConsumeOutcome::Replay
    );
}

#[test]
fn consume_unknown_token_is_invalid() {
    assert_eq!(
        classify_token_consume(0, false, false, false),
        TokenConsumeOutcome::Invalid
    );
}

#[test]
fn consume_binding_mismatch_is_invalid() {
    // Right token, wrong bound_resource → not a replay, a rejection.
    assert_eq!(
        classify_token_consume(0, true, false, false),
        TokenConsumeOutcome::Invalid
    );
}

#[test]
fn consume_expired_unconsumed_is_invalid() {
    // Found, unconsumed, binding ok, but UPDATE missed (expiry guard) → invalid.
    assert_eq!(
        classify_token_consume(0, true, false, true),
        TokenConsumeOutcome::Invalid
    );
}

#[test]
fn consume_never_double_proceeds() {
    // Exhaustive: across all 0-changed states, Proceed is impossible.
    for found in [false, true] {
        for consumed in [false, true] {
            for binding in [false, true] {
                assert_ne!(
                    classify_token_consume(0, found, consumed, binding),
                    TokenConsumeOutcome::Proceed,
                    "changed==0 must never proceed (found={found} consumed={consumed} binding={binding})"
                );
            }
        }
    }
}
