use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;
use zinnias_ciao_domain::status::DayTimeState;
use zinnias_ciao_domain::{EventInput, validate_event};

use crate::audit;
use crate::authz::require_admin;
use crate::db::{self, event as event_db, event_write, membership as membership_db};
use crate::handlers::event::classify_day;
use crate::render;
use crate::session::require_auth;

use super::forms::{render_details_only_event_edit_fields, render_single_day_edit_fields};
use super::policy::{
    EventDayUpdate, EventDetailsEdit, EventEditSubmission, edit_post_contains_schedule_fields,
    event_schedule_editable, validate_event_details,
};
use super::support::{query_escape, redirect};

pub async fn get_edit_event(
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

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None => return render::not_found(),
    };
    if event.status == "cancelled" {
        return render::page(
            i18n::JA_GENERAL_ERROR,
            &format!(
                "<main style=\"padding:2rem\"><p>{}</p><p><a href=\"javascript:history.back()\">{}</a></p></main>",
                i18n::JA_ADMIN_EDIT_CANCELLED,
                i18n::JA_GENERAL_BACK
            ),
        );
    }
    // RFC-018: editing is only allowed while the event is still upcoming.
    let days = event_db::days_for_event(&db, event_id).await?;
    let now_utc = db::now_utc();
    let already_started = days.iter().any(|d| {
        classify_day(&d.starts_at_utc, &d.ends_at_utc, &now_utc) != DayTimeState::Upcoming
    });
    if already_started {
        return render::page(
            i18n::JA_GENERAL_ERROR,
            &format!(
                "<main style=\"padding:2rem\"><p>{}</p><p><a href=\"javascript:history.back()\">{}</a></p></main>",
                i18n::JA_ADMIN_EDIT_STARTED,
                i18n::JA_GENERAL_BACK
            ),
        );
    }
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::EDIT_EVENT,
        Some(event_id),
    )
    .await;

    let community_tz = db::community::find_active(&db, community_id)
        .await?
        .map(|c| c.timezone)
        .unwrap_or_else(|| "UTC".to_string());
    let schedule_editable = event_schedule_editable(&event, &days);
    let (prefill_date, prefill_start, prefill_end) = if schedule_editable {
        // Display path: fall back to UTC for unknown zones. Correct config is
        // enforced at write time.
        let off = zinnias_ciao_contracts::tz::offset_minutes_or_utc(&community_tz);
        let d = &days[0];
        let (date, start) = zinnias_ciao_contracts::tz::to_local_parts(&d.starts_at_utc, off);
        let (_, end) = zinnias_ciao_contracts::tz::to_local_parts(&d.ends_at_utc, off);
        (Some(date), Some(start), Some(end))
    } else {
        (None, None, None)
    };

    let communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await
        .unwrap_or_default();
    let community_pairs: Vec<(String, String)> = communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();
    let nav = render::bottom_nav(community_id, "home");

    let url = req.url()?;
    let err: Option<String> = url
        .query_pairs()
        .find(|(k, _)| k == "err")
        .map(|(_, v)| v.to_string());

    let fields = if schedule_editable {
        render_single_day_edit_fields(
            Some(&event.title),
            event.location.as_deref(),
            event.description.as_deref(),
            err.as_deref(),
            prefill_date.as_deref(),
            prefill_start.as_deref(),
            prefill_end.as_deref(),
        )
    } else {
        render_details_only_event_edit_fields(&event, &days, &community_tz, err.as_deref())
    };

    let eet = i18n::JA_ADMIN_EDIT_EVENT_TITLE;
    let ees = i18n::JA_ADMIN_EDIT_EVENT_SUBMIT;
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">{eet}</h1>\
         <p style=\"font-size:.8125rem;color:{muted};margin-bottom:1rem\">\
           Members will see the updated event details.</p>\
         <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/edit\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {fields}\
           <button type=\"submit\" style=\"width:100%;padding:.875rem;background:{going};\
           color:#fff;border:none;border-radius:14px;font-size:1rem;font-weight:600;\
           min-height:44px;cursor:pointer;margin-top:1rem\">{ees}</button>\
         </form>\
         <div style=\"margin-top:1.5rem\">\
           <a href=\"/c/{cid}/events/{eid}\" \
              style=\"color:{muted};font-size:.875rem\">{back}</a>\
         </div>\
         </main>{nav}",
        header = render::header_with_switcher(
            i18n::JA_ADMIN_EDIT_EVENT_TITLE,
            community_id,
            &community_pairs
        ),
        cid = render::escape_html(community_id),
        eid = render::escape_html(event_id),
        tok = render::escape_html(&token),
        muted = "#6E6E73",
        back = i18n::JA_NAV_BACK,
        going = "#007AFF",
        fields = fields,
        nav = nav,
    );
    render::page(i18n::JA_ADMIN_EDIT_EVENT_TITLE, &body)
}

