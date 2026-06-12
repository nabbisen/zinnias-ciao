/// Server-side session lifetime in seconds (RFC-003 AD-2).
///
/// IMPORTANT: Cookie Max-Age is computed from this constant only.
/// It must NEVER be derived from an upstream token's `exp` field —
/// doing so caused a Max-Age=0 regression when tokens were at the
/// validation-leeway edge (browsers discard a Max-Age=0 cookie immediately).
pub const SESSION_TTL_SECONDS: u64 = 86_400; // 24 hours

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
    pub const CALENDAR_REVOKE: &str = "calendar_revoke";
    pub const REDEEM_INVITE: &str = "redeem_invite";
    pub const JOIN_PROFILE: &str = "join_profile";
    pub const LOGOUT: &str = "logout";
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
            SESSION_TTL_SECONDS <= 7 * 86_400,
            "SESSION_TTL_SECONDS too long for MVP"
        );
    }

    #[test]
    fn form_token_ttl_shorter_than_session() {
        assert!(FORM_TOKEN_TTL_SECONDS < SESSION_TTL_SECONDS);
    }
}
