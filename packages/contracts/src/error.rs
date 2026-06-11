use thiserror::Error;

/// Internal error codes — never shown to users; used for logging and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Auth / session
    SessionMissing,
    SessionExpired,
    SessionRevoked,
    // Form token (AD-4)
    TokenMissing,
    TokenInvalid,
    TokenExpired,
    TokenConsumed,
    // Authorization
    NotFound,       // also used to avoid leaking resource existence (RFC-004)
    Forbidden,
    // Invite codes
    InviteInvalidOrExpired,
    InviteRateLimited,
    // Validation
    ValidationFailed,
    // Infra
    DatabaseError,
    InternalError,
}

/// Application error carrying both an internal code (for logs/tests) and
/// a user-visible plain-language message (RFC-012/013).
#[derive(Debug, Error)]
#[error("{user_message}")]
pub struct AppError {
    pub code: ErrorCode,
    /// Plain-language message safe to show in the UI.
    /// Must never contain SQL/stack/internal details.
    pub user_message: &'static str,
    pub retryable: bool,
}

impl AppError {
    pub fn session_expired() -> Self {
        Self {
            code: ErrorCode::SessionExpired,
            user_message: "Your session expired. Please ask your community admin for a new invite code.",
            retryable: false,
        }
    }

    pub fn not_found() -> Self {
        Self {
            code: ErrorCode::NotFound,
            user_message: "Not found.",
            retryable: false,
        }
    }

    pub fn forbidden() -> Self {
        Self {
            code: ErrorCode::Forbidden,
            user_message: "Not found.", // deliberately generic to avoid resource existence leak
            retryable: false,
        }
    }

    pub fn token_invalid() -> Self {
        Self {
            code: ErrorCode::TokenInvalid,
            user_message: "This action could not be completed. Please try again.",
            retryable: true,
        }
    }

    pub fn invite_invalid() -> Self {
        Self {
            code: ErrorCode::InviteInvalidOrExpired,
            user_message: "Invalid or expired code.",
            retryable: false,
        }
    }

    pub fn invite_rate_limited() -> Self {
        Self {
            code: ErrorCode::InviteRateLimited,
            user_message: "Please wait a little and try again.",
            retryable: true,
        }
    }

    pub fn validation(msg: &'static str) -> Self {
        Self {
            code: ErrorCode::ValidationFailed,
            user_message: msg,
            retryable: false,
        }
    }

    pub fn internal() -> Self {
        Self {
            code: ErrorCode::InternalError,
            user_message: "Something went wrong. Please try again.",
            retryable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_and_not_found_same_message() {
        // Must not leak resource existence (RFC-004 / RFC-012)
        assert_eq!(AppError::forbidden().user_message, AppError::not_found().user_message);
    }

    #[test]
    fn session_expired_message_is_plain_language() {
        let msg = AppError::session_expired().user_message;
        assert!(!msg.contains("JWT"));
        assert!(!msg.contains("token"));
        assert!(!msg.contains("cookie"));
        assert!(!msg.contains("401"));
    }
}
