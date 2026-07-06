//! Home handler — multi-community nearby-events dashboard (RFC-005, RFC-056).

use worker::{Env, Request, Response, Result};

use crate::authz::require_membership;
use crate::db::{self, event as event_db, membership as membership_db};
use crate::render;
use crate::session::require_auth;
use zinnias_ciao_contracts::i18n;

pub async fn redirect_to_home(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return crate::render::session_expired(),
    };
    let db = env.d1("DB")?;
    let memberships = membership_db::list_active_for_user(&db, &auth.user_id).await?;
    if memberships.is_empty() {
        return render::session_expired();
    }
    // Use the first community as default; M3+ will add a selected-community cookie.
    let cid = &memberships[0].community_id;
    let mut resp = Response::empty()?;
    resp.headers_mut()
        .set("Location", &format!("/c/{cid}/home"))?;
    Ok(resp.with_status(303))
}

pub async fn get_home(req: Request, env: &Env, _rid: &str, community_id: &str) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    // Home window: today through 30 days ahead
    let from_utc = db::now_utc();
    let to_utc = db::utc_days_ahead(30);
    let memberships = membership_db::list_active_for_user(&db, &auth.user_id).await?;
    let community_summaries = membership_db::list_communities_for_user(&db, &auth.user_id).await?;
    let community_ids: Vec<&str> = community_summaries
        .iter()
        .map(|c| c.community_id.as_str())
        .collect();
    let rows =
        event_db::home_upcoming_for_communities(&db, &community_ids, &from_utc, &to_utc).await?;

    let nav = render::bottom_nav(community_id, "home");

    // ── Admin first-run card (RFC-030) ────────────────────────────────
    // When admin lands on empty Home, show an actionable setup guide
    // instead of a plain text paragraph. Detect first-run by member count.
    let is_first_run = rows.is_empty()
        && membership.is_admin()
        && memberships.len() == 1
        && community_summaries.len() == 1;
    let (empty_html, admin_shortcuts): (String, String) =
        if rows.is_empty() && membership.is_admin() {
            let intro = if is_first_run {
                i18n::JA_HOME_FIRST_RUN_WELCOME
            } else {
                i18n::JA_HOME_FIRST_RUN_NO_EVENTS
            };
            let invite_hint = if is_first_run {
                format!(
                    "<p style=\"font-size:.875rem;color:#6e6e73;margin:.5rem 0 0\">\
                     {}</p>",
                    i18n::JA_HOME_FIRST_RUN_INVITE_HINT
                )
            } else {
                String::new()
            };
            let card = format!(
                "<div style=\"background:#F5F5F7;border-radius:16px;padding:1.25rem;\
             margin-bottom:1.5rem\">\
             <p style=\"font-size:.9375rem;color:#6e6e73;margin:0 0 1rem\">{intro}</p>\
             <div style=\"display:flex;gap:.75rem;flex-direction:column\">\
               <a href=\"/c/{cid}/admin/events/new\" \
                  style=\"display:flex;align-items:center;justify-content:center;\
                  padding:.875rem;background:#007AFF;color:#fff;\
                  border-radius:14px;font-size:1rem;font-weight:600;\
                  text-align:center;text-decoration:none;min-height:44px\">\
                  {create_label}</a>\
               <a href=\"/c/{cid}/admin/members\" \
                  style=\"display:flex;align-items:center;justify-content:center;\
                  padding:.875rem;background:#fff;color:#007AFF;\
                  border:2px solid #007AFF;border-radius:14px;font-size:1rem;font-weight:600;\
                  text-align:center;text-decoration:none;min-height:44px\">\
                  {invite_label}</a>\
             </div>\
             {hint}\
             </div>",
                intro = intro,
                cid = render::escape_html(community_id),
                create_label = i18n::JA_HOME_FIRST_RUN_CREATE,
                invite_label = i18n::JA_HOME_MANAGE_MEMBERS,
                hint = invite_hint,
            );
            (card, String::new())
        } else if rows.is_empty() {
            // Member empty state
            let msg = format!(
                "<p style=\"color:#6e6e73;padding:2rem 0\">{}</p>",
                i18n::JA_EMPTY_EVENTS_HINT
            );
            (msg, String::new())
        } else {
            // Events exist: show persistent admin shortcuts
            let shortcuts = if membership.is_admin() {
                format!(
                    "<div style=\"display:flex;gap:.75rem;margin-bottom:1.25rem\">\
                   <a href=\"/c/{cid}/admin/events/new\" \
                      style=\"flex:1;padding:.75rem;background:#007AFF;color:#fff;\
                      border-radius:14px;font-size:.9375rem;font-weight:600;\
                      text-align:center;text-decoration:none;min-height:44px;\
                      display:flex;align-items:center;justify-content:center\">\
                      {create_label}</a>\
                   <a href=\"/c/{cid}/admin/members\" \
                      style=\"flex:1;padding:.75rem;background:#F5F5F7;color:#1D1D1F;\
                      border-radius:14px;font-size:.9375rem;font-weight:600;\
                      text-align:center;text-decoration:none;min-height:44px;\
                      display:flex;align-items:center;justify-content:center\">\
                      {invite_label}</a>\
                 </div>",
                    cid = render::escape_html(community_id),
                    create_label = i18n::JA_HOME_CREATE_EVENT,
                    invite_label = i18n::JA_HOME_MANAGE_MEMBERS,
                )
            } else {
                String::new()
            };
            (String::new(), shortcuts)
        };

    let community_sections = render_home_communities(&community_summaries, &rows);

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           {sections}{empty}\
           {shortcuts}\
         </main>\
         {nav}",
        header = render::header(i18n::JA_NAV_HOME, ""),
        sections = community_sections,
        shortcuts = admin_shortcuts,
        empty = empty_html,
        nav = nav,
    );
    render::page(i18n::JA_NAV_HOME, &body)
}

