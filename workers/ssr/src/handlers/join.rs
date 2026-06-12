//! Join / onboarding handlers — RFC-003.
//!
//! Flow:
//!   GET  /join              → render invite-code form
//!   POST /join              → validate + find invite; 303 → /join/profile
//!   GET  /join/profile      → render display-name form
//!   POST /join/profile      → create user + membership + session; 303 → /
//!
//! All writes are behind the form token (AD-4 / RFC-012).
//! Invite codes are looked up by HMAC — never stored or logged in plaintext.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::{auth::token_purpose, i18n};
use zinnias_ciao_domain::{validate_display_name, validate_invite_input};

use crate::audit;
use crate::crypto::{hmac_hex, normalize_invite_code, random_token};
use crate::rate_limit;
use crate::db::{invite as invite_db, membership as membership_db, session as session_db};
use crate::form_token;
use crate::render::{self, escape_html};
use crate::session::build_session_cookie;

// ── GET /join ────────────────────────────────────────────────────────────

pub async fn get_join(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    // If the user already has a valid session, redirect to home.
    if crate::session::require_auth(&req, &env).await.is_ok() {
        return redirect("/");
    }

    // Issue a form token for the join POST (CSRF, AD-4).
    // We use a placeholder user_id for pre-auth tokens; the token is
    // bound to the purpose so it cannot be replayed for another action.
    let pepper = crate::crypto::pepper(&env);
    let db = env.d1("DB")?;
    let anon_token =
        form_token::issue(&db, &pepper, "", token_purpose::REDEEM_INVITE, None).await?;

    render_join_form(&anon_token, None)
}

// ── POST /join ───────────────────────────────────────────────────────────

pub async fn post_join(mut req: Request, env: &Env, _rid: &str) -> Result<Response> {
    let body = req.form_data().await?;

    let raw_code = body.get_field("code").unwrap_or_default();
    let raw_token = body.get_field("_token").unwrap_or_default();

    // Rate-limit check (RFC-012) — before any DB work.
    let client_ip = rate_limit::client_ip(&req);
    if rate_limit::is_rate_limited(&env, &client_ip).await {
        return render_join_form(
            &refresh_anon_token(&env).await?,
            Some(zinnias_ciao_contracts::i18n::JA_JOIN_CODE_HINT),
        );
    }

    // Validate the invite code format (domain rule — before DB lookup).
    if let Err(_) = validate_invite_input(&raw_code) {
        return render_join_form(
            &refresh_anon_token(&env).await?,
            Some(i18n::JA_JOIN_CODE_HINT), // re-use hint as generic error position
        );
    }

    // Validate the form token (CSRF).
    let pepper = crate::crypto::pepper(&env);
    let db = env.d1("DB")?;
    // For anon tokens we used user_id = ""
    let _ = form_token::consume(
        &db,
        &pepper,
        "",
        token_purpose::REDEEM_INVITE,
        &raw_token,
        None,
    )
    .await?;

    // Look up the invite by HMAC (never by plaintext).
    let normalized = normalize_invite_code(&raw_code);
    let code_hmac = hmac_hex(&pepper, &normalized);

    let invite = invite_db::find_valid(&db, &code_hmac).await?;

    if invite.is_none() {
        // Generic error: do not reveal whether the code existed (RFC-003 §7).
        rate_limit::record_failure(&env, &client_ip).await;
        return render_join_form(
            &refresh_anon_token(&env).await?,
            Some(i18n::JA_JOIN_CODE_HINT),
        );
    }
    let invite = invite.unwrap();
    // Valid code — clear the failure counter so a legitimate user isn't
    // locked out by their own earlier mistakes.
    rate_limit::clear_failures(&env, &client_ip).await;

    // Stash invite_id + community_id in a short-lived join-ticket cookie
    // so the profile step can complete the redemption atomically.
    let ticket = random_token();
    let ticket_value = format!("{}:{}", invite.id, invite.community_id);
    let ticket_hmac = hmac_hex(&pepper, &ticket_value);

    // Issue a profile form token bound to this ticket.
    let profile_token = form_token::issue(
        &db,
        &pepper,
        &ticket, // ticket is the ephemeral "user_id" for this step
        token_purpose::JOIN_PROFILE,
        Some(&ticket_hmac),
    )
    .await?;

    // 303 → /join/profile, carrying ticket + profile_token in a short-lived cookie.
    let join_cookie = format!(
        "__join_ticket={ticket}|{ticket_value}; Max-Age=600; Path=/join; HttpOnly; Secure; SameSite=Strict"
    );
    let mut resp = redirect("/join/profile")?;
    resp.headers_mut().set("Set-Cookie", &join_cookie)?;
    // Store the profile token in another cookie so the GET can pre-fill the form.
    let token_cookie = format!(
        "__join_ptoken={profile_token}; Max-Age=600; Path=/join; HttpOnly; Secure; SameSite=Strict"
    );
    resp.headers_mut().append("Set-Cookie", &token_cookie)?;
    Ok(resp)
}

// ── GET /join/profile ────────────────────────────────────────────────────

pub async fn get_profile(req: Request, _env: &Env, _rid: &str) -> Result<Response> {
    let profile_token = extract_cookie(&req, "__join_ptoken").unwrap_or_default();
    let community_name = ""; // M1: community name looked up in POST; omit for GET
    render_profile_form(community_name, &profile_token, None)
}

// ── POST /join/profile ───────────────────────────────────────────────────

