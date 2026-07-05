//! Calendar export handlers (RFC-023).
//!
//! Four routes:
//!   GET  /c/:cid/me/calendar              — Me calendar page (show/generate feed URL)
//!   POST /c/:cid/me/calendar/regenerate   — generate or rotate feed token
//!   POST /c/:cid/me/calendar/revoke       — revoke (disable) feed
//!   GET  /c/:cid/cal/:token               — unauthenticated ICS feed (bearer URL)

use worker::{D1Database, Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz::require_membership;
use crate::crypto::{hmac_hex, random_token};
use crate::db::{self, calendar as cal_db};
use crate::render;
use crate::session::require_auth;

// ── GET /c/:cid/me/calendar ───────────────────────────────────────────────

pub async fn get_me_calendar(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let regen_token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::CALENDAR_REGENERATE, None)
            .await;
    let revoke_token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::CALENDAR_REVOKE, None).await;

    let active = cal_db::find_active_for_membership(&db, &membership.membership_id, community_id)
        .await
        .unwrap_or(None);

    let communities_for_switcher =
        crate::db::membership::list_communities_for_user(&db, &auth.user_id)
            .await
            .unwrap_or_default();
    let community_pairs: Vec<(String, String)> = communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();

    // Build the feed URL from the request URL origin.
    let origin = {
        let url = req.url()?;
        let host = url.host_str().unwrap_or("localhost");
        let host_with_port = match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_owned(),
        };
        format!("{}://{}", url.scheme(), host_with_port)
    };

    let url = req.url()?;
    let flash_code: Option<String> = url
        .query_pairs()
        .find(|(k, _)| k == "flash")
        .map(|(_, v)| v.to_string());
    let flash_html = calendar_flash_message(flash_code.as_deref())
        .map(|message| {
            format!(
                "<p role=\"status\" style=\"font-size:.875rem;color:#167A34;margin:.5rem 0\">{}</p>",
                render::escape_html(message)
            )
        })
        .unwrap_or_default();

    let feed_section = if let Some(ref tok) = active {
        // The URL-visible bearer is HMAC(pepper, token id). The application
        // stores and looks up only that HMAC; audit metadata never receives it.
        let feed_url = format!(
            "{origin}/c/{cid}/cal/{hmac}",
            cid = render::escape_html(community_id),
            hmac = render::escape_html(&hmac_hex(&pp, &tok.id)),
        );
        format!(
            "<div style=\"background:#f5f5f7;border-radius:12px;padding:1rem;margin:1rem 0\">\
             <p style=\"font-size:.8125rem;color:#6E6E73;margin:0 0 .5rem\">\
               {privacy_note}\
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
                 {disable}\
               </button>\
             </form>\
             <form method=\"post\" action=\"/c/{cid}/me/calendar/regenerate\" \
               style=\"display:inline\">\
               <input type=\"hidden\" name=\"_token\" value=\"{gentok}\">\
               <button type=\"submit\" \
                 style=\"font-size:.875rem;color:#007AFF;background:none;border:none;\
                 cursor:pointer;padding:.25rem 0;min-height:44px\">\
                 {regenerate}\
               </button>\
             </form>\
             </div>",
            feed_url = render::escape_html(&feed_url),
            cid = render::escape_html(community_id),
            rtok = render::escape_html(&revoke_token),
            gentok = render::escape_html(&regen_token),
            privacy_note = i18n::JA_CALENDAR_PRIVACY_NOTE,
            disable = i18n::JA_CALENDAR_DISABLE,
            regenerate = i18n::JA_CALENDAR_REGENERATE,
        )
    } else {
        format!(
            "<p style=\"font-size:.875rem;color:#6E6E73;margin:1rem 0\">{desc}</p>\
             <form method=\"post\" action=\"/c/{cid}/me/calendar/regenerate\">\
               <input type=\"hidden\" name=\"_token\" value=\"{gentok}\">\
               <button type=\"submit\" \
                 style=\"width:100%;padding:.875rem;background:#007AFF;color:#fff;\
                 border:none;border-radius:14px;font-size:1rem;font-weight:600;\
                 min-height:44px;cursor:pointer\">{cg}</button>\
             </form>",
            cid = render::escape_html(community_id),
            gentok = render::escape_html(&regen_token),
            cg = i18n::JA_CALENDAR_GENERATE,
            desc = i18n::JA_CALENDAR_DESCRIPTION,
        )
    };

    let nav = render::bottom_nav(community_id, "me");
    let back = format!(
        "<a href=\"/c/{}/me\" style=\"color:#007AFF;font-size:.9375rem\">\u{2190} {}</a>",
        render::escape_html(community_id),
        i18n::JA_NAV_ME,
    );
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         {back}\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin:1rem 0 .25rem\">{cal_title}</h1>\
         <p style=\"font-size:.875rem;color:#6E6E73;margin-bottom:1rem\">\
           {cal_desc}\
         </p>\
         {flash}\
         {feed}\
         </main>{nav}",
        header =
            render::header_with_switcher(i18n::JA_CALENDAR_TITLE, community_id, &community_pairs),
        cal_title = i18n::JA_CALENDAR_TITLE,
        cal_desc = i18n::JA_CALENDAR_DESCRIPTION,
        back = back,
        flash = flash_html,
        feed = feed_section,
        nav = nav,
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
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::CALENDAR_REGENERATE,
        &raw_token,
        None,
    )
    .await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/me/calendar"));
    }

    let now = db::now_utc();
    // Revoke any existing token first.
    cal_db::revoke_for_membership(&db, &membership.membership_id, community_id, &now).await?;

    // Generate new token — the ID is stored; HMAC(pepper, id) is the bearer secret.
    let token_id = random_token()[..32].to_owned();
    let token_hmac = hmac_hex(&pp, &token_id);
    cal_db::insert(
        &db,
        &token_id,
        community_id,
        &membership.membership_id,
        &token_hmac,
        &now,
    )
    .await?;

    let _ = write_calendar_token_audit(
        &db,
        rid,
        community_id,
        &membership.membership_id,
        "calendar_token_generated",
    )
    .await;

    redirect(&format!("/c/{community_id}/me/calendar?flash=generated"))
}

