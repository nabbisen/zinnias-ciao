//! Join / onboarding handlers — RFC-003.
//!
//! Flow:
//!   GET  /join              → render invite-code form
//!   POST /join              → validate invite; 303 → /join/profile
//!   GET  /join/profile      → render display-name form
//!   POST /join/profile      → atomic claim → session issue; 303 → /
//!
//! Codes, sessions, and form tokens are stored as HMACs. The invite claim is
//! the serialization point: profile completion wins a conditional
//! `used_at IS NULL` update before creating the user, membership, and session.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::Locale;
use zinnias_ciao_contracts::i18n;
use zinnias_ciao_domain::{validate_display_name, validate_invite_input};

use crate::abuse_control::{self, Outcome, Scope};
use crate::form_token::ConsumeResult;
use crate::render::{self, escape_html};

// ── GET /join ─────────────────────────────────────────────────────────────

pub async fn get_join(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    match crate::session::require_auth(&req, env).await {
        Ok(_) => return redirect("/"),
        Err(crate::session::AuthError::Unauthenticated) => {}
        Err(error) => return Err(error.into_worker_error()),
    }
    // RFC-083 §8.1: no membership exists on this route (rung 1 never
    // applies here) — resolve from Accept-Language (rung 2), falling to
    // Japanese (rung 3) when it doesn't negotiate.
    let locale = crate::authz::resolve_anonymous_locale(&req);
    let token = anon_token(env).await?;
    render_join_form(&token, None, locale)
}

// ── POST /join ────────────────────────────────────────────────────────────

pub async fn post_join(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    // Resolved before body parsing so it is available on every error path
    // below, including the ones that never reach `legacy_post_join`.
    let locale = crate::authz::resolve_anonymous_locale(&req);

    // Direct-edge ingress validation runs before body parsing, form-token
    // D1 access, limiter access, and application D1 access (RFC-078). A
    // rejection returns the fixed generic 503 without touching D1 or
    // issuing a token.
    let client_network = match abuse_control::canonical_client_network(&req) {
        Ok(subject) => subject,
        Err(rejection) => {
            abuse_control::log_ingress_rejected(rid, "join", rejection);
            return render::configuration_unavailable();
        }
    };

    let body = req.form_data().await?;
    let raw_code = body.get_field("code").unwrap_or_default();
    let raw_token = body.get_field("_token").unwrap_or_default();

    if validate_invite_input(&raw_code).is_err() {
        worker::console_log!("[{}] join invite rejected: reason=format", rid);
        return refresh_join_form(env, Some(i18n::t(locale, i18n::JOIN_CODE_HINT)), locale).await;
    }

    legacy_post_join(env, rid, raw_code, raw_token, client_network, locale).await
}

// ── GET /join/profile ──────────────────────────────────────────────────────

pub async fn get_profile(req: Request, _env: &Env, _rid: &str) -> Result<Response> {
    let locale = crate::authz::resolve_anonymous_locale(&req);
    let pt = extract_cookie(&req, "__join_ptoken").unwrap_or_default();
    render_profile_form(&pt, None, locale)
}

// ── POST /join/profile ─────────────────────────────────────────────────────

pub async fn post_profile(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    let locale = crate::authz::resolve_anonymous_locale(&req);
    let body = req.form_data().await?;
    let display_name_raw = body.get_field("display_name").unwrap_or_default();
    let raw_token = body.get_field("_token").unwrap_or_default();

    let display_name = match validate_display_name(&display_name_raw) {
        Ok(n) => n,
        Err(e) => {
            let pt = extract_cookie(&req, "__join_ptoken").unwrap_or_default();
            return render_profile_form(&pt, Some(e.to_string().leak()), locale);
        }
    };

    let ticket_raw = extract_cookie(&req, "__join_ticket").unwrap_or_default();

    legacy_post_profile(req, env, rid, ticket_raw, raw_token, display_name).await
}

// ── Shared storage-backed helpers ──────────────────────────────────────────

