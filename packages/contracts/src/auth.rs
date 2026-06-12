/// Server-side session lifetime in seconds (RFC-003 AD-2).
///
/// IMPORTANT: Cookie Max-Age is computed from this constant only.
/// It must NEVER be derived from an upstream token's `exp` field —
/// doing so caused a Max-Age=0 regression when tokens were at the
/// validation-leeway edge (browsers discard a Max-Age=0 cookie immediately).
pub const SESSION_TTL_SECONDS: u64 = 30 * 86_400; // 30 days

/// Lifetime of a server-issued single-use form token (AD-4).
/// Short enough to be safe; long enough to survive a slow mobile form fill.
pub const FORM_TOKEN_TTL_SECONDS: u64 = 3_600; // 1 hour

/// Cookie name. Changing this after deployment is a breaking change
/// (all existing sessions become unreadable).
pub const SESSION_COOKIE_NAME: &str = "ciao_sid";

/// Purpose strings for form tokens (AD-4 / RFC-002).
pub mod token_purpose {
    pub const SET_STATUS: &str = "set_status";
    pub const SAVE_NOTE: &str = "save_note";
    pub const DELETE_NOTE: &str = "delete_note";
    pub const CREATE_EVENT: &str = "create_event";
    pub const EDIT_EVENT: &str = "edit_event";
    pub const CANCEL_EVENT: &str = "cancel_event";
    pub const ATTENDANCE_OVERRIDE: &str = "attendance_override";
    pub const ADMIN_HIDE_NOTE: &str = "admin_hide_note";
    pub const REVOKE_INVITE: &str = "revoke_invite";
    pub const CALENDAR_REGENERATE: &str = "calendar_regenerate";
    pub const COMMUNITY_EXPORT: &str = "community_export";
    pub const CREATE_TEMPLATE: &str = "create_template";
    pub const DELETE_TEMPLATE: &str = "delete_template";
    pub const REMOVE_MEMBER: &str = "remove_member";
    pub const GENERATE_INVITE: &str = "generate_invite";
    pub const CALENDAR_REVOKE: &str = "calendar_revoke";
    pub const REDEEM_INVITE: &str = "redeem_invite";
    pub const JOIN_PROFILE: &str = "join_profile";
    pub const LOGOUT: &str = "logout";
}

/// Outcome of attempting to consume a single-use form token.
///
/// This encodes the decision the DB layer makes after an atomic conditional
/// UPDATE (`SET consumed_at WHERE ... AND consumed_at IS NULL`) and, when that
/// matches zero rows, a follow-up classification SELECT. Keeping the logic here
/// (pure, no Worker/D1 types) makes the idempotency/race contract unit-testable.
#[derive(Debug, PartialEq, Eq)]
pub enum TokenConsumeOutcome {
    /// This call won the race (UPDATE changed exactly one row). Proceed.
    Proceed,
    /// Token already consumed — idempotent replay. Return the prior result.
    Replay,
    /// Token not found, binding mismatch, or expired. Reject the request.
    Invalid,
}

/// Classify a consume attempt from the atomic-UPDATE affected-row count and the
/// follow-up row state. `changed` is the UPDATE's affected-row count. When
/// `changed == 0`, `found`/`already_consumed`/`binding_ok` describe the row (if
/// any) located by the classification SELECT.
pub fn classify_token_consume(
    changed: usize,
    found: bool,
    already_consumed: bool,
    binding_ok: bool,
) -> TokenConsumeOutcome {
    if changed == 1 {
        return TokenConsumeOutcome::Proceed;
    }
    // changed == 0: the conditional UPDATE matched nothing — classify why.
    if !found || !binding_ok {
        return TokenConsumeOutcome::Invalid;
    }
    if already_consumed {
        return TokenConsumeOutcome::Replay;
    }
    // Exists, unconsumed, binding ok, but UPDATE still missed → expired.
    TokenConsumeOutcome::Invalid
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard: session TTL must never be zero or near-zero.
    /// If this is set from an IdP token exp in future, this test must be
    /// updated to verify the decoupling explicitly (RFC-003 §8).
    #[test]
    fn session_ttl_is_positive_and_reasonable() {
        assert!(
            SESSION_TTL_SECONDS >= 3_600,
            "SESSION_TTL_SECONDS too short"
        );
        assert!(
            SESSION_TTL_SECONDS <= 31 * 86_400,
            "SESSION_TTL_SECONDS too long for invite-only MVP"
        );
    }

    #[test]
    fn form_token_ttl_shorter_than_session() {
        assert!(FORM_TOKEN_TTL_SECONDS < SESSION_TTL_SECONDS);
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
}
