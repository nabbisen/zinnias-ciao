//! Home handler — multi-community nearby-events dashboard (RFC-005, RFC-056).

use worker::{Env, Request, Response, Result};

use crate::authz::require_membership;
use crate::db::{self, event as event_db, membership as membership_db};
use crate::render;
use zinnias_ciao_contracts::i18n;

pub async fn redirect_to_home(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, crate::render::session_expired());

    // RFC-081 §2.1a / Handoff 048 §7.4: a community-bound session redirects
    // straight to its own granting community — never enumerates every
    // membership. Enumerating and picking `[0]` could redirect to a
    // *different* community than the one that granted the session, and the
    // resulting URL (even if the next request refuses it) would still name
    // a community this session has no business revealing.
    let cid = if let Some(scope) = auth.scope_community_id.clone() {
        scope
    } else {
        let db = env.d1("DB")?;
        let memberships = membership_db::list_active_for_user(&db, &auth.user_id).await?;
        if memberships.is_empty() {
            return render::session_expired();
        }
        // Use the first community as default; M3+ will add a selected-community cookie.
        memberships[0].community_id.clone()
    };
    let mut resp = Response::empty()?;
    resp.headers_mut()
        .set("Location", &format!("/c/{cid}/home"))?;
    Ok(resp.with_status(303))
}

pub async fn get_home(req: Request, env: &Env, rid: &str, community_id: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_membership(env, &auth, community_id, rid).await?;
    let locale = membership.locale;
    let db = env.d1("DB")?;

    // Home window: today through 30 days ahead
    let from_utc = db::now_utc();
    let to_utc = db::utc_days_ahead(30);

    // RFC-081 §2.1a / Handoff 049: `list_communities_for_user` is now
    // scope-filtered at the source (Handoff 049 §4.2), so a single
    // unconditional call already returns just the granting community for a
    // bound session — the hand-built single-element summary from Handoff
    // 048 is gone. `list_active_for_user` (used only for the `is_first_run`
    // count below) has no scope parameter — that is out of this slice's
    // §4.2, which named `list_communities_for_user` specifically — so
    // calling it unfiltered would leak cross-community membership *count*
    // through the first-run-vs-no-events wording choice. It is still
    // called only for unscoped sessions; a bound session's count is
    // definitionally 1 (`require_membership` above already proved exactly
    // this one community), so no query is needed for that case at all.
    let community_summaries = membership_db::list_communities_for_user(
        &db,
        &auth.user_id,
        auth.scope_community_id.as_deref(),
    )
    .await?;
    let membership_count = if auth.scope_community_id.is_some() {
        1
    } else {
        membership_db::list_active_for_user(&db, &auth.user_id)
            .await?
            .len()
    };
    let community_ids: Vec<&str> = community_summaries
        .iter()
        .map(|c| c.community_id.as_str())
        .collect();
    let rows =
        event_db::home_upcoming_for_communities(&db, &community_ids, &from_utc, &to_utc).await?;

    let nav = render::bottom_nav_localized(community_id, "home", locale);

    // ── Admin first-run card (RFC-030) ────────────────────────────────
    // When admin lands on empty Home, show an actionable setup guide
    // instead of a plain text paragraph. Detect first-run by member count.
    let is_first_run = rows.is_empty()
        && membership.is_admin()
        && membership_count == 1
        && community_summaries.len() == 1;
    let (empty_html, admin_shortcuts): (String, String) =
        if rows.is_empty() && membership.is_admin() {
            let intro = i18n::t(
                locale,
                if is_first_run {
                    i18n::HOME_FIRST_RUN_WELCOME
                } else {
                    i18n::HOME_FIRST_RUN_NO_EVENTS
                },
            );
            let invite_hint = if is_first_run {
                format!(
                    "<p class=\"cz-hint cz-home-hint--gap-top\">\
                     {}</p>",
                    i18n::t(locale, i18n::HOME_FIRST_RUN_INVITE_HINT)
                )
            } else {
                String::new()
            };
            let card = format!(
                "<div class=\"cz-home-first-run-card\">\
             <p class=\"cz-home-first-run-intro\">{intro}</p>\
             <div class=\"cz-home-first-run-actions\">\
               <a href=\"/c/{cid}/admin/events/new\" \
                  class=\"cz-home-first-run-link cz-home-first-run-link--primary\">\
                  {create_label}</a>\
               <a href=\"/c/{cid}/admin/members\" \
                  class=\"cz-home-first-run-link cz-home-first-run-link--secondary\">\
                  {invite_label}</a>\
             </div>\
             {hint}\
             </div>",
                intro = intro,
                cid = render::escape_html(community_id),
                create_label = i18n::t(locale, i18n::HOME_FIRST_RUN_CREATE),
                invite_label = i18n::t(locale, i18n::HOME_MANAGE_MEMBERS),
                hint = invite_hint,
            );
            (card, String::new())
        } else if rows.is_empty() {
            // Member empty state
            let msg = format!(
                "<p class=\"cz-home-empty-member\">{}</p>",
                i18n::t(locale, i18n::EMPTY_EVENTS_HINT)
            );
            (msg, String::new())
        } else {
            // Events exist: show persistent admin shortcuts
            let shortcuts = if membership.is_admin() {
                format!(
                    "<div class=\"cz-home-shortcuts-row\">\
                   <a href=\"/c/{cid}/admin/events/new\" \
                      class=\"cz-home-shortcut-link cz-home-shortcut-link--primary\">\
                      {create_label}</a>\
                   <a href=\"/c/{cid}/admin/members\" \
                      class=\"cz-home-shortcut-link cz-home-shortcut-link--secondary\">\
                      {invite_label}</a>\
                 </div>",
                    cid = render::escape_html(community_id),
                    create_label = i18n::t(locale, i18n::HOME_CREATE_EVENT),
                    invite_label = i18n::t(locale, i18n::HOME_MANAGE_MEMBERS),
                )
            } else {
                String::new()
            };
            (String::new(), shortcuts)
        };

    let community_sections = render_home_communities(&community_summaries, &rows, locale);

    let title = i18n::t(locale, i18n::NAV_HOME);
    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
           {sections}{empty}\
           {shortcuts}\
         </main>\
         {nav}",
        header = render::header(title, ""),
        sections = community_sections,
        shortcuts = admin_shortcuts,
        empty = empty_html,
        nav = nav,
    );
    render::page_localized(locale, title, &body)
}