async fn legacy_post_join(
    env: &Env,
    rid: &str,
    raw_code: String,
    raw_token: String,
    client_network: String,
    locale: Locale,
) -> Result<Response> {
    use zinnias_ciao_contracts::auth::token_purpose;
    let pepper = crate::crypto::pepper(env)?;
    let db = env.d1("DB")?;

    let consumed = crate::form_token::consume_detailed(
        &db,
        pepper.as_str(),
        "",
        token_purpose::REDEEM_INVITE,
        &raw_token,
        None,
    )
    .await?;
    if matches!(consumed, ConsumeResult::Replay(_)) {
        worker::console_log!("[{}] join invite rejected: reason=form_replay", rid);
        return refresh_join_form(env, Some(i18n::t(locale, i18n::JOIN_CODE_HINT)), locale).await;
    }

    match abuse_control::reserve(env, pepper.as_str(), Scope::Invite, &client_network).await {
        Outcome::Allowed => {}
        Outcome::Blocked {
            retry_after_seconds,
        } => {
            abuse_control::log_blocked(rid, "join", Scope::Invite);
            let resp =
                refresh_join_form(env, Some(i18n::t(locale, i18n::JOIN_CODE_HINT)), locale).await?;
            return abuse_control::apply_blocked(resp, retry_after_seconds);
        }
        Outcome::Unavailable { category } => {
            abuse_control::log_unavailable(rid, "join", Scope::Invite, category);
            let resp =
                refresh_join_form(env, Some(i18n::t(locale, i18n::JOIN_CODE_HINT)), locale).await?;
            return Ok(resp.with_status(503));
        }
    }

    let normalized = crate::crypto::normalize_invite_code(&raw_code);
    let code_hmac = crate::crypto::hmac_hex(pepper.as_str(), &normalized);
    let invite = crate::db::invite::find_valid(&db, &code_hmac).await?;
    if invite.is_none() {
        worker::console_log!("[{}] join invite rejected: reason=no_valid_invite", rid);
        return refresh_join_form(env, Some(i18n::t(locale, i18n::JOIN_CODE_HINT)), locale).await;
    }
    let invite = invite.unwrap();
    abuse_control::reset(env, rid, pepper.as_str(), Scope::Invite, &client_network).await;
    let ticket = crate::crypto::random_token();
    let ticket_value = format!("{}:{}", invite.id, invite.community_id);
    let ticket_hmac = crate::crypto::hmac_hex(pepper.as_str(), &ticket_value);
    let profile_token = crate::form_token::issue(
        &db,
        pepper.as_str(),
        &ticket,
        token_purpose::JOIN_PROFILE,
        Some(&ticket_hmac),
    )
    .await?;
    let join_cookie = format!(
        "__join_ticket={ticket}|{ticket_value}; Max-Age=600; Path=/join; HttpOnly; Secure; SameSite=Strict"
    );
    let token_cookie = format!(
        "__join_ptoken={profile_token}; Max-Age=600; Path=/join; HttpOnly; Secure; SameSite=Strict"
    );
    let mut resp = redirect("/join/profile")?;
    resp.headers_mut().set("Set-Cookie", &join_cookie)?;
    resp.headers_mut().append("Set-Cookie", &token_cookie)?;
    Ok(resp)
}

