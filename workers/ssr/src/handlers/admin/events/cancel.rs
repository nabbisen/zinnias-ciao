use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz::require_admin;
use crate::db::{self, event as event_db, event_write, membership as membership_db};
use crate::render;

use super::policy::event_schedule_editable;
use super::support::redirect;

pub async fn get_cancel_event(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let _membership = require_admin(env, &auth, community_id, rid).await?;
    let db = env.d1("DB")?;
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::CANCEL_EVENT,
        Some(event_id),
    )
    .await?;

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None => return render::not_found(),
    };
    let days = event_db::days_for_event(&db, event_id).await?;
    let whole_event_scope = !event_schedule_editable(&event, &days);
    let community = db::community::find_active(&db, community_id).await?;
    let _community_name = community.map(|c| c.name).unwrap_or_default();
    let _communities_for_switcher = membership_db::list_communities_for_user(
        &db,
        &auth.user_id,
        auth.scope_community_id.as_deref(),
    )
    .await
    .unwrap_or_default();
    let _community_pairs: Vec<(String, String)> = _communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
         <h1 class=\"cz-admin-title cz-admin-title--snug\">{cat}</h1>\
         <p class=\"cz-admin-confirm-subtitle\"><strong>{title}</strong></p>\
         <p class=\"cz-admin-confirm-note\">{body_text}</p>\
         <div class=\"cz-admin-confirm-actions\">\
           <a href=\"/c/{cid}/events/{eid}\" \
              class=\"cz-admin-confirm-keep-link\">\
              {keep}</a>\
           <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/cancel\" \
             class=\"cz-admin-confirm-delete-form\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               class=\"cz-admin-confirm-delete-button\">\
               {confirm}</button>\
           </form>\
         </div></main>{nav}",
        header = render::header_with_switcher(
            i18n::JA_ADMIN_CANCEL_EVENT_TITLE,
            community_id,
            &_community_pairs
        ),
        title = render::escape_html(&event.title),
        cid = render::escape_html(community_id),
        eid = render::escape_html(event_id),
        tok = render::escape_html(&token),
        nav = nav,
        cat = i18n::JA_ADMIN_CANCEL_EVENT_TITLE,
        body_text = if whole_event_scope {
            i18n::JA_ADMIN_CANCEL_EVENT_BODY_ALL_DAYS
        } else {
            i18n::JA_ADMIN_CANCEL_EVENT_BODY
        },
        keep = i18n::JA_ADMIN_CANCEL_EVENT_KEEP,
        confirm = if whole_event_scope {
            i18n::JA_ADMIN_CANCEL_EVENT_CONFIRM_ALL_DAYS
        } else {
            i18n::JA_ADMIN_CANCEL_EVENT_CONFIRM
        },
    );
    render::page(i18n::JA_ADMIN_CANCEL_EVENT_TITLE, &body)
}

pub async fn post_cancel_event(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
    let db = env.d1("DB")?;

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::CANCEL_EVENT,
        &raw_token,
        Some(event_id),
    )
    .await?;
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    event_write::cancel_event(&db, rid, community_id, event_id, &membership.membership_id).await?;

    redirect(&format!("/c/{community_id}/events/{event_id}"))
}
