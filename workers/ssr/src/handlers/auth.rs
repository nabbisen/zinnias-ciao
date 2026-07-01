//! Logout handler — RFC-003.
//!
//! On wasm32: uses codlet `SessionManager::revoke` + `CookiePolicy::build_clear_cookie`.
//! On non-wasm (native tests): uses legacy session DB + clear_session_cookie.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;

use crate::db::session as session_db;
use crate::form_token;
use crate::session::require_auth;

pub async fn post_logout(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a)  => a,
        Err(_) => return redirect("/join"),
    };

    let body      = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();

    let pepper = crate::crypto::pepper(env);
    let db     = env.d1("DB")?;

    // Validate the logout CSRF form token (legacy path — covers both wasm32
    // and non-wasm since all form tokens still go through the service table).
    let _ = form_token::consume(
        &db, &pepper, &auth.user_id,
        token_purpose::LOGOUT, &raw_token, None,
    ).await?;

    // ── Revoke session ────────────────────────────────────────────────────
    // Try the codlet session store first (for sessions issued after the
    // migration); fall back to the legacy sessions table.
    #[cfg(target_arch = "wasm32")]
    {
        use codlet_core::secret::SessionId;

        if let Ok(mgr) = crate::codlet::build_session_mgr(&env) {
            let session_id = SessionId::new(auth.session_id.clone().into());
            let _ = mgr.revoke(&session_id).await;
            // Also revoke in legacy table during the grace period.
            let _ = session_db::revoke(&db, &auth.session_id).await;

            let _ = crate::audit::write(
                &db, rid, None, None,
                "session", Some(&auth.session_id), "logout", None,
            ).await;

            let clear = crate::codlet::session_clear_cookie();
            let mut resp = redirect("/join")?;
            resp.headers_mut().set("Set-Cookie", &clear)?;
            return Ok(resp);
        }
        // CODLET_HMAC_KEY_V1 not yet set — fall through to legacy path.
    }

    // ── Legacy path (non-wasm tests or pre-CODLET_HMAC_KEY_V1 deploy) ────
    let _ = session_db::revoke(&db, &auth.session_id).await;
    let _ = crate::audit::write(
        &db, rid, None, None,
        "session", Some(&auth.session_id), "logout", None,
    ).await;

    let domain = env.var("SESSION_COOKIE_DOMAIN").ok()
        .map(|s| s.to_string()).filter(|s| !s.is_empty());

    #[cfg(not(target_arch = "wasm32"))]
    let clear = crate::session::clear_session_cookie(domain.as_deref());

    #[cfg(target_arch = "wasm32")]
    let clear = {
        let domain_part = domain
            .filter(|d| !d.is_empty())
            .map(|d| format!("; Domain={d}"))
            .unwrap_or_default();
        format!(
            "ciao_sid=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Strict{domain_part}"
        )
    };

    let mut resp = redirect("/join")?;
    resp.headers_mut().set("Set-Cookie", &clear)?;
    Ok(resp)
}

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}
