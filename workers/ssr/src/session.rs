//! Session middleware — RFC-003.
//!
//! Extracts and validates the session cookie on every authenticated request.
//! Identity derives from the session row; never from client-supplied headers.

use worker::{Env, Request};
use zinnias_ciao_contracts::{AppError, SESSION_COOKIE_NAME, SESSION_TTL_SECONDS};

use crate::crypto::hmac_hex;
use crate::db::session as session_db;

/// The resolved session attached to a request context.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub session_id: String,
    pub user_id: String,
    /// RFC-081 §2 / Handoff 048 — see `db::session::SessionRow` for what
    /// these mean and why `provenance` is optional in the type.
    pub provenance: Option<String>,
    pub scope_community_id: Option<String>,
    /// RFC-080 §6 / Handoff 055 — see `db::session::SessionRow` for the
    /// fail-closed NULL treatment.
    pub authenticated_at: Option<String>,
}

/// Authentication failure keeps configuration unavailability distinct from
/// ordinary missing/expired credentials until the handler boundary.
pub enum AuthError {
    Configuration(crate::crypto::PepperConfigError),
    Unauthenticated,
    Runtime(worker::Error),
}

impl AuthError {
    pub fn into_worker_error(self) -> worker::Error {
        match self {
            Self::Configuration(error) => error.into(),
            Self::Unauthenticated => {
                worker::Error::RustError(AppError::session_expired().user_message.to_string())
            }
            Self::Runtime(error) => error,
        }
    }
}

impl From<crate::crypto::PepperConfigError> for AuthError {
    fn from(error: crate::crypto::PepperConfigError) -> Self {
        Self::Configuration(error)
    }
}

impl From<worker::Error> for AuthError {
    fn from(error: worker::Error) -> Self {
        Self::Runtime(error)
    }
}

#[macro_export]
macro_rules! require_auth_or {
    ($req:expr, $env:expr, $unauthenticated:expr) => {{
        match $crate::session::require_auth($req, $env).await {
            Ok(auth) => auth,
            Err($crate::session::AuthError::Unauthenticated) => return $unauthenticated,
            Err(error) => return Err(error.into_worker_error()),
        }
    }};
}

/// Extract the session cookie, hash it, look it up in D1.
/// Returns `Ok(AuthContext)` on success, `Err(AppError)` otherwise.
///
pub async fn require_auth(req: &Request, env: &Env) -> std::result::Result<AuthContext, AuthError> {
    let pepper = crate::crypto::pepper(env)?;

    let cookie_secret =
        extract_cookie(req, SESSION_COOKIE_NAME).ok_or(AuthError::Unauthenticated)?;

    let db = env.d1("DB")?;

    let hmac = hmac_hex(pepper.as_str(), &cookie_secret);
    let session = session_db::find_active(&db, &hmac)
        .await?
        .ok_or(AuthError::Unauthenticated)?;

    Ok(AuthContext {
        session_id: session.id,
        user_id: session.user_id,
        provenance: session.provenance,
        scope_community_id: session.scope_community_id,
        authenticated_at: session.authenticated_at,
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
