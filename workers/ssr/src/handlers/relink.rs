//! Public active-member help-signin redemption — RFC-024.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::abuse_control::{self, Outcome, Scope};
use crate::crypto::{hmac_hex, normalize_invite_code, random_token};
use crate::db::relink as relink_db;
use crate::form_token::ConsumeResult;
use crate::render::{self, escape_html};

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}

// ── GET /relink ──────────────────────────────────────────────────────────

pub async fn get_relink(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    match crate::session::require_auth(&req, env).await {
        Ok(_) => return redirect("/"),
        Err(crate::session::AuthError::Unauthenticated) => {}
        Err(error) => return Err(error.into_worker_error()),
    }
    let token = relink_form_token(env).await?;
    render_relink_form(&token, None)
}

// ── POST /relink ─────────────────────────────────────────────────────────

pub async fn post_relink(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    // Direct-edge ingress validation runs before body parsing, form-token
    // D1 access, limiter access, and application D1 access (RFC-078). A
    // rejection returns the fixed generic 503 without touching D1 or
    // issuing a token.
    let client_network = match abuse_control::canonical_client_network(&req) {
        Ok(subject) => subject,
        Err(rejection) => {
            abuse_control::log_ingress_rejected(rid, "relink", rejection);
            return render::configuration_unavailable();
        }
    };

    let body = req.form_data().await?;
    let raw_code = body.get_field("code").unwrap_or_default();
    let raw_token = body.get_field("_token").unwrap_or_default();
    let pepper = crate::crypto::pepper(env)?;
    let db = env.d1("DB")?;

    let consumed = crate::form_token::consume_detailed(
        &db,
        pepper.as_str(),
        "",
        token_purpose::REDEEM_RELINK,
        &raw_token,
        None,
    )
    .await?;
    if matches!(consumed, ConsumeResult::Replay(_)) {
        worker::console_log!("[{}] relink rejected: reason=form_replay", rid);
        return refresh_relink_form(env, Some(i18n::JA_RELINK_INVALID)).await;
    }

    match abuse_control::reserve(env, pepper.as_str(), Scope::Relink, &client_network).await {
        Outcome::Allowed => {}
        Outcome::Blocked {
            retry_after_seconds,
        } => {
            abuse_control::log_blocked(rid, "relink", Scope::Relink);
            let resp = refresh_relink_form(env, Some(i18n::JA_RELINK_INVALID)).await?;
            return abuse_control::apply_blocked(resp, retry_after_seconds);
        }
        Outcome::Unavailable { category } => {
            abuse_control::log_unavailable(rid, "relink", Scope::Relink, category);
            let resp = refresh_relink_form(env, Some(i18n::JA_RELINK_INVALID)).await?;
            return Ok(resp.with_status(503));
        }
    }

    let normalized = normalize_invite_code(&raw_code);
    let code_hmac = hmac_hex(pepper.as_str(), &normalized);
    let Some(target) = relink_db::find_valid_by_hmac(&db, &code_hmac).await? else {
        worker::console_log!("[{}] relink rejected: reason=no_valid_relink", rid);
        return refresh_relink_form(env, Some(i18n::JA_RELINK_INVALID)).await;
    };

    let session_secret = random_token();
    let session_hmac = hmac_hex(pepper.as_str(), &session_secret);
    let session_id = random_token();
    if let Err(error) =
        relink_db::redeem_required(&db, rid, &target, &session_id, &session_hmac).await
    {
        if relink_db::find_valid_by_hmac(&db, &code_hmac)
            .await?
            .is_none()
        {
            worker::console_log!("[{}] relink rejected: reason=claim_lost", rid);
            return refresh_relink_form(env, Some(i18n::JA_RELINK_INVALID)).await;
        }
        return Err(error);
    }
    abuse_control::reset(env, rid, pepper.as_str(), Scope::Relink, &client_network).await;

    let cookie_domain = env
        .var("SESSION_COOKIE_DOMAIN")
        .ok()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());
    let session_cookie =
        crate::session::build_session_cookie(&session_secret, cookie_domain.as_deref());
    let mut resp = redirect("/")?;
    resp.headers_mut().set("Set-Cookie", &session_cookie)?;
    Ok(resp)
}

async fn relink_form_token(env: &Env) -> Result<String> {
    let pepper = crate::crypto::pepper(env)?;
    let db = env.d1("DB")?;
    crate::form_token::issue(&db, pepper.as_str(), "", token_purpose::REDEEM_RELINK, None).await
}

async fn refresh_relink_form(env: &Env, error: Option<&'static str>) -> Result<Response> {
    let token = relink_form_token(env).await?;
    render_relink_form(&token, error)
}

fn render_relink_form(token: &str, error: Option<&str>) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" class=\"cz-relink-error-text\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let body = format!(
        "<main class=\"cz-anon-main\">\
         <h1 class=\"cz-anon-title\">{title}</h1>\
         <p class=\"cz-anon-subtitle\">{body}</p>\
         {error_html}\
         <form method=\"post\" action=\"/relink\" class=\"cz-anon-form\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           <label for=\"code\" class=\"cz-relink-label\">{code_label}</label>\
           <input id=\"code\" name=\"code\" inputmode=\"text\" autocomplete=\"one-time-code\" required \
             class=\"cz-relink-code-input\">\
           <button type=\"submit\" \
             class=\"cz-anon-submit-button cz-anon-submit-button--sized\">\
             {submit}</button>\
         </form>\
         </main>",
        title = i18n::JA_RELINK_TITLE,
        body = i18n::JA_RELINK_BODY,
        error_html = error_html,
        tok = escape_html(token),
        code_label = i18n::JA_RELINK_CODE_LABEL,
        submit = i18n::JA_RELINK_SUBMIT,
    );
    render::page(i18n::JA_RELINK_TITLE, &body)
}