// ── POST /c/:cid/me/calendar/revoke ──────────────────────────────────────

pub async fn post_revoke_calendar(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::CALENDAR_REVOKE,
        &raw_token,
        None,
    )
    .await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/me/calendar"));
    }

    let now = db::now_utc();
    cal_db::revoke_for_membership(&db, &membership.membership_id, community_id, &now).await?;

    let _ = write_calendar_token_audit(
        &db,
        rid,
        community_id,
        &membership.membership_id,
        "calendar_token_revoked",
    )
    .await;

    redirect(&format!("/c/{community_id}/me/calendar?flash=disabled"))
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
            return Ok(
                Response::from_html(format!("<p>{}</p>", i18n::JA_NOT_FOUND))?.with_status(404),
            );
        }
    };

    // Verify the membership is still active in this community.
    let still_active =
        crate::db::membership::find_active_by_id(&db, &claims.membership_id, community_id)
            .await?
            .is_some();
    if !still_active {
        return Ok(
            Response::from_html(format!("<p>{}</p>", i18n::JA_GENERAL_ERROR))?.with_status(403),
        );
    }

    // Fetch events.
    let events = cal_db::events_for_feed(&db, community_id).await?;

    // Build ICS.
    let community = crate::db::community::find_active(&db, community_id).await?;
    let cal_name = community
        .map(|c| c.name)
        .unwrap_or_else(|| "Community".to_owned());
    let days: Vec<zinnias_ciao_contracts::ics::IcsDay<'_>> = events
        .iter()
        .map(|ev| zinnias_ciao_contracts::ics::IcsDay {
            uid: &ev.day_id,
            title: &ev.title,
            location: ev.location.as_deref(),
            status: &ev.status,
            starts_at_utc: &ev.starts_at_utc,
            ends_at_utc: &ev.ends_at_utc,
        })
        .collect();
    let ics = zinnias_ciao_contracts::ics::build_vcalendar(&cal_name, &days);

    // Return as text/calendar.
    let mut resp = Response::ok(ics)?;
    resp.headers_mut()
        .set("Content-Type", "text/calendar; charset=utf-8")?;
    resp.headers_mut().set(
        "Content-Disposition",
        &format!(
            "attachment; filename=\"{}.ics\"",
            zinnias_ciao_contracts::ics::sanitize_filename(&cal_name)
        ),
    )?;
    // Prevent caching of private feed data.
    resp.headers_mut()
        .set("Cache-Control", "no-store, private")?;
    resp.headers_mut().set("Referrer-Policy", "no-referrer")?;
    resp.headers_mut()
        .set("X-Content-Type-Options", "nosniff")?;
    Ok(resp)
}

async fn write_calendar_token_audit(
    db: &D1Database,
    rid: &str,
    community_id: &str,
    membership_id: &str,
    action: &str,
) -> Result<()> {
    // Security-relevant audit event (RFC-045 P1-5). Keep token ids, HMACs,
    // and bearer URLs out of both target_id and metadata.
    let target_id: Option<&str> = None;
    let metadata: Option<serde_json::Value> = None;
    crate::audit::write(
        db,
        rid,
        Some(community_id),
        Some(membership_id),
        "calendar_feed",
        target_id,
        action,
        metadata,
    )
    .await
}

fn redirect(location: &str) -> Result<Response> {
    let mut resp = Response::from_html("")?;
    resp.headers_mut().set("Location", location)?;
    Ok(resp.with_status(303))
}

fn calendar_flash_message(code: Option<&str>) -> Option<&'static str> {
    match code {
        Some("generated") => Some(i18n::JA_CALENDAR_GENERATED_FLASH),
        Some("disabled") => Some(i18n::JA_CALENDAR_REVOKED_FLASH),
        _ => None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
