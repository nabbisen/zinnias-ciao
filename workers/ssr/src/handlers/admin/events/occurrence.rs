use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::{auth::token_purpose, i18n};

use crate::audit;
use crate::authz::require_admin;
use crate::db::{event as event_db, event_write};
use crate::render;
use crate::session::require_auth;

use super::support::redirect;

pub async fn get_cancel_occurrence(
    _req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
    day_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&_req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) if e.status != "cancelled" => e,
        _ => return render::not_found(),
    };
    let days = event_db::days_for_event(&db, event_id).await?;
    let day = match days.iter().find(|day| day.id == day_id) {
        Some(day) if day.series_id.is_some() && day.occurrence_status != "cancelled" => day,
        _ => return render::not_found(),
    };
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::CANCEL_OCCURRENCE,
        Some(day_id),
    )
    .await;
    let body = format!(
        "<main style=\"padding:1rem 1rem 5rem;max-width:42rem;margin:0 auto\">\
         <h1 style=\"font-size:1.25rem;font-weight:700;margin:0 0 .75rem\">{title}</h1>\
         <p style=\"font-size:.9375rem;line-height:1.5;color:#3A3A3C;margin:0 0 1rem\">{helper}</p>\
         <p style=\"font-size:.875rem;color:#6e6e73;margin:0 0 1rem\">{event_title} · {date}</p>\
         <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/days/{did}/cancel\">\
           <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
           <button type=\"submit\" style=\"width:100%;padding:.875rem;background:#B42318;\
           color:#fff;border:none;border-radius:14px;font-size:1rem;font-weight:600;\
           min-height:44px;cursor:pointer\">{submit}</button>\
         </form>\
         <a href=\"/c/{cid}/events/{eid}\" style=\"display:inline-flex;align-items:center;\
         min-height:44px;margin-top:.75rem;color:#007AFF;text-decoration:none\">{back}</a>\
         </main>",
        title = i18n::JA_OCCURRENCE_CANCEL_TITLE,
        helper = i18n::JA_OCCURRENCE_CANCEL_HELPER,
        event_title = render::escape_html(&event.title),
        date = render::escape_html(&day.day_date),
        cid = render::escape_html(community_id),
        eid = render::escape_html(event_id),
        did = render::escape_html(day_id),
        token = render::escape_html(&token),
        submit = i18n::JA_OCCURRENCE_CANCEL_SUBMIT,
        back = i18n::JA_EVENT_TITLE_HEADER,
    );
    render::page(i18n::JA_OCCURRENCE_CANCEL_TITLE, &body)
}

pub async fn post_cancel_occurrence(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
    day_id: &str,
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
        token_purpose::CANCEL_OCCURRENCE,
        &raw_token,
        Some(day_id),
    )
    .await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }
    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) if e.status != "cancelled" => e,
        _ => return render::not_found(),
    };
    let days = event_db::days_for_event(&db, event_id).await?;
    let day = match days.iter().find(|day| day.id == day_id) {
        Some(day) if day.series_id.is_some() && day.occurrence_status != "cancelled" => day,
        _ => return render::not_found(),
    };
    let series_id = day.series_id.as_deref().unwrap_or_default();
    let exception_day_date = day
        .series_occurrence_date
        .as_deref()
        .unwrap_or(day.day_date.as_str());
    event_write::cancel_occurrence(
        &db,
        day_id,
        &membership.membership_id,
        series_id,
        community_id,
        exception_day_date,
    )
    .await?;
    let _ = audit::write(
        &db,
        rid,
        Some(community_id),
        Some(&membership.membership_id),
        "event_day",
        Some(day_id),
        "occurrence_cancelled",
        Some(serde_json::json!({
            "event_id": event.id,
            "series_id": series_id,
            "exception_day_date": exception_day_date
        })),
    )
    .await;
    redirect(&format!("/c/{community_id}/events/{event_id}"))
}
