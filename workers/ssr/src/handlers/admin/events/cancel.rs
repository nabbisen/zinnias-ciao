use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::audit;
use crate::authz::require_admin;
use crate::db::{self, event as event_db, event_write, membership as membership_db};
use crate::render;
use crate::session::require_auth;

use super::policy::event_schedule_editable;
use super::support::redirect;

pub async fn get_cancel_event(
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
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::CANCEL_EVENT,
        Some(event_id),
    )
    .await;

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None => return render::not_found(),
    };
    let days = event_db::days_for_event(&db, event_id).await?;
    let whole_event_scope = !event_schedule_editable(&event, &days);
    let community = db::community::find_active(&db, community_id).await?;
    let _community_name = community.map(|c| c.name).unwrap_or_default();
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await
        .unwrap_or_default();
    let _community_pairs: Vec<(String, String)> = _communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">{cat}</h1>\
         <p style=\"font-size:.9375rem;color:#6e6e73\"><strong>{title}</strong></p>\
         <p style=\"font-size:.875rem;color:#6e6e73\">{body_text}</p>\
         <div style=\"display:flex;flex-wrap:wrap;gap:.75rem;margin-top:1.5rem\">\
           <a href=\"/c/{cid}/events/{eid}\" \
              style=\"flex:1 1 9rem;min-width:0;box-sizing:border-box;padding:.875rem;\
              border:2px solid #e5e5ea;border-radius:14px;text-align:center;\
              text-decoration:none;color:#1D1D1F;font-weight:600;overflow-wrap:anywhere\">\
              {keep}</a>\
           <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/cancel\" \
             style=\"flex:1 1 9rem;min-width:0;margin:0\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               style=\"width:100%;box-sizing:border-box;padding:.875rem;background:#FF3B30;\
               color:#fff;border:none;border-radius:14px;font-weight:600;min-height:44px;\
               cursor:pointer;white-space:normal;overflow-wrap:anywhere\">\
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
        token_purpose::CANCEL_EVENT,
        &raw_token,
        Some(event_id),
    )
    .await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    event_write::cancel_event(&db, event_id, &membership.membership_id).await?;
    let _ = audit::write(
        &db,
        rid,
        Some(community_id),
        Some(&membership.membership_id),
        "event",
        Some(event_id),
        "cancelled",
        None,
    )
    .await;

    redirect(&format!("/c/{community_id}/events/{event_id}"))
}