async fn legacy_post_profile(
    _req: Request,
    env: &Env,
    rid: &str,
    ticket_raw: String,
    raw_token: String,
    display_name: String,
) -> Result<Response> {
    use zinnias_ciao_contracts::auth::token_purpose;
    let mut parts = ticket_raw.splitn(2, '|');
    let ticket = parts.next().unwrap_or_default().to_owned();
    let ticket_value = parts.next().unwrap_or_default().to_owned();
    if ticket.is_empty() || ticket_value.is_empty() {
        return redirect("/join");
    }
    let pepper = crate::crypto::pepper(env)?;
    let ticket_hmac = crate::crypto::hmac_hex(pepper.as_str(), &ticket_value);
    let db = env.d1("DB")?;
    let replay = crate::form_token::consume_detailed(
        &db,
        pepper.as_str(),
        &ticket,
        token_purpose::JOIN_PROFILE,
        &raw_token,
        Some(&ticket_hmac),
    )
    .await?;
    if matches!(replay, ConsumeResult::Replay(_)) {
        return redirect("/");
    }
    let mut tv = ticket_value.splitn(2, ':');
    let invite_id = tv.next().unwrap_or_default().to_owned();
    let community_id = tv.next().unwrap_or_default().to_owned();
    if invite_id.is_empty() || community_id.is_empty() {
        return redirect("/join");
    }
    let grants_role = crate::db::invite::find_by_id(&db, &invite_id)
        .await?
        .map(|inv| inv.grants_role)
        .unwrap_or_else(|| "member".to_owned());
    let user_id = crate::crypto::random_token();
    let membership_id = crate::crypto::random_token();
    let session_secret = crate::crypto::random_token();
    let session_hmac = crate::crypto::hmac_hex(pepper.as_str(), &session_secret);
    let session_id = crate::crypto::random_token();
    if let Err(error) = crate::db::invite::redeem_required(
        &db,
        rid,
        &invite_id,
        &community_id,
        &grants_role,
        &user_id,
        &membership_id,
        &display_name,
        &session_id,
        &session_hmac,
    )
    .await
    {
        if !crate::db::invite::claim_is_still_eligible(&db, &invite_id, &community_id, &grants_role)
            .await?
        {
            return redirect("/join");
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
    let clear_join = "__join_ticket=; Max-Age=0; Path=/join; HttpOnly; Secure; SameSite=Strict";
    let clear_ptoken = "__join_ptoken=; Max-Age=0; Path=/join; HttpOnly; Secure; SameSite=Strict";
    let mut resp = redirect("/")?;
    resp.headers_mut().set("Set-Cookie", &session_cookie)?;
    resp.headers_mut().append("Set-Cookie", clear_join)?;
    resp.headers_mut().append("Set-Cookie", clear_ptoken)?;
    Ok(resp)
}

// ── Shared helpers ─────────────────────────────────────────────────────────

async fn anon_token(env: &Env) -> Result<String> {
    use zinnias_ciao_contracts::auth::token_purpose;
    let pepper = crate::crypto::pepper(env)?;
    let db = env.d1("DB")?;
    crate::form_token::issue(&db, pepper.as_str(), "", token_purpose::REDEEM_INVITE, None).await
}

async fn refresh_join_form(
    env: &Env,
    error: Option<&'static str>,
    locale: Locale,
) -> Result<Response> {
    let token = anon_token(env).await?;
    render_join_form(&token, error, locale)
}

fn extract_cookie(req: &Request, name: &str) -> Option<String> {
    let h = req.headers().get("Cookie").ok()??;
    for pair in h.split(';') {
        let mut p = pair.splitn(2, '=');
        if p.next()?.trim() == name {
            return Some(p.next()?.trim().to_owned());
        }
    }
    None
}

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}

fn render_join_form(token: &str, error: Option<&str>, locale: Locale) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" class=\"cz-anon-error-text\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let title = i18n::t(locale, i18n::JOIN_PAGE_TITLE);
    let body = format!(
        "<main class=\"cz-anon-main\">\
         <h1 class=\"cz-anon-title\">{heading}</h1>\
         <p class=\"cz-anon-subtitle\">{sub}</p>\
         {error_html}\
         <form method=\"post\" action=\"/join\" class=\"cz-anon-form\">\
           <label class=\"cz-anon-label\">{label}</label>\
           <input name=\"code\" type=\"text\" autocomplete=\"off\" inputmode=\"text\" \
                  maxlength=\"16\" class=\"cz-field-input\" required>\
           <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
           <button type=\"submit\" class=\"cz-anon-submit-button\">{submit}</button>\
         </form>\
         <p class=\"cz-join-code-hint\">{hint}</p>\
         <p class=\"cz-join-relink-hint\">\
           {relink_hint} <a href=\"/relink\" class=\"cz-plain-link\">\
           {relink_link}</a></p>\
         </main>",
        heading = i18n::t(locale, i18n::JOIN_HEADING),
        sub = i18n::t(locale, i18n::JOIN_SUBHEADING),
        label = i18n::t(locale, i18n::JOIN_CODE_LABEL),
        token = escape_html(token),
        submit = i18n::t(locale, i18n::JOIN_SUBMIT),
        hint = i18n::t(locale, i18n::JOIN_CODE_HINT),
        relink_hint = i18n::t(locale, i18n::JOIN_RELINK_HINT),
        relink_link = i18n::t(locale, i18n::JOIN_RELINK_LINK),
    );
    render::page_localized(locale, title, &body)
}

