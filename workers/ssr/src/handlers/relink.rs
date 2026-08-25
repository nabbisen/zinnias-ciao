//! Public active-member help-signin redemption — RFC-024.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::Locale;
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
    // RFC-083 §8.1: no membership on this route (rung 1 never applies) —
    // resolve from Accept-Language (rung 2), falling to Japanese (rung 3).
    let locale = crate::authz::resolve_anonymous_locale(&req);
    let token = relink_form_token(env).await?;
    render_relink_form(&token, None, locale)
}

// ── POST /relink ─────────────────────────────────────────────────────────

pub async fn post_relink(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    // Resolved before body parsing so it is available on every error path.
    let locale = crate::authz::resolve_anonymous_locale(&req);

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
        return refresh_relink_form(env, Some(i18n::t(locale, i18n::RELINK_INVALID)), locale).await;
    }

    match abuse_control::reserve(env, pepper.as_str(), Scope::Relink, &client_network).await {
        Outcome::Allowed => {}
        Outcome::Blocked {
            retry_after_seconds,
        } => {
            abuse_control::log_blocked(rid, "relink", Scope::Relink);
            let resp =
                refresh_relink_form(env, Some(i18n::t(locale, i18n::RELINK_INVALID)), locale)
                    .await?;
            return abuse_control::apply_blocked(resp, retry_after_seconds);
        }
        Outcome::Unavailable { category } => {
            abuse_control::log_unavailable(rid, "relink", Scope::Relink, category);
            let resp =
                refresh_relink_form(env, Some(i18n::t(locale, i18n::RELINK_INVALID)), locale)
                    .await?;
            return Ok(resp.with_status(503));
        }
    }

    let normalized = normalize_invite_code(&raw_code);
    let code_hmac = hmac_hex(pepper.as_str(), &normalized);
    let Some(target) = relink_db::find_valid_by_hmac(&db, &code_hmac).await? else {
        worker::console_log!("[{}] relink rejected: reason=no_valid_relink", rid);
        return refresh_relink_form(env, Some(i18n::t(locale, i18n::RELINK_INVALID)), locale).await;
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
            return refresh_relink_form(env, Some(i18n::t(locale, i18n::RELINK_INVALID)), locale)
                .await;
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

async fn refresh_relink_form(
    env: &Env,
    error: Option<&'static str>,
    locale: Locale,
) -> Result<Response> {
    let token = relink_form_token(env).await?;
    render_relink_form(&token, error, locale)
}

fn render_relink_form(token: &str, error: Option<&str>, locale: Locale) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" class=\"cz-relink-error-text\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let title = i18n::t(locale, i18n::RELINK_TITLE);
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
        title = title,
        body = i18n::t(locale, i18n::RELINK_BODY),
        error_html = error_html,
        tok = escape_html(token),
        code_label = i18n::t(locale, i18n::RELINK_CODE_LABEL),
        submit = i18n::t(locale, i18n::RELINK_SUBMIT),
    );
    render::page_localized(locale, title, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Duplicated locally per `admin/members.rs`'s own precedent (Handoff
    /// 072) rather than shared/exported.
    fn contains_japanese_codepoint(s: &str) -> bool {
        s.chars().any(|c| {
            let cp = c as u32;
            (0x3040..=0x30FF).contains(&cp)
                || (0x4E00..=0x9FFF).contains(&cp)
                || (0x3000..=0x303F).contains(&cp)
                || (0xFF00..=0xFFEF).contains(&cp)
        })
    }

    /// Handoff 075 §8: this anonymous page has no header or nav — composes
    /// the same body pieces `render_relink_form` assembles, at
    /// `Locale::En`, with a `Locale::Ja` discriminating half.
    #[test]
    fn relink_form_renders_with_no_japanese_codepoint_in_english_locale() {
        let en_body = format!(
            "{title}{body}{code_label}{submit}",
            title = i18n::t(Locale::En, i18n::RELINK_TITLE),
            body = i18n::t(Locale::En, i18n::RELINK_BODY),
            code_label = i18n::t(Locale::En, i18n::RELINK_CODE_LABEL),
            submit = i18n::t(Locale::En, i18n::RELINK_SUBMIT),
        );
        assert!(
            !contains_japanese_codepoint(&en_body),
            "English-locale relink form must contain no Japanese codepoint, found some in: {en_body}"
        );

        let ja_body = format!(
            "{title}{body}",
            title = i18n::t(Locale::Ja, i18n::RELINK_TITLE),
            body = i18n::t(Locale::Ja, i18n::RELINK_BODY),
        );
        assert!(
            contains_japanese_codepoint(&ja_body),
            "Japanese-locale relink form render must contain Japanese text"
        );
    }
}
