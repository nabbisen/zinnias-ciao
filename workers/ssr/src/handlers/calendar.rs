//! Calendar export handlers (RFC-023).
//!
//! Four routes:
//!   GET  /c/:cid/me/calendar              — Me calendar page (show/generate feed URL)
//!   POST /c/:cid/me/calendar/regenerate   — generate or rotate feed token
//!   POST /c/:cid/me/calendar/revoke       — revoke (disable) feed
//!   GET  /c/:cid/cal/:token               — unauthenticated ICS feed (bearer URL)

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz::require_membership;
use crate::crypto::{hmac_hex, random_token};
use crate::db::{self, calendar as cal_db};
use crate::form_token;
use crate::render;
use crate::session::require_auth;


// ── GET /c/:cid/me/calendar ───────────────────────────────────────────────

pub async fn get_me_calendar(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let regen_token = form_token::issue(
        &db, &pp, &auth.user_id,
        token_purpose::CALENDAR_REGENERATE, None,
    ).await.unwrap_or_default();
    let revoke_token = form_token::issue(
        &db, &pp, &auth.user_id,
        token_purpose::CALENDAR_REVOKE, None,
    ).await.unwrap_or_default();

    let active = cal_db::find_active_for_membership(&db, &membership.membership_id, community_id).await
        .unwrap_or(None);

    let communities_for_switcher = crate::db::membership::list_communities_for_user(&db, &auth.user_id)
        .await.unwrap_or_default();
    let community_pairs: Vec<(String, String)> = communities_for_switcher.iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone())).collect();

    // Build the feed URL from the request URL origin.
    let origin = {
        let url = req.url()?;
        format!("{}://{}", url.scheme(), url.host_str().unwrap_or("localhost"))
    };

    let url = req.url()?;
    let flash: Option<String> = url.query_pairs()
        .find(|(k, _)| k == "flash").map(|(_, v)| v.to_string());
    let flash_html = flash.map(|f| format!(
        "<p role=\"status\" style=\"font-size:.875rem;color:#167A34;margin:.5rem 0\">{}</p>",
        render::escape_html(&f)
    )).unwrap_or_default();

    let feed_section = if let Some(ref tok) = active {
        // Show feed URL — the token ID is used as the bearer (not the HMAC).
        // We need to recover the plaintext token by re-checking: actually we store
        // the HMAC only. We use the token ID as the URL-visible value and verify
        // it via HMAC(pepper, id) on the feed endpoint. This means the URL is
        // HMAC(pepper, id) itself, which is unguessable without the pepper.
        // We display the feed URL using the token ID as a stable human-readable handle;
        // the actual bearer secret in the URL will be HMAC(pepper, id).
        let feed_url = format!(
            "{origin}/c/{cid}/cal/{hmac}",
            cid  = render::escape_html(community_id),
            hmac = render::escape_html(&hmac_hex(&pp, &tok.id)),
        );
        format!(
            "<div style=\"background:#f5f5f7;border-radius:12px;padding:1rem;margin:1rem 0\">\
             <p style=\"font-size:.8125rem;color:#6E6E73;margin:0 0 .5rem\">\
               Your personal calendar feed URL. Keep this private — \
               anyone with the URL can read your community events.\
             </p>\
             <div style=\"font-size:.75rem;font-family:monospace;word-break:break-all;\
               background:#fff;border:1px solid #E5E5EA;border-radius:8px;\
               padding:.5rem;margin:.5rem 0\">{feed_url}</div>\
             <form method=\"post\" action=\"/c/{cid}/me/calendar/revoke\" \
               style=\"display:inline;margin-right:.5rem\">\
               <input type=\"hidden\" name=\"_token\" value=\"{rtok}\">\
               <button type=\"submit\" \
                 style=\"font-size:.875rem;color:#FF3B30;background:none;border:none;\
                 cursor:pointer;padding:.25rem 0;min-height:44px\">\
                 Disable feed\
               </button>\
             </form>\
             <form method=\"post\" action=\"/c/{cid}/me/calendar/regenerate\" \
               style=\"display:inline\">\
               <input type=\"hidden\" name=\"_token\" value=\"{gentok}\">\
               <button type=\"submit\" \
                 style=\"font-size:.875rem;color:#007AFF;background:none;border:none;\
                 cursor:pointer;padding:.25rem 0;min-height:44px\">\
                 Regenerate URL\
               </button>\
             </form>\
             </div>",
            feed_url = render::escape_html(&feed_url),
            cid      = render::escape_html(community_id),
            rtok     = render::escape_html(&revoke_token),
            gentok   = render::escape_html(&regen_token),
        )
    } else {
        format!(
            "<p style=\"font-size:.875rem;color:#6E6E73;margin:1rem 0\">\
               No calendar feed is active. Generate one to subscribe from your phone calendar.\
             </p>\
             <form method=\"post\" action=\"/c/{cid}/me/calendar/regenerate\">\
               <input type=\"hidden\" name=\"_token\" value=\"{gentok}\">\
               <button type=\"submit\" \
                 style=\"width:100%;padding:.875rem;background:#007AFF;color:#fff;\
                 border:none;border-radius:14px;font-size:1rem;font-weight:600;\
                 min-height:44px;cursor:pointer\">Generate feed URL</button>\
             </form>",
            cid    = render::escape_html(community_id),
            gentok = render::escape_html(&regen_token),
        )
    };

    let nav  = render::bottom_nav(community_id, "me");
    let back = format!(
        "<a href=\"/c/{}/me\" style=\"color:#007AFF;font-size:.9375rem\">\u{2190} {}</a>",
        render::escape_html(community_id), i18n::JA_NAV_ME,
    );
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         {back}\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin:1rem 0 .25rem\">Calendar feed</h1>\
         <p style=\"font-size:.875rem;color:#6E6E73;margin-bottom:1rem\">\
           Subscribe in Apple Calendar, Google Calendar, or any app that supports \
           calendar subscriptions (.ics / webcal).\
         </p>\
         {flash}\
         {feed}\
         </main>{nav}",
        header = render::header_with_switcher(i18n::JA_CALENDAR_TITLE, community_id, &community_pairs),
        back   = back,
        flash  = flash_html,
        feed   = feed_section,
        nav    = nav,
    );
    render::page(i18n::JA_CALENDAR_TITLE, &body)
}

