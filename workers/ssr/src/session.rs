//! Session middleware — RFC-003.
//!
//! Extracts and validates the session cookie on every authenticated request.
//! Identity derives from the session row; never from client-supplied headers.

use worker::{Env, Request, Result};
#[cfg(not(target_arch = "wasm32"))]
use zinnias_ciao_contracts::SESSION_TTL_SECONDS;
use zinnias_ciao_contracts::{AppError, SESSION_COOKIE_NAME};

use crate::crypto::hmac_hex;
use crate::db::session as session_db;

/// The resolved session attached to a request context.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub session_id: String,
    pub user_id: String,
}

/// Extract the session cookie, hash it, look it up in D1.
/// Returns `Ok(AuthContext)` on success, `Err(AppError)` otherwise.
///
/// ## Parallel lookup — 30-day grace period (codlet Option A migration)
///
/// New sessions issued after codlet integration are stored in `codlet_sessions`
/// under codlet's domain-separated HMAC (`codlet/v1/lookup\0session\0value`).
/// Existing sessions (issued before the migration, up to 30 days old) live in
/// `sessions` under the legacy HMAC (`hmac_hex(pepper, value)`).
///
/// We try the codlet table first (fast path for all new sessions), then fall
/// back to the legacy table. Once `SELECT COUNT(*) FROM sessions WHERE
/// revoked_at IS NULL AND expires_at > unixepoch()` returns 0 (or after 30
/// days from the first codlet deploy), remove the fallback path.
pub async fn require_auth(req: &Request, env: &Env) -> Result<AuthContext> {
    let pepper = crate::crypto::pepper(env);

    let cookie_secret = extract_cookie(req, SESSION_COOKIE_NAME).ok_or_else(|| {
        worker::Error::RustError(AppError::session_expired().user_message.to_string())
    })?;

    let db = env.d1("DB")?;

    // ── Codlet path (new sessions) ────────────────────────────────────────
    // Try codlet_sessions first; this is the fast path for all sessions
    // issued after the migration. Fails gracefully if the table doesn't exist
    // yet (e.g. before migration 0007 runs) so existing sessions still work.
    #[cfg(target_arch = "wasm32")]
    if let Ok(Some(ctx)) = try_codlet_session(&cookie_secret, env).await {
        return Ok(ctx);
    }

    // ── Legacy path (pre-migration sessions) ──────────────────────────────
    // Falls back to the original sessions table. Remove after 30 days.
    let hmac = hmac_hex(&pepper, &cookie_secret);
    let session = session_db::find_active(&db, &hmac).await?.ok_or_else(|| {
        worker::Error::RustError(AppError::session_expired().user_message.to_string())
    })?;

    Ok(AuthContext {
        session_id: session.id,
        user_id: session.user_id,
    })
}

/// Try to validate a session against codlet_sessions (wasm32 only).
/// Returns `Ok(None)` for any miss or error so the legacy path runs.
///
/// Uses `codlet::build_session_mgr()` — a single shared construction path —
/// rather than duplicating the `SessionManager` setup here.
#[cfg(target_arch = "wasm32")]
async fn try_codlet_session(
    cookie_secret: &str,
    env: &worker::Env,
) -> worker::Result<Option<AuthContext>> {
    use codlet_core::state::SessionValidationOutcome;

    // build_session_mgr() returns Err if CODLET_HMAC_KEY_V1 is not yet set.
    let mgr = match crate::codlet::build_session_mgr(env) {
        Ok(m) => m,
        Err(_) => return Ok(None), // key not configured yet — fall through to legacy
    };

    match mgr.validate(cookie_secret).await {
        Ok(SessionValidationOutcome::Authenticated {
            subject,
            session_id,
            ..
        }) => Ok(Some(AuthContext {
            session_id: session_id.as_str().to_owned(),
            user_id: subject.as_str().to_owned(),
        })),
        _ => Ok(None),
    }
}

/// Parse a named cookie from the `Cookie` request header.
fn extract_cookie(req: &Request, name: &str) -> Option<String> {
    let cookie_header = req.headers().get("Cookie").ok()??;
    for pair in cookie_header.split(';') {
        let mut parts = pair.splitn(2, '=');
        let k = parts.next()?.trim();
        let v = parts.next()?.trim();
        if k == name {
            return Some(v.to_owned());
        }
    }
    None
}

/// Build a `Set-Cookie` header value for the session cookie (RFC-003).
///
/// Used by the legacy (non-wasm) code path. On wasm32, codlet's
/// `CookiePolicy::build_set_cookie` handles session cookie construction.
#[cfg(not(target_arch = "wasm32"))]
pub fn build_session_cookie(secret: &str, domain: Option<&str>) -> String {
    let domain_part = domain
        .filter(|d| !d.is_empty())
        .map(|d| format!("; Domain={d}"))
        .unwrap_or_default();
    format!(
        "{name}={secret}; Max-Age={max_age}; Path=/; HttpOnly; Secure; SameSite=Strict{domain_part}",
        name = SESSION_COOKIE_NAME,
        secret = secret,
        max_age = SESSION_TTL_SECONDS,
        domain_part = domain_part,
    )
}

/// Build a `Set-Cookie` header that clears the session cookie (logout).
/// Used by the legacy (non-wasm) path; codlet's `CookiePolicy::build_clear_cookie`
/// handles this on wasm32.
#[cfg(not(target_arch = "wasm32"))]
pub fn clear_session_cookie(domain: Option<&str>) -> String {
    let domain_part = domain
        .filter(|d| !d.is_empty())
        .map(|d| format!("; Domain={d}"))
        .unwrap_or_default();
    format!(
        "{name}=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Strict{domain_part}",
        name = SESSION_COOKIE_NAME,
        domain_part = domain_part,
    )
}
