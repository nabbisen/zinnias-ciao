//! Calendar page for the active community (RFC-056).

use worker::{Env, Request, Response, Result};

use crate::db::{self, event as event_db, membership as membership_db};
use crate::render;
use crate::session::require_auth;
use zinnias_ciao_contracts::{i18n, tz};

pub async fn get_communities(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let db = env.d1("DB")?;

    let summaries = membership_db::list_communities_for_user(&db, &auth.user_id).await?;
    if !summaries.iter().any(|s| s.community_id == community_id) {
        return render::not_found();
    }

    let community = db::community::find_active(&db, community_id).await?;
    let community_tz = community
        .as_ref()
        .map(|c| c.timezone.as_str())
        .unwrap_or("UTC");
    let now_prefix = db::now_utc();
    let tz_offset = tz::offset_minutes_or_utc(community_tz);
    let (today_date, _) = tz::to_local_parts(&now_prefix, tz_offset);
    let (month_start, next_month_start) = month_bounds(&today_date);
    let rows =
        event_db::calendar_month_for_community(&db, community_id, &month_start, &next_month_start)
            .await?;
    let calendar = super::home::render_month_calendar(&today_date, &rows);
    let event_list = render_calendar_events(community_id, community_tz, &rows);

    // Header uses list_communities_for_user result as switcher pairs.
    let community_pairs: Vec<(String, String)> = summaries
        .iter()
        .map(|s| (s.community_id.clone(), s.community_name.clone()))
        .collect();

    let nav = render::bottom_nav(community_id, "communities");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           {calendar}{event_list}\
         </main>{nav}",
        header = render::header_with_switcher_next(
            i18n::JA_NAV_COMMUNITIES,
            community_id,
            &community_pairs,
            "communities"
        ),
        calendar = calendar,
        event_list = event_list,
        nav = nav,
    );
    render::page(i18n::JA_NAV_COMMUNITIES, &body)
}

fn month_bounds(today_date: &str) -> (String, String) {
    let year = today_date
        .get(..4)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1970);
    let month = today_date
        .get(5..7)
        .and_then(|s| s.parse::<i32>().ok())
        .filter(|m| (1..=12).contains(m))
        .unwrap_or(1);
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    (
        format!("{year:04}-{month:02}-01"),
        format!("{next_year:04}-{next_month:02}-01"),
    )
}

fn render_calendar_events(
    community_id: &str,
    community_tz: &str,
    rows: &[event_db::HomeEventRow],
) -> String {
    let items: String = rows
        .iter()
        .map(|row| {
            let date = render::format_day_time_tz(
                &render::CardDay {
                    starts_at_utc: &row.starts_at_utc,
                    ends_at_utc: &row.ends_at_utc,
                    day_date: &row.day_date,
                },
                community_tz,
            );
            let cancelled = if row.event_status == "cancelled" {
                format!(
                    "<span style=\"font-size:.75rem;color:#B42318;margin-left:.35rem\">{}</span>",
                    i18n::JA_EVENT_CANCELLED_BADGE
                )
            } else {
                String::new()
            };
            let location = row.event_location.as_deref().unwrap_or("");
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
                cid = render::escape_html(community_id),
                eid = render::escape_html(&row.event_id),
                title = render::escape_html(&row.event_title),
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

    format!(
        "<section style=\"margin:0 auto 1.5rem;max-width:42rem\">\
         <h2 style=\"font-size:1.125rem;font-weight:700;margin:0\">{title}</h2>\
         {content}</section>",
        title = i18n::JA_HOME_AGENDA_TITLE,
        content = content
    )
}
