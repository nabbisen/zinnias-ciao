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
use crate::render;

// ── GET /c/:cid/me/calendar ───────────────────────────────────────────────

pub async fn get_me_calendar(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_membership(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env)?;

    let regen_token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::CALENDAR_REGENERATE, None)
            .await?;
    let revoke_token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::CALENDAR_REVOKE, None)
            .await?;

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
                "<p role=\"status\" class=\"cz-note-flash\">{}</p>",
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
            hmac = render::escape_html(&hmac_hex(pp.as_str(), &tok.id)),
        );
        format!(
            "<div class=\"cz-calendar-feed-card\">\
             <p class=\"cz-calendar-feed-note\">\
               {privacy_note}\
             </p>\
             <div class=\"cz-calendar-feed-url\">{feed_url}</div>\
             <form method=\"post\" action=\"/c/{cid}/me/calendar/revoke\" \
               class=\"cz-calendar-feed-form--inline cz-calendar-feed-form--inline-gap\">\
               <input type=\"hidden\" name=\"_token\" value=\"{rtok}\">\
               <button type=\"submit\" \
                 class=\"cz-calendar-feed-action cz-calendar-feed-action--danger\">\
                 {disable}\
               </button>\
             </form>\
             <form method=\"post\" action=\"/c/{cid}/me/calendar/regenerate\" \
               class=\"cz-calendar-feed-form--inline\">\
               <input type=\"hidden\" name=\"_token\" value=\"{gentok}\">\
               <button type=\"submit\" \
                 class=\"cz-calendar-feed-action cz-calendar-feed-action--neutral\">\
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
            "<p class=\"cz-calendar-feed-description\">{desc}</p>\
             <form method=\"post\" action=\"/c/{cid}/me/calendar/regenerate\">\
               <input type=\"hidden\" name=\"_token\" value=\"{gentok}\">\
               <button type=\"submit\" \
                 class=\"cz-calendar-feed-generate-button\">{cg}</button>\
             </form>",
            cid = render::escape_html(community_id),
            gentok = render::escape_html(&regen_token),
            cg = i18n::JA_CALENDAR_GENERATE,
            desc = i18n::JA_CALENDAR_DESCRIPTION,
        )
    };

    let nav = render::bottom_nav(community_id, "me");
    let back = format!(
        "<a href=\"/c/{}/me\" class=\"cz-event-back-link\">\u{2190} {}</a>",
        render::escape_html(community_id),
        i18n::JA_NAV_ME,
    );
    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
         {back}\
         <h1 class=\"cz-event-title-heading\">{cal_title}</h1>\
         <p class=\"cz-calendar-feed-page-desc\">\
           {cal_desc}\
         </p>\
         {flash}\
         {feed}\
         </main>{nav}",
        header = render::header_with_switcher_next(
            i18n::JA_CALENDAR_TITLE,
            community_id,
            &community_pairs,
            "calendar_feed"
        ),
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
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_membership(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env)?;

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
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/me/calendar"));
    }

    let now = db::now_utc();
    // Generate new token — the ID is stored; HMAC(pepper, id) is the bearer secret.
    let token_id = random_token()[..32].to_owned();
    let token_hmac = hmac_hex(pp.as_str(), &token_id);
    cal_db::rotate_required(
        &db,
        rid,
        &token_id,
        community_id,
        &membership.membership_id,
        &token_hmac,
        &now,
    )
    .await?;

    redirect(&format!("/c/{community_id}/me/calendar?flash=generated"))
}

// ── POST /c/:cid/me/calendar/revoke ──────────────────────────────────────

pub async fn post_revoke_calendar(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
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
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/me/calendar"));
    }

    let now = db::now_utc();
    cal_db::revoke_required(&db, rid, community_id, &membership.membership_id, &now).await?;

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
