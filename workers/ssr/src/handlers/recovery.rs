//! Anonymous account recovery credential consumption — RFC-081 §3
//! (Handoff 057 §5.2). Modeled directly on `handlers/relink.rs`: the same
//! trusted-ingress-first shape, the same `abuse_control::reserve` call
//! before any credential lookup (RFC-078 — a stop condition per Handoff
//! 057 §5.2, not a nice-to-have, since this route authenticates an entire
//! account), and the same one-generic-error-for-every-cause discipline.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::Locale;
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::abuse_control::{self, Outcome, Scope};
use crate::crypto::{hmac_hex, normalize_invite_code, random_token};
use crate::db::recovery as recovery_db;
use crate::form_token::ConsumeResult;
use crate::render::{self, escape_html};

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}

// ── GET /recovery ─────────────────────────────────────────────────────────

pub async fn get_recovery(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    match crate::session::require_auth(&req, env).await {
        Ok(_) => return redirect("/"),
        Err(crate::session::AuthError::Unauthenticated) => {}
        Err(error) => return Err(error.into_worker_error()),
    }
    // RFC-083 §8.1: no membership on this route (rung 1 never applies) —
    // resolve from Accept-Language (rung 2), falling to Japanese (rung 3).
    let locale = crate::authz::resolve_anonymous_locale(&req);
    let token = recovery_form_token(env).await?;
    render_recovery_form(&token, None, locale)
}

// ── POST /recovery ────────────────────────────────────────────────────────

pub async fn post_recovery(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    // Resolved before body parsing so it is available on every error path.
    let locale = crate::authz::resolve_anonymous_locale(&req);

    // Direct-edge ingress validation runs before body parsing, form-token
    // D1 access, limiter access, and application D1 access (RFC-078) —
    // same order `handlers/relink.rs::post_relink` already establishes.
    let client_network = match abuse_control::canonical_client_network(&req) {
        Ok(subject) => subject,
        Err(rejection) => {
            abuse_control::log_ingress_rejected(rid, "recovery", rejection);
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
        token_purpose::REDEEM_RECOVERY,
        &raw_token,
        None,
    )
    .await?;
    if matches!(consumed, ConsumeResult::Replay(_)) {
        worker::console_log!("[{}] recovery rejected: reason=form_replay", rid);
        return refresh_recovery_form(env, Some(i18n::t(locale, i18n::RECOVERY_INVALID)), locale)
            .await;
    }

    // Handoff 057 §5.2 / §6 gate 2: reserved before any credential lookup.
    match abuse_control::reserve(env, pepper.as_str(), Scope::Recovery, &client_network).await {
        Outcome::Allowed => {}
        Outcome::Blocked {
            retry_after_seconds,
        } => {
            abuse_control::log_blocked(rid, "recovery", Scope::Recovery);
            let resp =
                refresh_recovery_form(env, Some(i18n::t(locale, i18n::RECOVERY_INVALID)), locale)
                    .await?;
            return abuse_control::apply_blocked(resp, retry_after_seconds);
        }
        Outcome::Unavailable { category } => {
            abuse_control::log_unavailable(rid, "recovery", Scope::Recovery, category);
            let resp =
                refresh_recovery_form(env, Some(i18n::t(locale, i18n::RECOVERY_INVALID)), locale)
                    .await?;
            return Ok(resp.with_status(503));
        }
    }

    let normalized = normalize_invite_code(&raw_code);
    let code_hmac = hmac_hex(pepper.as_str(), &normalized);
    let Some(target) = recovery_db::find_valid_by_hmac(&db, &code_hmac).await? else {
        worker::console_log!("[{}] recovery rejected: reason=no_valid_credential", rid);
        return refresh_recovery_form(env, Some(i18n::t(locale, i18n::RECOVERY_INVALID)), locale)
            .await;
    };

    let session_secret = random_token();
    let session_hmac = hmac_hex(pepper.as_str(), &session_secret);
    let session_id = random_token();
    if let Err(error) =
        recovery_db::consume_required(&db, rid, &target, &session_id, &session_hmac).await
    {
        if recovery_db::find_valid_by_hmac(&db, &code_hmac)
            .await?
            .is_none()
        {
            worker::console_log!("[{}] recovery rejected: reason=claim_lost", rid);
            return refresh_recovery_form(
                env,
                Some(i18n::t(locale, i18n::RECOVERY_INVALID)),
                locale,
            )
            .await;
        }
        return Err(error);
    }

    let cookie_domain = env
        .var("SESSION_COOKIE_DOMAIN")
        .ok()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());
    let session_cookie =
        crate::session::build_session_cookie(&session_secret, cookie_domain.as_deref());
    let mut resp = redirect("/account")?;
    resp.headers_mut().set("Set-Cookie", &session_cookie)?;
    Ok(resp)
}

async fn recovery_form_token(env: &Env) -> Result<String> {
    let pepper = crate::crypto::pepper(env)?;
    let db = env.d1("DB")?;
    crate::form_token::issue(
        &db,
        pepper.as_str(),
        "",
        token_purpose::REDEEM_RECOVERY,
        None,
    )
    .await
}

async fn refresh_recovery_form(
    env: &Env,
    error: Option<&'static str>,
    locale: Locale,
) -> Result<Response> {
    let token = recovery_form_token(env).await?;
    render_recovery_form(&token, error, locale)
}

fn render_recovery_form(token: &str, error: Option<&str>, locale: Locale) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" class=\"cz-recovery-error-text\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let title = i18n::t(locale, i18n::RECOVERY_TITLE);
    let body = format!(
        "<main class=\"cz-anon-main\">\
         <h1 class=\"cz-anon-title\">{title}</h1>\
         <p class=\"cz-anon-subtitle\">{body}</p>\
         {error_html}\
         <form method=\"post\" action=\"/recovery\" class=\"cz-anon-form\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           <label for=\"code\" class=\"cz-recovery-label\">{code_label}</label>\
           <input id=\"code\" name=\"code\" inputmode=\"text\" autocomplete=\"one-time-code\" required \
             class=\"cz-recovery-code-input\">\
           <button type=\"submit\" \
             class=\"cz-anon-submit-button cz-anon-submit-button--sized\">\
             {submit}</button>\
         </form>\
         </main>",
        title = title,
        body = i18n::t(locale, i18n::RECOVERY_BODY),
        error_html = error_html,
        tok = escape_html(token),
        code_label = i18n::t(locale, i18n::RECOVERY_CODE_LABEL),
        submit = i18n::t(locale, i18n::RECOVERY_SUBMIT),
    );
    render::page_localized(locale, title, &body)
}