fn render_home_communities(
    communities: &[membership_db::CommunitySummary],
    rows: &[event_db::HomeEventRow],
    locale: zinnias_ciao_contracts::Locale,
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
                let date = render::format_day_time_tz_localized(
                    &render::CardDay {
                        starts_at_utc: &r.starts_at_utc,
                        ends_at_utc: &r.ends_at_utc,
                        day_date: &r.day_date,
                    },
                    &community.timezone,
                    locale,
                );
                let cancelled =
                    if r.event_status == "cancelled" || r.occurrence_status == "cancelled" {
                        format!(
                            "<span class=\"cz-event-cancelled-badge\">{}</span>",
                            i18n::t(
                                locale,
                                if r.occurrence_status == "cancelled" {
                                    i18n::OCCURRENCE_CANCELLED_BADGE
                                } else {
                                    i18n::EVENT_CANCELLED_BADGE
                                }
                            )
                        )
                    } else {
                        String::new()
                    };
                let location = r.event_location.as_deref().unwrap_or("");
                let location_html = if location.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<span class=\"cz-home-event-location\"> · {}</span>",
                        render::escape_html(location)
                    )
                };
                format!(
                    "<li class=\"cz-event-list-item\">\
                     <a href=\"/c/{cid}/events/{eid}\" class=\"cz-event-link\">\
                     <span class=\"cz-event-title\">{title}{cancelled}</span>\
                     <span class=\"cz-event-meta\">{date}{location}</span>\
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
                "<p class=\"cz-hint cz-hint--gap-top\">{}</p>",
                i18n::t(locale, i18n::HOME_CALENDAR_EMPTY)
            )
        } else {
            format!("<ul class=\"cz-event-list\">{items}</ul>")
        };
        html.push_str(&format!(
            "<section class=\"cz-home-community-section\">\
             <h2 class=\"cz-section-title\">{name}</h2>\
             {content}</section>",
            name = render::escape_html(&community.community_name),
            content = content
        ));
    }
    html
}

#[cfg(test)]
mod tests;