pub async fn post_profile(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    let body = req.form_data().await?;
    let display_name_raw = body.get_field("display_name").unwrap_or_default();
    let raw_token = body.get_field("_token").unwrap_or_default();

    // Validate display name (domain rule).
    let display_name = match validate_display_name(&display_name_raw) {
        Ok(n) => n,
        Err(e) => {
            let profile_token = extract_cookie(&req, "__join_ptoken").unwrap_or_default();
            return render_profile_form("", &profile_token, Some(e.to_string().leak()));
        }
    };

    // Recover join ticket from cookie.
    let ticket_raw = extract_cookie(&req, "__join_ticket").unwrap_or_default();
    let mut parts = ticket_raw.splitn(2, '|');
    let ticket = parts.next().unwrap_or_default();
    let ticket_value = parts.next().unwrap_or_default();
    if ticket.is_empty() || ticket_value.is_empty() {
        return redirect("/join");
    }

    let pepper = crate::crypto::pepper(&env);
    let ticket_hmac = hmac_hex(&pepper, ticket_value);

    // Validate profile form token.
    let db = env.d1("DB")?;
    let replay = form_token::consume(
        &db,
        &pepper,
        ticket,
        token_purpose::JOIN_PROFILE,
        &raw_token,
        Some(&ticket_hmac),
    )
    .await?;

    // If this token was already consumed (replay), redirect to home.
    if replay.is_some() {
        return redirect("/");
    }

    // Parse ticket_value: "invite_id:community_id"
    let mut tv = ticket_value.splitn(2, ':');
    let invite_id = tv.next().unwrap_or_default().to_owned();
    let community_id = tv.next().unwrap_or_default().to_owned();
    if invite_id.is_empty() || community_id.is_empty() {
        return redirect("/join");
    }

    // ── Redemption sequence ───────────────────────────────────────────────
    // D1 via worker-rs does not support multi-statement transactions, so we
    // make the invite the single point of serialization: claim it FIRST with
    // a conditional UPDATE (used_at IS NULL AND not revoked AND not expired).
    // Only the caller that wins that atomic transition proceeds to create the
    // user/membership/session. Concurrent submissions of the same invite lose
    // the race and are redirected without creating a second member.
    //
    // We need a membership_id for used_by_membership_id, so we generate IDs up
    // front but only persist them after winning the claim.
    let grants_role = invite_db::find_by_id(&db, &invite_id)
        .await?
        .map(|inv| inv.grants_role)
        .unwrap_or_else(|| "member".to_owned());

    let user_id = random_token();
    let membership_id = random_token();

    // 1. Claim the invite atomically. If we don't win, someone already redeemed
    //    it (or it expired/was revoked) — redirect without creating records.
    let won = invite_db::mark_used(&db, &invite_id, &membership_id).await?;
    if !won {
        return redirect("/join");
    }

    // 2. Create user.
    membership_db::insert_user(&db, &user_id).await?;

    // 3. Create membership — role comes from the invite code, not hardcoded.
    //    setup.mjs seeds the bootstrap invite with grants_role='admin'.
    //    Admin-generated invites for new members use grants_role='member' (default).
    membership_db::insert_membership(
        &db,
        &membership_id,
        &community_id,
        &user_id,
        &grants_role,
        &display_name,
    )
    .await?;

    // 4. Create session
    let session_secret = random_token();
    let session_hmac = hmac_hex(&pepper, &session_secret);
    let session_id = random_token();
    session_db::insert(&db, &session_id, &user_id, &session_hmac).await?;

    // 5. Audit
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

    // 6. Set session cookie and redirect to home.
    let cookie_domain = get_domain(&env);
    let session_cookie = build_session_cookie(&session_secret, cookie_domain.as_deref());
    let clear_join = "__join_ticket=; Max-Age=0; Path=/join; HttpOnly; Secure; SameSite=Strict";
    let clear_ptoken = "__join_ptoken=; Max-Age=0; Path=/join; HttpOnly; Secure; SameSite=Strict";

    let mut resp = redirect("/")?;
    resp.headers_mut().set("Set-Cookie", &session_cookie)?;
    resp.headers_mut().append("Set-Cookie", clear_join)?;
    resp.headers_mut().append("Set-Cookie", clear_ptoken)?;
    Ok(resp)
}

// ── Helpers ───────────────────────────────────────────────────────────────


/// The cookie Domain attribute. Read from the `SESSION_COOKIE_DOMAIN` var
/// (a normal `[vars]` binding — the domain is not secret material).
/// Returns `None` when unset/empty so the cookie becomes host-only.
fn get_domain(env: &Env) -> Option<String> {
    env.var("SESSION_COOKIE_DOMAIN")
        .ok()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
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

async fn refresh_anon_token(env: &Env) -> Result<String> {
    let pepper = crate::crypto::pepper(env);
    let db = env.d1("DB")?;
    form_token::issue(&db, &pepper, "", token_purpose::REDEEM_INVITE, None).await
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

fn render_profile_form(
    community_name: &str,
    token: &str,
    error: Option<&'static str>,
) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" style=\"color:#FF3B30\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let community_html = if community_name.is_empty() {
        String::new()
    } else {
        format!(
            "<p style=\"color:#6e6e73\">{}</p>",
            escape_html(community_name)
        )
    };

    let body = format!(
        "<main style=\"padding:2rem;max-width:480px;margin:auto;font-family:system-ui,sans-serif\">\
         <h1 style=\"font-size:1.25rem;font-weight:600\">{heading}</h1>\
         {community_html}\
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