// ── POST /c/:cid/me/calendar/regenerate ───────────────────────────────────

pub async fn post_regenerate_calendar(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(
        &db, &pp, &auth.user_id,
        token_purpose::CALENDAR_REGENERATE, &raw_token, None,
    ).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/me/calendar"));
    }

    let now = db::now_utc();
    // Revoke any existing token first.
    cal_db::revoke_for_membership(&db, &membership.membership_id, community_id, &now).await?;

    // Generate new token — the ID is stored; HMAC(pepper, id) is the bearer secret.
    let token_id  = random_token()[..32].to_owned();
    let token_hmac = hmac_hex(&pp, &token_id);
    cal_db::insert(&db, &token_id, community_id, &membership.membership_id, &token_hmac, &now).await?;

    // Audit calendar token generation (security-relevant, RFC-045 P1-5).
    // The token secret is never logged — only that a feed was (re)generated.
    let _ = crate::audit::write(
        &db, rid, Some(community_id), Some(&membership.membership_id),
        "calendar_feed", None, "calendar_token_generated", None,
    ).await;

    redirect(&format!("/c/{community_id}/me/calendar?flash=Feed+URL+generated"))
}

// ── POST /c/:cid/me/calendar/revoke ──────────────────────────────────────

pub async fn post_revoke_calendar(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(
        &db, &pp, &auth.user_id,
        token_purpose::CALENDAR_REVOKE, &raw_token, None,
    ).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/me/calendar"));
    }

    let now = db::now_utc();
    cal_db::revoke_for_membership(&db, &membership.membership_id, community_id, &now).await?;

    // Audit calendar token revocation (security-relevant, RFC-045 P1-5).
    let _ = crate::audit::write(
        &db, rid, Some(community_id), Some(&membership.membership_id),
        "calendar_feed", None, "calendar_token_revoked", None,
    ).await;

    redirect(&format!("/c/{community_id}/me/calendar?flash=Feed+disabled"))
}

// ── GET /c/:cid/cal/:token ────────────────────────────────────────────────
// Unauthenticated bearer URL. Returns ICS content.

pub async fn get_ics_feed(
    _req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    bearer_token: &str,
) -> Result<Response> {
    let db = env.d1("DB")?;
    // pepper unused here — the bearer token in the URL IS the stored HMAC.

    // The bearer token IS the stored HMAC — look it up directly.
    let claims = cal_db::find_by_hmac(&db, bearer_token).await?;
    let claims = match claims {
        Some(c) if c.community_id == community_id => c,
        _ => {
            // Generic not-found: don't reveal whether token exists.
            return Ok(Response::from_html(
                "<p>Calendar feed not found or has been revoked.</p>"
            )?.with_status(404));
        }
    };

    // Verify the membership is still active in this community.
    let still_active = crate::db::membership::find_active_by_id(
        &db, &claims.membership_id, community_id
    ).await?.is_some();
    if !still_active {
        return Ok(Response::from_html(
            "<p>Calendar feed not available.</p>"
        )?.with_status(403));
    }

    // Fetch events.
    let events = cal_db::events_for_feed(&db, community_id).await?;

    // Build ICS.
    let community = crate::db::community::find_active(&db, community_id).await?;
    let cal_name = community.map(|c| c.name).unwrap_or_else(|| "Community".to_owned());
    let days: Vec<zinnias_ciao_contracts::ics::IcsDay<'_>> = events.iter().map(|ev| {
        zinnias_ciao_contracts::ics::IcsDay {
            uid:           &ev.day_id,
            title:         &ev.title,
            location:      ev.location.as_deref(),
            status:        &ev.status,
            starts_at_utc: &ev.starts_at_utc,
            ends_at_utc:   &ev.ends_at_utc,
        }
    }).collect();
    let ics = zinnias_ciao_contracts::ics::build_vcalendar(&cal_name, &days);

    // Return as text/calendar.
    let mut resp = Response::ok(ics)?;
    resp.headers_mut().set("Content-Type", "text/calendar; charset=utf-8")?;
    resp.headers_mut().set(
        "Content-Disposition",
        &format!("attachment; filename=\"{}.ics\"",
            zinnias_ciao_contracts::ics::sanitize_filename(&cal_name)),
    )?;
    // Prevent caching of private feed data.
    resp.headers_mut().set("Cache-Control", "no-store, private")?;
    Ok(resp)
}

fn redirect(location: &str) -> Result<Response> {
    let mut resp = Response::from_html("")?;
    resp.headers_mut().set("Location", location)?;
    Ok(resp.with_status(303))
}

// ── Tests ─────────────────────────────────────────────────────────────────

