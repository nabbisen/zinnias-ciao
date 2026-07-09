use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;
use zinnias_ciao_domain::{
    DayInput, EventInput, RecurrenceFreq, expand_recurrence, validate_event,
};

use crate::audit;
use crate::authz::require_admin;
use crate::db::{self, event as event_db, event_write, membership as membership_db};
use crate::render;
use crate::session::require_auth;

use super::forms::render_event_create_fields;
use super::policy::{admin_events_new_next, event_can_seed_recreate, valid_prefill_day};
use super::support::{query_escape, redirect};

pub async fn get_create_event(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::CREATE_EVENT, None).await;

    let _community = db::community::find_active(&db, community_id).await?;
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await
        .unwrap_or_default();
    let _community_pairs: Vec<(String, String)> = _communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();
    let nav = render::bottom_nav(community_id, "home");

    // RFC-032: pre-fill from template if ?template=TID is present.
    let url = req.url()?;
    let template_id = url
        .query_pairs()
        .find(|(k, _)| k == "template")
        .map(|(_, v)| v.to_string());
    let err_msg: Option<String> = url
        .query_pairs()
        .find(|(k, _)| k == "err")
        .map(|(_, v)| v.to_string());
    let prefill_day = url
        .query_pairs()
        .find(|(k, _)| k == "day")
        .map(|(_, v)| v.to_string())
        .filter(|day| valid_prefill_day(day));
    let (prefill_title, prefill_location) = if let Some(ref tid) = template_id {
        let tmpl = db::event_template::find_active(&db, tid, community_id)
            .await
            .ok()
            .flatten();
        (
            tmpl.as_ref().map(|t| t.title.clone()),
            tmpl.as_ref().and_then(|t| t.location.clone()),
        )
    } else {
        (None, None)
    };

    let templates_link = format!(
        "<a href=\"/c/{cid}/admin/templates\" \
           style=\"display:block;text-align:center;color:#007AFF;\
           font-size:.875rem;margin-top:1rem;min-height:44px;line-height:44px\">\
           Use a template</a>",
        cid = render::escape_html(community_id),
    );

    let cet = i18n::JA_ADMIN_CREATE_EVENT_TITLE;
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">{cet}</h1>\
         <form method=\"post\" action=\"/c/{cid}/admin/events\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {fields}\
           <button type=\"submit\" style=\"width:100%;padding:.875rem;background:#007AFF;\
           color:#fff;border:none;border-radius:14px;font-size:1rem;font-weight:600;\
           min-height:44px;cursor:pointer;margin-top:1rem\">{submit}</button>\
         </form>\
         {tmpl_link}\
         </main>{nav}",
        header = render::header_with_switcher_next(
            i18n::JA_ADMIN_CREATE_EVENT_TITLE,
            community_id,
            &_community_pairs,
            &admin_events_new_next(prefill_day.as_deref())
        ),
        cid = render::escape_html(community_id),
        tok = render::escape_html(&token),
        fields = render_event_create_fields(
            prefill_title.as_deref(),
            prefill_location.as_deref(),
            None,
            err_msg.as_deref(),
            prefill_day.as_deref(),
            None,
            None,
        ),
        submit = i18n::JA_ADMIN_CREATE_EVENT_SUBMIT,
        tmpl_link = templates_link,
        nav = nav,
    );
    render::page(i18n::JA_ADMIN_CREATE_EVENT_TITLE, &body)
}

pub async fn post_create_event(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
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
        token_purpose::CREATE_EVENT,
        &raw_token,
        None,
    )
    .await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/home"));
    }

    let input = EventInput {
        title: body.get_field("title").unwrap_or_default(),
        location: Some(body.get_field("location").unwrap_or_default()),
        description: Some(body.get_field("description").unwrap_or_default()),
        days: vec![DayInput {
            day_date: body.get_field("day_date").unwrap_or_default(),
            starts_at: body.get_field("starts_at").unwrap_or_default(),
            ends_at: body.get_field("ends_at").unwrap_or_default(),
        }],
    };
    let copy_source_event_id = body
        .get_field("copy_source_event_id")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let copy_source_event_id = if let Some(source_id) = copy_source_event_id {
        let Some(source_event) =
            event_db::find_for_community(&db, &source_id, community_id).await?
        else {
            return render::not_found();
        };
        if !event_can_seed_recreate(&source_event) {
            return render::not_found();
        }
        Some(source_event.id)
    } else {
        None
    };

    // RFC-022: recurrence
    let freq_str = body.get_field("repeat_rule").unwrap_or_default();
    let freq = RecurrenceFreq::parse_form_value(&freq_str);
    let rep_count = body
        .get_field("repeat_count")
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);

    let validated = match validate_event(input) {
        Ok(v) => v,
        Err(e) => {
            let msg = query_escape(&e.to_string());
            return redirect(&format!("/c/{community_id}/admin/events/new?err={msg}"));
        }
    };

    // Expand recurrence from the single validated base day.
    let base_day = validated.days[0].clone();
    let expanded = match expand_recurrence(&base_day, freq, rep_count) {
        Ok(v) => v,
        Err(e) => {
            let msg = query_escape(&e.to_string());
            return redirect(&format!("/c/{community_id}/admin/events/new?err={msg}"));
        }
    };

    // Convert community-local "HH:MM" on day_date to true UTC (RFC-018).
    // The community timezone determines the offset for local->UTC conversion.
    // Unknown timezone names are a hard configuration error; we must not
    // silently store wrong UTC times.
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
    let days_utc: Vec<(String, String, String)> = expanded
        .iter()
        .map(|d| {
            let starts = zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.starts_at, off);
            let ends = zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.ends_at, off);
            (d.day_date.clone(), starts, ends)
        })
        .collect();

    let repeat_count_stored = if freq.is_recurring() {
        Some(expanded.len() as u32)
    } else {
        None
    };
    let event_id = event_write::create_event(
        &db,
        community_id,
        &membership.membership_id,
        &validated.title,
        validated.location.as_deref(),
        validated.description.as_deref(),
        &days_utc,
        freq.as_str(),
        repeat_count_stored,
    )
    .await?;

    let _ = audit::write(
        &db,
        rid,
        Some(community_id),
        Some(&membership.membership_id),
        "event",
        Some(&event_id),
        "created",
        Some(match copy_source_event_id {
            Some(source_id) => serde_json::json!({
                "created_from_cancelled_event_id": source_id
            }),
            None => serde_json::json!({ "title": validated.title }),
        }),
    )
    .await;

    redirect(&format!("/c/{community_id}/events/{event_id}"))
}
