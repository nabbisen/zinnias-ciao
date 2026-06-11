//! Session middleware — RFC-003.
//!
//! Extracts and validates the session cookie on every authenticated request.
//! Identity derives from the session row; never from client-supplied headers.

use worker::{Env, Request, Result};
use zinnias_ciao_contracts::{AppError, SESSION_COOKIE_NAME, SESSION_TTL_SECONDS};

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
/// Error variant tells the handler whether to redirect to /join (expired)
/// or return 400 (missing cookie on a POST that should have had one).
pub async fn require_auth(req: &Request, env: &Env) -> Result<AuthContext> {
    let pepper = env
        .secret("HMAC_PEPPER")
        .map(|s| s.to_string())
        .unwrap_or_else(|_| "dev-pepper-change-in-production".to_string());

    let cookie_secret = extract_cookie(req, SESSION_COOKIE_NAME).ok_or_else(|| {
        worker::Error::RustError(AppError::session_expired().user_message.to_string())
    })?;

    let hmac = hmac_hex(&pepper, &cookie_secret);

    let db = env.d1("DB")?;
    let session = session_db::find_active(&db, &hmac).await?.ok_or_else(|| {
        worker::Error::RustError(AppError::session_expired().user_message.to_string())
    })?;

    Ok(AuthContext {
        session_id: session.id,
        user_id: session.user_id,
    })
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
/// Max-Age is set from `SESSION_TTL_SECONDS` **only** — never from an
/// upstream token exp (regression rule, RFC-003 §8).
pub fn build_session_cookie(secret: &str, domain: &str) -> String {
    let max_age = SESSION_TTL_SECONDS;
    format!(
        "{name}={secret}; Max-Age={max_age}; Path=/; HttpOnly; Secure; SameSite=Strict; Domain={domain}",
        name = SESSION_COOKIE_NAME,
        secret = secret,
        max_age = max_age,
        domain = domain,
    )
}

/// Build a `Set-Cookie` header that clears the session cookie (logout).
pub fn clear_session_cookie(domain: &str) -> String {
    format!(
        "{name}=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Strict; Domain={domain}",
        name = SESSION_COOKIE_NAME,
        domain = domain,
    )
}
