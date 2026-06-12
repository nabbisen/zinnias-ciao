//! Home handler — upcoming event list (RFC-005).

use worker::{Env, Request, Response, Result};

use crate::authz::require_membership;
use crate::db::{self, event as event_db, attendance as attendance_db, membership as membership_db};
use crate::render;
use crate::session::require_auth;

pub async fn redirect_to_home(req: Request, env: &Env, rid: &str) -> Result<Response> {
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

pub async fn get_home(req: Request, env: &Env, rid: &str, community_id: &str) -> Result<Response> {
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
    let all_members = membership_db::list_active_for_user(&db, &auth.user_id).await?;
    // Use a rough count; accurate count is per community — we fetch it here
    let member_count = membership_db::count_active(&db, community_id).await?;

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

    // Deduplicate by event_id (home query returns one row per day; take nearest day per event)
    let mut seen_events: std::collections::HashSet<String> = std::collections::HashSet::new();

    for row in &rows {
        if seen_events.contains(&row.event_id) { continue; }
        seen_events.insert(row.event_id.clone());

        let my_status = my_attendances.get(&row.day_id).map(|s| s.as_str());
        let counts = attendance_db::counts_for_day(&db, &row.day_id, member_count).await?;

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

    let empty_html = if seen_events.is_empty() {
        if membership.is_admin() {
            "<p style=\"color:#6e6e73;padding:2rem 0\">No events yet. Create the first event for this community.</p>"
        } else {
            "<p style=\"color:#6e6e73;padding:2rem 0\">No events yet. Ask your community admin to add one.</p>"
        }
    } else { "" };

    // Fetch community name for the header
    let community = db::community::find_active(&db, community_id).await?;
    let community_name = community.map(|c| c.name).unwrap_or_default();

    let nav = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           {today}{thisweek}{later}{empty}\
         </main>\
         {nav}",
        header   = render::header("Home", &community_name),
        today    = section("Today", &today_cards),
        thisweek = section("This Week", &thisweek_cards),
        later    = section("Later", &later_cards),
        empty    = empty_html,
        nav      = nav,
    );
    render::page("Home", &body)
}
