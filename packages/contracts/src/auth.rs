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

/// Lifetime of an admin-issued active-member help-signin code (RFC-024).
/// This is intentionally short because the code is a bearer credential.
pub const RELINK_CODE_TTL_SECONDS: u64 = 15 * 60; // 15 minutes

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
    pub const CANCEL_OCCURRENCE: &str = "cancel_occurrence";
    pub const ATTENDANCE_OVERRIDE: &str = "attendance_override";
    pub const ADMIN_HIDE_NOTE: &str = "admin_hide_note";
    pub const REVOKE_INVITE: &str = "revoke_invite";
    pub const CALENDAR_REGENERATE: &str = "calendar_regenerate";
    pub const CALENDAR_MATRIX_CSV_EXPORT: &str = "calendar_matrix_csv_export";
    pub const COMMUNITY_EXPORT: &str = "community_export";
    pub const CREATE_TEMPLATE: &str = "create_template";
    pub const DELETE_TEMPLATE: &str = "delete_template";
    pub const REMOVE_MEMBER: &str = "remove_member";
    pub const PROMOTE_MEMBER: &str = "promote_member";
    pub const DEMOTE_MEMBER: &str = "demote_member";
    pub const HELP_SIGNIN: &str = "help_signin";
    pub const REDEEM_RELINK: &str = "redeem_relink";
    pub const GENERATE_INVITE: &str = "generate_invite";
    pub const CALENDAR_REVOKE: &str = "calendar_revoke";
    pub const REDEEM_INVITE: &str = "redeem_invite";
    pub const JOIN_PROFILE: &str = "join_profile";
    pub const LOGOUT: &str = "logout";
    pub const CREATE_COMMUNITY: &str = "create_community";
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
mod tests;
