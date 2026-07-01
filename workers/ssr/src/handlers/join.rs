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
use zinnias_ciao_contracts::i18n;
use zinnias_ciao_domain::{validate_display_name, validate_invite_input};

use crate::audit;
use crate::db::membership as membership_db;
use crate::render::{self, escape_html};

// ── GET /join ─────────────────────────────────────────────────────────────

pub async fn get_join(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    if crate::session::require_auth(&req, env).await.is_ok() {
        return redirect("/");
    }
    let token = anon_token(env).await?;
    render_join_form(&token, None)
}

// ── POST /join ────────────────────────────────────────────────────────────

pub async fn post_join(mut req: Request, env: &Env, _rid: &str) -> Result<Response> {
    let body = req.form_data().await?;
    let raw_code = body.get_field("code").unwrap_or_default();
    let raw_token = body.get_field("_token").unwrap_or_default();

    if validate_invite_input(&raw_code).is_err() {
        return refresh_join_form(env, Some(i18n::JA_JOIN_CODE_HINT)).await;
    }

    legacy_post_join(req, env, raw_code, raw_token).await
}

// ── GET /join/profile ──────────────────────────────────────────────────────

pub async fn get_profile(req: Request, _env: &Env, _rid: &str) -> Result<Response> {
    let pt = extract_cookie(&req, "__join_ptoken").unwrap_or_default();
    render_profile_form(&pt, None)
}

// ── POST /join/profile ─────────────────────────────────────────────────────

pub async fn post_profile(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    let body = req.form_data().await?;
    let display_name_raw = body.get_field("display_name").unwrap_or_default();
    let raw_token = body.get_field("_token").unwrap_or_default();

    let display_name = match validate_display_name(&display_name_raw) {
        Ok(n) => n,
        Err(e) => {
            let pt = extract_cookie(&req, "__join_ptoken").unwrap_or_default();
            return render_profile_form(&pt, Some(e.to_string().leak()));
        }
    };

    let ticket_raw = extract_cookie(&req, "__join_ticket").unwrap_or_default();

    legacy_post_profile(req, env, rid, ticket_raw, raw_token, display_name).await
}

// ── Shared storage-backed helpers ──────────────────────────────────────────

