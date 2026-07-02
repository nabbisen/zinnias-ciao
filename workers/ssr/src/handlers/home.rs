//! Home handler — calendar-centered dashboard (RFC-005, RFC-056).

use std::collections::BTreeMap;

use worker::{Env, Request, Response, Result};

use crate::authz::require_membership;
use crate::db::{self, event as event_db, membership as membership_db};
use crate::render;
use crate::session::require_auth;
use zinnias_ciao_contracts::{i18n, tz};

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
               <a href=\"/c/{cid}/admin/invites\" \
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
                invite_label = i18n::JA_HOME_INVITE_MEMBERS,
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
                   <a href=\"/c/{cid}/admin/invites\" \
                      style=\"flex:1;padding:.75rem;background:#F5F5F7;color:#1D1D1F;\
                      border-radius:14px;font-size:.9375rem;font-weight:600;\
                      text-align:center;text-decoration:none;min-height:44px;\
                      display:flex;align-items:center;justify-content:center\">\
                      {invite_label}</a>\
                 </div>",
                    cid = render::escape_html(community_id),
                    create_label = i18n::JA_HOME_CREATE_EVENT,
                    invite_label = i18n::JA_HOME_INVITE_MEMBERS,
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

pub(crate) fn render_month_calendar(today_date: &str, rows: &[event_db::HomeEventRow]) -> String {
    let Some((year, month, today)) = parse_ymd(today_date) else {
        return String::new();
    };

    let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
    for row in rows {
        let Some((row_year, row_month, row_day)) = parse_ymd(&row.day_date) else {
            continue;
        };
        if row_year == year && row_month == month {
            *counts.entry(row_day).or_default() += 1;
        }
    }

    let weekdays = ["日", "月", "火", "水", "木", "金", "土"];
    let mut cells = String::new();
    for label in weekdays {
        cells.push_str(&format!(
            "<div style=\"min-height:28px;display:flex;align-items:center;\
             justify-content:center;font-size:.75rem;font-weight:700;color:#6e6e73\">\
             {label}</div>"
        ));
    }

    for _ in 0..weekday_sunday_zero(year, month, 1) {
        cells.push_str(
            "<div aria-hidden=\"true\" style=\"min-height:54px;border-radius:10px\"></div>",
        );
    }

    let days_in_month = tz::days_in_month(year, month);
    for day in 1..=days_in_month {
        let count = counts.get(&day).copied().unwrap_or_default();
        let is_today = day == today;
        let has_events = count > 0;
        let bg = if is_today {
            "#EAF3FF"
        } else if has_events {
            "#FFFFFF"
        } else {
            "#F5F5F7"
        };
        let border = if is_today {
            "#007AFF"
        } else if has_events {
            "#D1D1D6"
        } else {
            "#F5F5F7"
        };
        let day_color = if is_today { "#0057B8" } else { "#1D1D1F" };
        let marker_html = match (is_today, has_events) {
            (true, true) => "<span style=\"display:flex;gap:.125rem;align-items:center;\
                 justify-content:center;font-size:.6875rem;font-weight:700;\
                 color:#0057B8;line-height:1.2\">\
                 <span>今日</span><span aria-hidden=\"true\">●</span></span>"
                .to_string(),
            (true, false) => "<span style=\"font-size:.6875rem;font-weight:700;color:#0057B8;\
                 line-height:1.2\">今日</span>"
                .to_string(),
            (false, true) => {
                "<span aria-hidden=\"true\" style=\"font-size:.8125rem;font-weight:700;\
                 color:#007AFF;line-height:1.2\">●</span>"
                    .to_string()
            }
            (false, false) => {
                "<span aria-hidden=\"true\" style=\"font-size:.6875rem;line-height:1.2\">\
                 &nbsp;</span>"
                    .to_string()
            }
        };
        let aria_label = if has_events {
            let today_suffix = if is_today { "、今日" } else { "" };
            format!(
                "{year}年{month}月{day}日{today_suffix}、予定{count}{}",
                i18n::JA_HOME_CALENDAR_COUNT_SUFFIX
            )
        } else if is_today {
            format!("{year}年{month}月{day}日、今日")
        } else {
            format!("{year}年{month}月{day}日")
        };
        cells.push_str(&format!(
            "<div aria-label=\"{aria}\" style=\"min-height:60px;border:1px solid {border};\
             border-radius:10px;background:{bg};padding:.375rem .25rem;display:flex;\
             flex-direction:column;align-items:center;justify-content:space-between;\
             text-align:center;box-sizing:border-box\">\
             <span style=\"font-size:.9375rem;font-weight:700;color:{day_color};\
             line-height:1.1\">{day}</span>{marker_html}</div>",
            aria = render::escape_html(&aria_label),
            border = border,
            bg = bg,
            day_color = day_color,
            day = day,
            marker_html = marker_html
        ));
    }

    let empty = if counts.is_empty() {
        format!(
            "<p style=\"font-size:.875rem;color:#6e6e73;margin:.75rem 0 0;\
             text-align:center\">{}</p>",
            i18n::JA_HOME_CALENDAR_EMPTY
        )
    } else {
        String::new()
    };

    format!(
        "<section aria-label=\"{title}\" style=\"margin:0 auto 1.5rem;\
         max-width:42rem\">\
         <div style=\"display:flex;align-items:flex-end;justify-content:space-between;\
         gap:.75rem;margin-bottom:.75rem\">\
         <h2 style=\"font-size:1.25rem;font-weight:700;margin:0\">{title}</h2>\
         <p style=\"font-size:.9375rem;font-weight:700;color:#6e6e73;margin:0\">\
         {year}年{month}月</p>\
         </div>\
         <p style=\"font-size:.875rem;color:#6e6e73;line-height:1.5;margin:0 0 .75rem\">\
         {helper}</p>\
         <div style=\"background:#FFFFFF;border:1px solid #E5E5EA;border-radius:16px;\
         padding:.75rem;box-shadow:0 1px 2px rgba(0,0,0,.04)\">\
         <div style=\"display:grid;grid-template-columns:repeat(7,minmax(0,1fr));\
         gap:.25rem\">{cells}</div>{empty}</div>\
         </section>",
        title = i18n::JA_HOME_CALENDAR_TITLE,
        helper = i18n::JA_HOME_CALENDAR_HELPER,
        year = year,
        month = month,
        cells = cells,
        empty = empty
    )
}

fn parse_ymd(date: &str) -> Option<(i32, i32, i32)> {
    let mut parts = date.get(..10)?.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) {
        return None;
    }
    if !(1..=tz::days_in_month(year, month)).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

fn weekday_sunday_zero(year: i32, month: i32, day: i32) -> i32 {
    if !(1..=12).contains(&month) {
        return 0;
    }
    let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = year;
    if month < 3 {
        y -= 1;
    }
    (y + y / 4 - y / 100 + y / 400 + offsets[(month - 1) as usize] + day) % 7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_weekday_uses_sunday_zero() {
        assert_eq!(weekday_sunday_zero(2026, 7, 1), 3);
        assert_eq!(weekday_sunday_zero(2026, 7, 5), 0);
    }

    #[test]
    fn parse_ymd_rejects_invalid_dates() {
        assert_eq!(parse_ymd("2026-07-01"), Some((2026, 7, 1)));
        assert_eq!(parse_ymd("2026-02-29"), None);
        assert_eq!(parse_ymd("2024-02-29"), Some((2024, 2, 29)));
    }
}
