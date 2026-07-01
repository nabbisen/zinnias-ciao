//! Logout handler — RFC-003.
//!
//! Revokes the server-side session row and clears the session cookie.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;

use crate::db::session as session_db;
use crate::session::require_auth;

pub async fn post_logout(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return redirect("/join"),
    };

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();

    let db = env.d1("DB")?;

    // Validate the logout CSRF form token.
    let _ =
        crate::codlet::consume_token(env, &auth.user_id, token_purpose::LOGOUT, &raw_token, None)
            .await?;

    let _ = session_db::revoke(&db, &auth.session_id).await;
    let _ = crate::audit::write(
        &db,
        rid,
        None,
        None,
        "session",
        Some(&auth.session_id),
        "logout",
        None,
    )
    .await;

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
