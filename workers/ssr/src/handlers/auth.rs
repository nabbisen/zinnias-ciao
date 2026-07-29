//! Logout handler — RFC-003.
//!
//! Revokes the server-side session row and clears the session cookie.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;

use crate::db::session as session_db;

pub async fn post_logout(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, redirect("/join"));

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();

    let db = env.d1("DB")?;

    // Validate the logout CSRF form token. Invalid or absent tokens are
    // still rejected — the trailing `?` below propagates that `Err`
    // unchanged. Unlike every other purpose, only a *replayed* (previously
    // valid) logout token is deliberately allowed to proceed rather than
    // rejected: logout is RFC-079's sole Class C safety-first exception
    // (never block a logout), `session_db::revoke` is itself idempotent,
    // and cookie clearing is idempotent too. The only visible effect of a
    // replay is a second `session.logout` audit row, which is noise, not a
    // defect. Matched explicitly here, rather than discarding the result
    // outright, so this reads as a deliberate decision rather than the
    // oversight fixed at the other 21 form-token call sites (2026-07-28
    // remediation).
    match crate::codlet::consume_token(env, &auth.user_id, token_purpose::LOGOUT, &raw_token, None)
        .await?
    {
        crate::codlet::ConsumeResult::Proceed | crate::codlet::ConsumeResult::Replay(_) => {}
    }

    session_db::revoke(&db, &auth.session_id).await?;
    crate::audit::write_logout_secondary(&db, rid).await;

    let domain = env
        .var("SESSION_COOKIE_DOMAIN")
        .ok()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    let clear = crate::session::clear_session_cookie(domain.as_deref());

    let mut resp = redirect("/join")?;
    resp.headers_mut().set("Set-Cookie", &clear)?;
    Ok(resp)
}

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}