pub async fn post_edit_event(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::EDIT_EVENT,
        &raw_token,
        Some(event_id),
    )
    .await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None => return render::not_found(),
    };
    if event.status == "cancelled" {
        return render::not_found();
    }
    // RFC-018: reject POST edits if the event has already started.
    let days_check = event_db::days_for_event(&db, event_id).await?;
    let now_check = db::now_utc();
    if days_check.iter().any(|d| {
        classify_day(&d.starts_at_utc, &d.ends_at_utc, &now_check) != DayTimeState::Upcoming
    }) {
        return render::not_found();
    }

    let schedule_editable = event_schedule_editable(&event, &days_check);
    let submission = if schedule_editable {
        let input = EventInput {
            title: body.get_field("title").unwrap_or_default(),
            location: Some(body.get_field("location").unwrap_or_default()),
            description: Some(body.get_field("description").unwrap_or_default()),
            days: vec![zinnias_ciao_domain::DayInput {
                day_date: body.get_field("day_date").unwrap_or_default(),
                starts_at: body.get_field("starts_at").unwrap_or_default(),
                ends_at: body.get_field("ends_at").unwrap_or_default(),
            }],
        };

        let validated = match validate_event(input) {
            Ok(v) => v,
            Err(e) => {
                let msg = query_escape(&e.to_string());
                return redirect(&format!(
                    "/c/{community_id}/admin/events/{event_id}/edit?err={msg}"
                ));
            }
        };
        let community_tz = db::community::find_active(&db, community_id)
            .await?
            .map(|c| c.timezone)
            .unwrap_or_else(|| "UTC".to_string());
        let off = match zinnias_ciao_contracts::tz::offset_minutes(&community_tz) {
            Some(o) => o,
            None => {
                return render::page(
                    i18n::JA_GENERAL_ERROR,
                    &format!("<p style=\"color:#FF3B30\">{}</p>", i18n::JA_TZ_ERROR),
                );
            }
        };
        let d = &validated.days[0];
        EventEditSubmission {
            details: EventDetailsEdit {
                title: validated.title,
                location: validated.location,
                description: validated.description,
            },
            day_update: Some(EventDayUpdate {
                day_date: d.day_date.clone(),
                starts_at_utc: zinnias_ciao_contracts::tz::local_to_utc(
                    &d.day_date,
                    &d.starts_at,
                    off,
                ),
                ends_at_utc: zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.ends_at, off),
            }),
        }
    } else {
        if edit_post_contains_schedule_fields(&body) {
            return redirect(&format!(
                "/c/{community_id}/admin/events/{event_id}/edit?err={}",
                query_escape(i18n::JA_ADMIN_EDIT_SCHEDULE_NOT_EDITABLE)
            ));
        }
        let details = match validate_event_details(
            body.get_field("title").unwrap_or_default(),
            body.get_field("location").unwrap_or_default(),
            body.get_field("description").unwrap_or_default(),
        ) {
            Ok(v) => v,
            Err(e) => {
                let msg = query_escape(&e.to_string());
                return redirect(&format!(
                    "/c/{community_id}/admin/events/{event_id}/edit?err={msg}"
                ));
            }
        };
        EventEditSubmission {
            details,
            day_update: None,
        }
    };

    event_write::edit_event(
        &db,
        event_id,
        &submission.details.title,
        submission.details.location.as_deref(),
        submission.details.description.as_deref(),
        submission.day_update.as_ref().map(|day| {
            (
                day.day_date.as_str(),
                day.starts_at_utc.as_str(),
                day.ends_at_utc.as_str(),
            )
        }),
    )
    .await?;

    let _ = audit::write(
        &db,
        rid,
        Some(community_id),
        Some(&membership.membership_id),
        "event",
        Some(event_id),
        "edited",
        Some(serde_json::json!({
            "edit_scope": if schedule_editable { "single_day_schedule" } else { "details_only" }
        })),
    )
    .await;

    redirect(&format!("/c/{community_id}/events/{event_id}"))
}
