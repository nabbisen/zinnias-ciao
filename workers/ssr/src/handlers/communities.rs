//! Calendar page for the active community (RFC-056).

use worker::{Env, Request, Response, Result};

mod calendar;
mod matrix;

use crate::db::{
    self, attendance as attendance_db, event as event_db, membership as membership_db,
};
use crate::render;
use crate::session::require_auth;
use zinnias_ciao_contracts::{i18n, tz};
use zinnias_ciao_domain::{
    month_intersects_materialization_window, recurrence_materialization_window,
};

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
    let Some(active_membership) =
        membership_db::find_active(&db, &auth.user_id, community_id).await?
    else {
        return render::not_found();
    };
    let can_create_event = active_membership.role == "admin";

    let community = db::community::find_active(&db, community_id).await?;
    let community_tz = community
        .as_ref()
        .map(|c| c.timezone.as_str())
        .unwrap_or("UTC");
    let now_prefix = db::now_utc();
    let tz_offset = tz::offset_minutes_or_utc(community_tz);
    let (today_date, _) = tz::to_local_parts(&now_prefix, tz_offset);
    let url = req.url()?;
    let requested_month = url
        .query_pairs()
        .find(|(k, _)| k == "month")
        .and_then(|(_, v)| calendar::parse_month(&v));
    let (today_year, today_month, today_day) =
        calendar::parse_ymd(&today_date).unwrap_or((1970, 1, 1));
    let (year, month) = requested_month.unwrap_or((today_year, today_month));
    let selected_day = url
        .query_pairs()
        .find(|(k, _)| k == "day")
        .map(|(_, v)| v.to_string())
        .filter(|day| {
            calendar::parse_ymd(day)
                .map(|(dy, dm, _)| dy == year && dm == month)
                .unwrap_or(false)
        });
    let view = matrix::CalendarView::from_query(
        url.query_pairs()
            .find(|(k, _)| k == "view")
            .map(|(_, v)| v.to_string())
            .as_deref(),
    );
    let (month_start, next_month_start) = calendar::month_bounds(year, month);
    let month_end = format!("{year:04}-{month:02}-{:02}", tz::days_in_month(year, month));
    let materialization_notice = match recurrence_materialization_window(&today_date) {
        Some(window)
            if month_intersects_materialization_window(
                &month_start,
                &next_month_start,
                &window,
            ) =>
        {
            let report =
                db::event_series::materialize_for_community_through(&db, community_id, &month_end)
                    .await?;
            if report.cap_reached {
                Some(i18n::JA_CALENDAR_MATERIALIZATION_LIMIT)
            } else {
                None
            }
        }
        Some(_) => Some(i18n::JA_CALENDAR_OUT_OF_RANGE),
        None => None,
    };
    let rows = match view {
        matrix::CalendarView::Month => {
            event_db::calendar_month_for_community(
                &db,
                community_id,
                &month_start,
                &next_month_start,
            )
            .await?
        }
        matrix::CalendarView::Matrix => {
            event_db::calendar_month_for_community_limited(
                &db,
                community_id,
                &month_start,
                &next_month_start,
                matrix::EVENT_DAY_ROW_CAP + 1,
            )
            .await?
        }
    };
    let today_day = if year == today_year && month == today_month {
        Some(today_day)
    } else {
        None
    };
    let mode_tabs =
        matrix::render_mode_tabs(community_id, year, month, selected_day.as_deref(), view);
    let content = match view {
        matrix::CalendarView::Month => {
            let calendar = calendar::render_calendar_month(
                community_id,
                year,
                month,
                today_day,
                selected_day.as_deref(),
                &rows,
            );
            let event_list = calendar::render_calendar_events(
                community_id,
                community_tz,
                &rows,
                selected_day.as_deref(),
                year,
                month,
                can_create_event,
            );
            format!("{calendar}{event_list}")
        }
        matrix::CalendarView::Matrix => {
            let members = membership_db::list_all_active(&db, community_id).await?;
            let attendances = if members.len() > matrix::MEMBER_ROW_CAP
                || rows.len() > matrix::EVENT_DAY_ROW_CAP
            {
                std::collections::HashMap::new()
            } else {
                let day_ids: Vec<&str> = rows.iter().map(|row| row.day_id.as_str()).collect();
                attendance_db::list_for_event_days(&db, &day_ids).await?
            };
            matrix::render_matrix(matrix::MatrixRenderInput {
                community_id,
                community_tz,
                year,
                month,
                selected_day: selected_day.as_deref(),
                rows: &rows,
                members: &members,
                attendances: &attendances,
            })
        }
    };

    // Header uses list_communities_for_user result as switcher pairs.
    let community_pairs: Vec<(String, String)> = summaries
        .iter()
        .map(|s| (s.community_id.clone(), s.community_name.clone()))
        .collect();

    let nav = render::bottom_nav(community_id, "communities");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           {notice}{mode_tabs}{content}\
         </main>{nav}",
        header = render::header_with_switcher_next(
            i18n::JA_NAV_COMMUNITIES,
            community_id,
            &community_pairs,
            &matrix::switcher_next(year, month, selected_day.as_deref(), view)
        ),
        mode_tabs = mode_tabs,
        notice = materialization_notice
            .map(|msg| format!(
                "<p role=\"status\" style=\"font-size:.875rem;color:#6e6e73;\
                 background:#F5F5F7;border-radius:12px;padding:.75rem;margin:0 auto 1rem;\
                 max-width:42rem;line-height:1.5\">{}</p>",
                render::escape_html(msg)
            ))
            .unwrap_or_default(),
        content = content,
        nav = nav,
    );
    render::page(i18n::JA_NAV_COMMUNITIES, &body)
}

#[cfg(test)]
mod tests;
