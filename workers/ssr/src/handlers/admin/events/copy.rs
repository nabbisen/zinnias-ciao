use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::{i18n, tz};
use zinnias_ciao_domain::recurrence_materialization_window;

use crate::authz::require_admin;
use crate::db::{self, event as event_db, event_series as series_db, membership as membership_db};
use crate::render;
use crate::session::require_auth;

use super::forms::{RepeatFieldPrefill, render_event_create_fields_with_repeat};
use super::policy::event_is_recurring;

pub(super) struct EventCopyPrefill {
    pub(super) title: String,
    pub(super) location: Option<String>,
    pub(super) description: Option<String>,
    pub(super) helpers: Vec<&'static str>,
    pub(super) day_date: Option<String>,
    pub(super) starts_at: Option<String>,
    pub(super) ends_at: Option<String>,
    pub(super) repeat: RepeatFieldPrefill,
}

pub async fn get_copy_event(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let source_event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(event) => event,
        None => return render::not_found(),
    };
    let source_days = event_db::days_for_event(&db, event_id).await?;
    let source_series = if event_is_recurring(&source_event) {
        series_db::find_for_event(&db, event_id, community_id).await?
    } else {
        None
    };
    let community = db::community::find_active(&db, community_id).await?;
    let community_tz = community
        .as_ref()
        .map(|c| c.timezone.as_str())
        .unwrap_or("UTC");
    let offset = match tz::offset_minutes(community_tz) {
        Some(o) => o,
        None => {
            return render::page(
                i18n::JA_GENERAL_ERROR,
                &format!("<p style=\"color:#FF3B30\">{}</p>", i18n::JA_TZ_ERROR),
            );
        }
    };
    let (today_local, _) = tz::to_local_parts(&db::now_utc(), offset);
    let window = match recurrence_materialization_window(&today_local) {
        Some(w) => w,
        None => return render::internal_error(),
    };
    let prefill = build_event_copy_prefill(
        &source_event,
        &source_days,
        source_series.as_ref(),
        community_tz,
        &today_local,
        &window.through_day_date,
    );

    let token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::CREATE_EVENT, None).await;
    let communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await
        .unwrap_or_default();
    let community_pairs: Vec<(String, String)> = communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">{title}</h1>\
         <form method=\"post\" action=\"/c/{cid}/admin/events\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {fields}\
           <button type=\"submit\" style=\"width:100%;padding:.875rem;background:#007AFF;\
           color:#fff;border:none;border-radius:14px;font-size:1rem;font-weight:600;\
           min-height:44px;cursor:pointer;margin-top:1rem\">{submit}</button>\
         </form>\
         <div style=\"margin-top:1.5rem\">\
           <a href=\"/c/{cid}/events/{eid}\" style=\"color:#6E6E73;font-size:.875rem\">{back}</a>\
         </div>\
         </main>{nav}",
        header = render::header_with_switcher_next(
            i18n::JA_ADMIN_COPY_EVENT_TITLE,
            community_id,
            &community_pairs,
            "admin_events_new",
        ),
        title = i18n::JA_ADMIN_COPY_EVENT_TITLE,
        cid = render::escape_html(community_id),
        eid = render::escape_html(event_id),
        tok = render::escape_html(&token),
        fields = render_copy_event_create_fields(event_id, &prefill, None),
        submit = i18n::JA_ADMIN_CREATE_EVENT_SUBMIT,
        back = i18n::JA_NAV_BACK,
        nav = nav,
    );
    render::page(i18n::JA_ADMIN_COPY_EVENT_TITLE, &body)
}

pub(super) fn render_copy_event_create_fields(
    source_event_id: &str,
    prefill: &EventCopyPrefill,
    error: Option<&str>,
) -> String {
    let helpers = prefill
        .helpers
        .iter()
        .map(|helper| {
            format!(
                "<p role=\"note\" style=\"font-size:.875rem;color:#6E6E73;line-height:1.5;\
                 margin:0 0 .75rem\">{}</p>",
                render::escape_html(helper)
            )
        })
        .collect::<String>();
    format!(
        "<input type=\"hidden\" name=\"copy_source_event_id\" value=\"{eid}\">\
         <input type=\"hidden\" name=\"copy_mode\" value=\"event_copy\">\
         {helpers}\
         {fields}",
        eid = render::escape_html(source_event_id),
        helpers = helpers,
        fields = render_event_create_fields_with_repeat(
            Some(&prefill.title),
            prefill.location.as_deref(),
            prefill.description.as_deref(),
            error,
            prefill.day_date.as_deref(),
            prefill.starts_at.as_deref(),
            prefill.ends_at.as_deref(),
            &prefill.repeat,
        ),
    )
}

