//! Logout handler — RFC-003.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;

use crate::crypto::hmac_hex;
use crate::db::session as session_db;
use crate::form_token;
use crate::session::{clear_session_cookie, require_auth};

pub async fn post_logout(mut req: Request, env: &Env, _rid: &str) -> Result<Response> {
    // Require a valid session first.
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return redirect("/join"),
    };

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();

    let pepper = env
        .secret("HMAC_PEPPER")
        .map(|s| s.to_string())
        .unwrap_or_else(|_| "dev-pepper-change-in-production".to_string());
    let db = env.d1("DB")?;

    // Validate logout form token.
    let _ = form_token::consume(
        &db,
        &pepper,
        &auth.user_id,
        token_purpose::LOGOUT,
        &raw_token,
        None,
    )
    .await?;

    // Revoke session.
    let _ = session_db::revoke(&db, &auth.session_id).await;

    // Clear cookie and redirect.
    let domain = env
        .var("SESSION_COOKIE_DOMAIN")
        .map(|s| s.to_string())
        .unwrap_or_else(|_| "localhost".to_string());
    let clear = clear_session_cookie(&domain);

    let mut resp = redirect("/join")?;
    resp.headers_mut().set("Set-Cookie", &clear)?;
    Ok(resp)
}

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}
