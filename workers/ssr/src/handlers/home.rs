//! Home handler — upcoming event list (RFC-005).

use worker::{Env, Request, Response, Result};

use crate::authz::require_membership;
use zinnias_ciao_contracts::i18n;
use crate::db::{self, event as event_db, attendance as attendance_db, membership as membership_db};
use crate::render;
use crate::session::require_auth;

pub async fn redirect_to_home(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
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
    resp.headers_mut().set("Location", &format!("/c/{cid}/home"))?;
    Ok(resp.with_status(303))
}

pub async fn get_home(req: Request, env: &Env, _rid: &str, community_id: &str) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    // Home window: today through 30 days ahead
    let from_utc = db::now_utc();
    let to_utc   = db::utc_days_ahead(30);
    let rows = event_db::home_upcoming(&db, community_id, &from_utc, &to_utc).await?;

    // Active member count for no_answer calculation
    let _all_members = membership_db::list_active_for_user(&db, &auth.user_id).await?;
    // Use a rough count; accurate count is per community — we fetch it here
    let member_count = membership_db::count_active(&db, community_id).await?;

    // Fetch community (name + timezone) before the event loop (needed for time display).
    let community = db::community::find_active(&db, community_id).await?;
    let _community_name = community.as_ref().map(|c| c.name.as_str()).unwrap_or_default();
    let community_tz   = community.as_ref().map(|c| c.timezone.as_str()).unwrap_or("UTC");

    // Collect my attendances for the listed days in one query
    let my_attendances = attendance_db::list_mine_for_days(
        &db,
        &membership.membership_id,
        &rows.iter().map(|r| r.day_id.as_str()).collect::<Vec<_>>(),
    ).await?;

    // Build cards grouped by section: Today / This Week / Later
    let now_prefix = db::now_utc();
    let today_date = &now_prefix[..10]; // "2026-06-12"

    let mut today_cards   = String::new();
    let mut thisweek_cards = String::new();
    let mut later_cards   = String::new();

    // Batch-fetch all day counts in a single query (RFC-029: no N+1).
    let all_day_ids: Vec<&str> = rows.iter().map(|r| r.day_id.as_str()).collect();
    let all_counts = attendance_db::counts_for_days(&db, &all_day_ids, member_count).await?;

    // Deduplicate by event_id (home query returns one row per day; take nearest day per event)
    let mut seen_events: std::collections::HashSet<String> = std::collections::HashSet::new();

    for row in &rows {
        if seen_events.contains(&row.event_id) { continue; }
        seen_events.insert(row.event_id.clone());

        let my_status = my_attendances.get(&row.day_id).map(|s| s.as_str());
        let empty_counts = attendance_db::DayCountRow {
            going: 0, not_going: 0, attended: 0, no_answer: member_count
        };
        let counts = all_counts.get(&row.day_id).unwrap_or(&empty_counts);

        let card = render::event_card(
            community_id,
            &row.event_id,
            &row.event_title,
            row.event_location.as_deref(),
            row.event_status == "cancelled",
            &render::CardDay {
                starts_at_utc: &row.starts_at_utc,
                ends_at_utc:   &row.ends_at_utc,
                day_date:      &row.day_date,
            },
            row.total_days,
            my_status,
            counts.going, counts.not_going, counts.no_answer,
            community_tz,
        );

        if row.day_date == today_date {
            today_cards.push_str(&card);
        } else if &row.day_date[..10] <= &db::utc_days_ahead(7)[..10] {
            thisweek_cards.push_str(&card);
        } else {
            later_cards.push_str(&card);
        }
    }

    let section = |label: &str, cards: &str| -> String {
        if cards.is_empty() { return String::new(); }
        format!(
            "<section style=\"margin-bottom:1.5rem\">\
             <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
             text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">\
             {label}</h2>{cards}</section>"
        )
    };

    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();

    let nav = render::bottom_nav(community_id, "home");

    // ── Admin first-run card (RFC-030) ────────────────────────────────
    // When admin lands on empty Home, show an actionable setup guide
    // instead of a plain text paragraph. Detect first-run by member count.
    let is_first_run = seen_events.is_empty() && membership.is_admin() && member_count <= 1;
    let (empty_html, admin_shortcuts): (String, String) = if seen_events.is_empty() && membership.is_admin() {
        let intro = if is_first_run {
            i18n::EN_HOME_FIRST_RUN_WELCOME
        } else {
            i18n::EN_HOME_FIRST_RUN_NO_EVENTS
        };
        let invite_hint = if is_first_run {
            format!(
                "<p style=\"font-size:.875rem;color:#6e6e73;margin:.5rem 0 0\">\
                 Invite members so they can see your events.</p>"
            )
        } else { String::new() };
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
                  + Create first event</a>\
               <a href=\"/c/{cid}/admin/invites\" \
                  style=\"display:flex;align-items:center;justify-content:center;\
                  padding:.875rem;background:#fff;color:#007AFF;\
                  border:2px solid #007AFF;border-radius:14px;font-size:1rem;font-weight:600;\
                  text-align:center;text-decoration:none;min-height:44px\">\
                  Invite members</a>\
             </div>\
             {hint}\
             </div>",
            intro = intro,
            cid   = render::escape_html(community_id),
            hint  = invite_hint,
        );
        (card, String::new())
    } else if seen_events.is_empty() {
        // Member empty state
        let msg = format!(
            "<p style=\"color:#6e6e73;padding:2rem 0\">{}</p>",
            i18n::EN_EMPTY_EVENTS_HINT
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
                      + Create event</a>\
                   <a href=\"/c/{cid}/admin/invites\" \
                      style=\"flex:1;padding:.75rem;background:#F5F5F7;color:#1D1D1F;\
                      border-radius:14px;font-size:.9375rem;font-weight:600;\
                      text-align:center;text-decoration:none;min-height:44px;\
                      display:flex;align-items:center;justify-content:center\">\
                      Invite members</a>\
                 </div>",
                cid = render::escape_html(community_id),
            )
        } else { String::new() };
        (String::new(), shortcuts)
    };

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           {shortcuts}\
           {today}{thisweek}{later}{empty}\
         </main>\
         {nav}",
        header    = render::header_with_switcher("Home", community_id, &_community_pairs),
        shortcuts = admin_shortcuts,
        today    = section(i18n::EN_HOME_TODAY, &today_cards),
        thisweek = section(i18n::EN_HOME_THIS_WEEK, &thisweek_cards),
        later    = section(i18n::EN_HOME_LATER, &later_cards),
        empty    = empty_html,
        nav      = nav,
    );
    render::page("Home", &body)
}