pub(super) fn build_event_copy_prefill(
    event: &event_db::EventRow,
    days: &[event_db::EventDayRow],
    series: Option<&series_db::EventSeriesRow>,
    community_tz: &str,
    today_local: &str,
    window_through: &str,
) -> EventCopyPrefill {
    let mut prefill = EventCopyPrefill {
        title: event.title.clone(),
        location: event.location.clone(),
        description: event.description.clone(),
        helpers: vec![i18n::JA_ADMIN_COPY_EVENT_HELPER],
        day_date: None,
        starts_at: None,
        ends_at: None,
        repeat: RepeatFieldPrefill::normal_create_default(),
    };

    if event_is_recurring(event) {
        apply_recurring_prefill(&mut prefill, series, today_local, window_through);
    } else if days.len() == 1 {
        apply_single_day_prefill(&mut prefill, &days[0], community_tz);
    } else if days.len() > 1 {
        prefill
            .helpers
            .push(i18n::JA_ADMIN_COPY_EVENT_MULTI_DAY_HELPER);
    } else {
        prefill
            .helpers
            .push(i18n::JA_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE);
    }

    prefill
}

fn apply_single_day_prefill(
    prefill: &mut EventCopyPrefill,
    day: &event_db::EventDayRow,
    community_tz: &str,
) {
    let offset = tz::offset_minutes_or_utc(community_tz);
    let (_, starts_at) = tz::to_local_parts(&day.starts_at_utc, offset);
    let (_, ends_at) = tz::to_local_parts(&day.ends_at_utc, offset);
    prefill.day_date = Some(day.day_date.clone());
    prefill.starts_at = Some(starts_at);
    prefill.ends_at = Some(ends_at);
    prefill.helpers.push(i18n::JA_ADMIN_COPY_EVENT_DATE_WARNING);
}

fn apply_recurring_prefill(
    prefill: &mut EventCopyPrefill,
    series: Option<&series_db::EventSeriesRow>,
    today_local: &str,
    window_through: &str,
) {
    let Some(series) = series else {
        prefill
            .helpers
            .push(i18n::JA_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE);
        return;
    };
    let (Some(starts_at), Some(ends_at)) = (
        series.starts_at_local.as_deref(),
        series.ends_at_local.as_deref(),
    ) else {
        prefill
            .helpers
            .push(i18n::JA_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE);
        return;
    };

    prefill.repeat.repeat_rule = series.frequency.clone();
    prefill.starts_at = Some(starts_at.to_string());
    prefill.ends_at = Some(ends_at.to_string());

    if series.start_day_date.as_str() < today_local {
        prefill
            .helpers
            .push(i18n::JA_ADMIN_COPY_EVENT_RECURRING_PAST);
        return;
    }
    if series.start_day_date.as_str() > window_through {
        prefill
            .helpers
            .push(i18n::JA_ADMIN_COPY_EVENT_RECURRING_WINDOW);
        return;
    }

    prefill.day_date = Some(series.start_day_date.clone());
    prefill.helpers.push(i18n::JA_ADMIN_COPY_EVENT_DATE_WARNING);
    match series.end_mode.as_str() {
        "open_ended" => prefill.repeat.repeat_end_mode = "open_ended".to_string(),
        "after_count" => match series.occurrence_count {
            Some(count) => {
                prefill.repeat.repeat_end_mode = "after_count".to_string();
                prefill.repeat.repeat_count = Some(count);
            }
            None => prefill
                .helpers
                .push(i18n::JA_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE),
        },
        "until_date" => match series.until_day_date.as_deref() {
            Some(until) if until >= series.start_day_date.as_str() => {
                prefill.repeat.repeat_end_mode = "until_date".to_string();
                prefill.repeat.repeat_until = Some(until.to_string());
            }
            _ => prefill
                .helpers
                .push(i18n::JA_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE),
        },
        _ => prefill
            .helpers
            .push(i18n::JA_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE),
    }
}