async fn legacy_post_join(
    req: Request,
    env: &Env,
    raw_code: String,
    raw_token: String,
) -> Result<Response> {
    use zinnias_ciao_contracts::auth::token_purpose;
    let client_ip = crate::rate_limit::client_ip(&req);
    if crate::rate_limit::is_rate_limited(env, &client_ip).await {
        return refresh_join_form(env, Some(i18n::JA_JOIN_CODE_HINT)).await;
    }
    let pepper = crate::crypto::pepper(env);
    let db = env.d1("DB")?;
    let _ = crate::form_token::consume(
        &db,
        &pepper,
        "",
        token_purpose::REDEEM_INVITE,
        &raw_token,
        None,
    )
    .await?;
    let normalized = crate::crypto::normalize_invite_code(&raw_code);
    let code_hmac = crate::crypto::hmac_hex(&pepper, &normalized);
    let invite = crate::db::invite::find_valid(&db, &code_hmac).await?;
    if invite.is_none() {
        crate::rate_limit::record_failure(env, &client_ip).await;
        return refresh_join_form(env, Some(i18n::JA_JOIN_CODE_HINT)).await;
    }
    let invite = invite.unwrap();
    crate::rate_limit::clear_failures(env, &client_ip).await;
    let ticket = crate::crypto::random_token();
    let ticket_value = format!("{}:{}", invite.id, invite.community_id);
    let ticket_hmac = crate::crypto::hmac_hex(&pepper, &ticket_value);
    let profile_token = crate::form_token::issue(
        &db,
        &pepper,
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
    let pepper = crate::crypto::pepper(env);
    let ticket_hmac = crate::crypto::hmac_hex(&pepper, &ticket_value);
    let db = env.d1("DB")?;
    let replay = crate::form_token::consume(
        &db,
        &pepper,
        &ticket,
        token_purpose::JOIN_PROFILE,
        &raw_token,
        Some(&ticket_hmac),
    )
    .await?;
    if replay.is_some() {
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
    let won = crate::db::invite::mark_used(&db, &invite_id).await?;
    if !won {
        return redirect("/join");
    }
    membership_db::insert_user(&db, &user_id).await?;
    membership_db::insert_membership(
        &db,
        &membership_id,
        &community_id,
        &user_id,
        &grants_role,
        &display_name,
    )
    .await?;
    crate::db::invite::assign_used_membership(&db, &invite_id, &membership_id).await?;
    let session_secret = crate::crypto::random_token();
    let session_hmac = crate::crypto::hmac_hex(&pepper, &session_secret);
    let session_id = crate::crypto::random_token();
    crate::db::session::insert(&db, &session_id, &user_id, &session_hmac).await?;
    let _ = audit::write(
        &db,
        rid,
        Some(&community_id),
        Some(&membership_id),
        "invite_code",
        Some(&invite_id),
        "redeemed",
        Some(serde_json::json!({ "membership_id": membership_id })),
    )
    .await;
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
    let pepper = crate::crypto::pepper(env);
    let db = env.d1("DB")?;
    crate::form_token::issue(&db, &pepper, "", token_purpose::REDEEM_INVITE, None).await
}

async fn refresh_join_form(env: &Env, error: Option<&'static str>) -> Result<Response> {
    let token = anon_token(env).await?;
    render_join_form(&token, error)
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

fn render_join_form(token: &str, error: Option<&str>) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" style=\"color:#FF3B30\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let body = format!(
        "<main style=\"padding:2rem;max-width:480px;margin:auto;font-family:system-ui,sans-serif\">\
         <h1 style=\"font-size:1.25rem;font-weight:600\">{heading}</h1>\
         <p style=\"color:#6e6e73\">{sub}</p>\
         {error_html}\
         <form method=\"post\" action=\"/join\" style=\"margin-top:1.5rem\">\
           <label style=\"display:block;margin-bottom:.5rem;font-size:.875rem\">{label}</label>\
           <input name=\"code\" type=\"text\" autocomplete=\"off\" inputmode=\"text\" \
                  maxlength=\"16\" style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
                  border-radius:12px;font-size:1rem\" required>\
           <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
           <button type=\"submit\" style=\"margin-top:1rem;width:100%;padding:.875rem;\
                   background:#007AFF;color:#fff;border:none;border-radius:14px;\
                   font-size:1rem;font-weight:600;cursor:pointer\">{submit}</button>\
         </form>\
         <p style=\"margin-top:1.5rem;color:#6e6e73;font-size:.8125rem\">{hint}</p>\
         </main>",
        heading = i18n::JA_JOIN_HEADING,
        sub = i18n::JA_JOIN_SUBHEADING,
        label = i18n::JA_JOIN_CODE_LABEL,
        token = escape_html(token),
        submit = i18n::JA_JOIN_SUBMIT,
        hint = i18n::JA_JOIN_CODE_HINT,
    );
    render::page(i18n::JA_JOIN_PAGE_TITLE, &body)
}

fn render_profile_form(token: &str, error: Option<&'static str>) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" style=\"color:#FF3B30\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let body = format!(
        "<main style=\"padding:2rem;max-width:480px;margin:auto;font-family:system-ui,sans-serif\">\
         <h1 style=\"font-size:1.25rem;font-weight:600\">{heading}</h1>\
         <p style=\"color:#6e6e73;font-size:.875rem\">{hint}</p>\
         {error_html}\
         <form method=\"post\" action=\"/join/profile\" style=\"margin-top:1.5rem\">\
           <label style=\"display:block;margin-bottom:.5rem;font-size:.875rem\">{label}</label>\
           <input name=\"display_name\" type=\"text\" autocomplete=\"nickname\" \
                  maxlength=\"40\" style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
                  border-radius:12px;font-size:1rem\" required>\
           <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
           <button type=\"submit\" style=\"margin-top:1rem;width:100%;padding:.875rem;\
                   background:#007AFF;color:#fff;border:none;border-radius:14px;\
                   font-size:1rem;font-weight:600;cursor:pointer\">{submit}</button>\
         </form>\
         </main>",
        heading = i18n::JA_JOIN_PROFILE_HEADING,
        hint = i18n::JA_JOIN_PROFILE_HINT,
        label = i18n::JA_JOIN_PROFILE_LABEL,
        token = escape_html(token),
        submit = i18n::JA_JOIN_PROFILE_SUBMIT,
    );
    render::page(i18n::JA_JOIN_PROFILE_PAGE_TITLE, &body)
}
