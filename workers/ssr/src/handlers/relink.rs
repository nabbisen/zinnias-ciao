//! Public active-member help-signin redemption — RFC-024.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::audit;
use crate::crypto::{hmac_hex, normalize_invite_code, random_token};
use crate::db::relink as relink_db;
use crate::render::{self, escape_html};

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}

// ── GET /relink ──────────────────────────────────────────────────────────

pub async fn get_relink(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    if crate::session::require_auth(&req, env).await.is_ok() {
        return redirect("/");
    }
    let token = relink_form_token(env).await?;
    render_relink_form(&token, None)
}

// ── POST /relink ─────────────────────────────────────────────────────────

pub async fn post_relink(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    let client_ip = crate::rate_limit::client_ip(&req);
    if crate::rate_limit::is_relink_rate_limited(env, &client_ip).await {
        return refresh_relink_form(env, Some(i18n::JA_RELINK_INVALID)).await;
    }

    let body = req.form_data().await?;
    let raw_code = body.get_field("code").unwrap_or_default();
    let raw_token = body.get_field("_token").unwrap_or_default();
    let pepper = crate::crypto::pepper(env);
    let db = env.d1("DB")?;

    let replay = crate::form_token::consume(
        &db,
        &pepper,
        "",
        token_purpose::REDEEM_RELINK,
        &raw_token,
        None,
    )
    .await?;
    if replay.is_some() {
        return refresh_relink_form(env, Some(i18n::JA_RELINK_INVALID)).await;
    }

    let normalized = normalize_invite_code(&raw_code);
    let code_hmac = hmac_hex(&pepper, &normalized);
    let Some(target) = relink_db::find_valid_by_hmac(&db, &code_hmac).await? else {
        crate::rate_limit::record_relink_failure(env, &client_ip).await;
        return refresh_relink_form(env, Some(i18n::JA_RELINK_INVALID)).await;
    };

    if !relink_db::mark_used(&db, &target.id).await? {
        crate::rate_limit::record_relink_failure(env, &client_ip).await;
        return refresh_relink_form(env, Some(i18n::JA_RELINK_INVALID)).await;
    }

    let session_secret = random_token();
    let session_hmac = hmac_hex(&pepper, &session_secret);
    let session_id = random_token();
    crate::db::session::insert(&db, &session_id, &target.user_id, &session_hmac).await?;
    crate::db::session::revoke_others_for_user(&db, &target.user_id, &session_id).await?;
    crate::rate_limit::clear_relink_failures(env, &client_ip).await;

    let _ = audit::write(
        &db,
        rid,
        Some(&target.community_id),
        Some(&target.membership_id),
        "membership",
        Some(&target.membership_id),
        "membership.relink_redeemed",
        Some(serde_json::json!({
            "membership_id": target.membership_id,
            "created_by_membership_id": target.created_by_membership_id,
            "community_id": target.community_id,
        })),
    )
    .await;

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
    let pepper = crate::crypto::pepper(env);
    let db = env.d1("DB")?;
    crate::form_token::issue(&db, &pepper, "", token_purpose::REDEEM_RELINK, None).await
}

async fn refresh_relink_form(env: &Env, error: Option<&'static str>) -> Result<Response> {
    let token = relink_form_token(env).await?;
    render_relink_form(&token, error)
}

fn render_relink_form(token: &str, error: Option<&str>) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" style=\"color:#FF3B30;margin:.75rem 0\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let body = format!(
        "<main style=\"padding:2rem;max-width:480px;margin:auto;font-family:system-ui,sans-serif\">\
         <h1 style=\"font-size:1.25rem;font-weight:600\">{title}</h1>\
         <p style=\"color:#6e6e73\">{body}</p>\
         {error_html}\
         <form method=\"post\" action=\"/relink\" style=\"margin-top:1.5rem\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           <label for=\"code\" style=\"display:block;font-size:.875rem;margin-bottom:.375rem\">{code_label}</label>\
           <input id=\"code\" name=\"code\" inputmode=\"text\" autocomplete=\"one-time-code\" required \
             style=\"width:100%;font-size:1.25rem;padding:.75rem;border:1px solid #d1d1d6;border-radius:12px;box-sizing:border-box;text-transform:uppercase\">\
           <button type=\"submit\" \
             style=\"width:100%;margin-top:1rem;padding:.875rem;background:#007AFF;color:#fff;\
             border:none;border-radius:14px;font-size:1rem;font-weight:600;min-height:44px;cursor:pointer\">\
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