fn render_home_communities(
    communities: &[membership_db::CommunitySummary],
    rows: &[event_db::HomeEventRow],
) -> String {
    let mut html = String::new();
    for community in communities {
        let mut seen = std::collections::HashSet::new();
        let items: String = rows
            .iter()
            .filter(|r| r.community_id == community.community_id)
            .filter(|r| seen.insert(r.event_id.clone()))
            .take(4)
            .map(|r| {
                let date = render::format_day_time_tz(
                    &render::CardDay {
                        starts_at_utc: &r.starts_at_utc,
                        ends_at_utc: &r.ends_at_utc,
                        day_date: &r.day_date,
                    },
                    &community.timezone,
                );
                let cancelled = if r.event_status == "cancelled" {
                    format!(
                        "<span style=\"font-size:.75rem;color:#B42318;margin-left:.35rem\">{}</span>",
                        i18n::JA_EVENT_CANCELLED_BADGE
                    )
                } else {
                    String::new()
                };
                let location = r.event_location.as_deref().unwrap_or("");
                let location_html = if location.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<span style=\"color:#6e6e73\"> · {}</span>",
                        render::escape_html(location)
                    )
                };
                format!(
                    "<li style=\"border-top:1px solid #F5F5F7\">\
                     <a href=\"/c/{cid}/events/{eid}\" style=\"display:block;\
                     padding:.875rem 0;text-decoration:none;color:inherit\">\
                     <span style=\"display:block;font-size:1rem;font-weight:600;\
                     line-height:1.35\">{title}{cancelled}</span>\
                     <span style=\"display:block;font-size:.8125rem;color:#6e6e73;\
                     margin-top:.25rem\">{date}{location}</span>\
                     </a></li>",
                    cid = render::escape_html(&community.community_id),
                    eid = render::escape_html(&r.event_id),
                    title = render::escape_html(&r.event_title),
                    cancelled = cancelled,
                    date = render::escape_html(&date),
                    location = location_html,
                )
            })
            .collect();
        let content = if items.is_empty() {
            format!(
                "<p style=\"font-size:.875rem;color:#6e6e73;margin:.75rem 0 0\">{}</p>",
                i18n::JA_HOME_CALENDAR_EMPTY
            )
        } else {
            format!("<ul style=\"list-style:none;margin:.5rem 0 0;padding:0\">{items}</ul>")
        };
        html.push_str(&format!(
            "<section style=\"margin:0 0 1.5rem\">\
             <h2 style=\"font-size:1.125rem;font-weight:700;margin:0\">{name}</h2>\
             {content}</section>",
            name = render::escape_html(&community.community_name),
            content = content
        ));
    }
    html
}
