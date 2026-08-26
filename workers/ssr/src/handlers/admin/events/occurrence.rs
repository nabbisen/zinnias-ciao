use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::{auth::token_purpose, i18n};

use crate::authz::require_admin;
use crate::db::{event as event_db, event_write};
use crate::render;

use super::support::redirect;

pub async fn get_cancel_occurrence(
    _req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
    day_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&_req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
    let locale = membership.locale;
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
    .await?;
    let body = format!(
        "<main class=\"cz-page-main cz-page-main--narrow\">\
         <h1 class=\"cz-admin-occurrence-title\">{title}</h1>\
         <p class=\"cz-admin-occurrence-helper\">{helper}</p>\
         <p class=\"cz-admin-occurrence-subtitle\">{event_title} · {date}</p>\
         <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/days/{did}/cancel\">\
           <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
           <button type=\"submit\" class=\"cz-admin-occurrence-cancel-button\">{submit}</button>\
         </form>\
         <a href=\"/c/{cid}/events/{eid}\" class=\"cz-admin-occurrence-back-link\">{keep}</a>\
         </main>",
        title = i18n::t(locale, i18n::OCCURRENCE_CANCEL_TITLE),
        helper = i18n::t(locale, i18n::OCCURRENCE_CANCEL_HELPER),
        event_title = render::escape_html(&event.title),
        date = render::escape_html(&day.day_date),
        cid = render::escape_html(community_id),
        eid = render::escape_html(event_id),
        did = render::escape_html(day_id),
        token = render::escape_html(&token),
        submit = i18n::t(locale, i18n::OCCURRENCE_CANCEL_SUBMIT),
        keep = i18n::t(locale, i18n::OCCURRENCE_CANCEL_KEEP),
    );
    render::page_localized(
        locale,
        i18n::t(locale, i18n::OCCURRENCE_CANCEL_TITLE),
        &body,
    )
}

pub async fn post_cancel_occurrence(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
    day_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
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
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }
    let _event = match event_db::find_for_community(&db, event_id, community_id).await? {
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
        rid,
        event_id,
        day_id,
        &membership.membership_id,
        series_id,
        community_id,
        exception_day_date,
    )
    .await?;
    redirect(&format!("/c/{community_id}/events/{event_id}"))
}