fn render_profile_form(
    token: &str,
    error: Option<&'static str>,
    locale: Locale,
) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" class=\"cz-anon-error-text\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let title = i18n::t(locale, i18n::JOIN_PROFILE_PAGE_TITLE);
    let body = format!(
        "<main class=\"cz-anon-main\">\
         <h1 class=\"cz-anon-title\">{heading}</h1>\
         <p class=\"cz-anon-hint-text\">{hint}</p>\
         {error_html}\
         <form method=\"post\" action=\"/join/profile\" class=\"cz-anon-form\">\
           <label class=\"cz-anon-label\">{label}</label>\
           <input name=\"display_name\" type=\"text\" autocomplete=\"nickname\" \
                  maxlength=\"40\" class=\"cz-field-input\" required>\
           <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
           <button type=\"submit\" class=\"cz-anon-submit-button\">{submit}</button>\
         </form>\
         </main>",
        heading = i18n::t(locale, i18n::JOIN_PROFILE_HEADING),
        hint = i18n::t(locale, i18n::JOIN_PROFILE_HINT),
        label = i18n::t(locale, i18n::JOIN_PROFILE_LABEL),
        token = escape_html(token),
        submit = i18n::t(locale, i18n::JOIN_PROFILE_SUBMIT),
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

    /// Handoff 075 §8: these anonymous pages have no header or nav (no
    /// community context to switch between) — composes the same body
    /// pieces `render_join_form` assembles, at `Locale::En`, with a
    /// `Locale::Ja` discriminating half.
    #[test]
    fn join_code_form_renders_with_no_japanese_codepoint_in_english_locale() {
        let en_body = format!(
            "{heading}{sub}{label}{submit}{hint}{relink_hint}{relink_link}",
            heading = i18n::t(Locale::En, i18n::JOIN_HEADING),
            sub = i18n::t(Locale::En, i18n::JOIN_SUBHEADING),
            label = i18n::t(Locale::En, i18n::JOIN_CODE_LABEL),
            submit = i18n::t(Locale::En, i18n::JOIN_SUBMIT),
            hint = i18n::t(Locale::En, i18n::JOIN_CODE_HINT),
            relink_hint = i18n::t(Locale::En, i18n::JOIN_RELINK_HINT),
            relink_link = i18n::t(Locale::En, i18n::JOIN_RELINK_LINK),
        );
        assert!(
            !contains_japanese_codepoint(&en_body),
            "English-locale join form must contain no Japanese codepoint, found some in: {en_body}"
        );

        let ja_body = format!(
            "{heading}{sub}",
            heading = i18n::t(Locale::Ja, i18n::JOIN_HEADING),
            sub = i18n::t(Locale::Ja, i18n::JOIN_SUBHEADING),
        );
        assert!(
            contains_japanese_codepoint(&ja_body),
            "Japanese-locale join form render must contain Japanese text"
        );
    }

    #[test]
    fn join_profile_form_renders_with_no_japanese_codepoint_in_english_locale() {
        let en_body = format!(
            "{heading}{hint}{label}{submit}",
            heading = i18n::t(Locale::En, i18n::JOIN_PROFILE_HEADING),
            hint = i18n::t(Locale::En, i18n::JOIN_PROFILE_HINT),
            label = i18n::t(Locale::En, i18n::JOIN_PROFILE_LABEL),
            submit = i18n::t(Locale::En, i18n::JOIN_PROFILE_SUBMIT),
        );
        assert!(
            !contains_japanese_codepoint(&en_body),
            "English-locale join profile form must contain no Japanese codepoint, found some in: {en_body}"
        );

        let ja_body = i18n::t(Locale::Ja, i18n::JOIN_PROFILE_HEADING);
        assert!(
            contains_japanese_codepoint(ja_body),
            "Japanese-locale join profile form render must contain Japanese text"
        );
    }
}
