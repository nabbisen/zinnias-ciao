use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::audit;
use crate::authz::require_admin;
use crate::db::{event as event_db, membership as membership_db};
use crate::render;
use crate::session::require_auth;

use super::support::redirect;

pub async fn get_admin_hide_note_confirm(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
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
        token_purpose::ADMIN_HIDE_NOTE,
        Some(event_id),
    )
    .await;

    let all = membership_db::list_all_active(&db, community_id).await?;
    let target_name = all
        .iter()
        .find(|m| m.id == target_membership_id)
        .map(|m| m.display_name.as_str())
        .unwrap_or("this member");

    let communities = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await
        .unwrap_or_default();
    let pairs: Vec<(String, String)> = communities
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">{nd}</h1>\
           <p style=\"font-size:.9375rem;color:#6E6E73;margin-bottom:1.5rem\">\
             {consequence} {name}</p>\
           <div style=\"display:flex;gap:.75rem\">\
             <a href=\"/c/{cid}/events/{eid}\" \
                style=\"flex:1;padding:.875rem;border:2px solid #e5e5ea;border-radius:14px;\
                text-align:center;text-decoration:none;color:#1D1D1F;font-weight:600;min-height:44px;\
                display:flex;align-items:center;justify-content:center\">{keep}</a>\
             <form method=\"post\" \
                   action=\"/c/{cid}/admin/events/{eid}/notes/{mid}/hide\" style=\"flex:1\">\
               <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
               <button type=\"submit\" \
                 style=\"width:100%;padding:.875rem;background:#FF3B30;color:#fff;\
                 border:none;border-radius:14px;font-weight:600;min-height:44px;cursor:pointer\">\
                 {nd}</button>\
             </form>\
           </div>\
         </main>{nav}",
        header = render::header_with_switcher(i18n::JA_NOTE_DELETE, community_id, &pairs),
        name = render::escape_html(target_name),
        cid = render::escape_html(community_id),
        eid = render::escape_html(event_id),
        mid = render::escape_html(target_membership_id),
        tok = render::escape_html(&token),
        nav = nav,
        nd = i18n::JA_NOTE_DELETE,
        keep = i18n::JA_NOTE_KEEP_ACTION,
        consequence = i18n::JA_ADMIN_REMOVE_CONSEQUENCE,
    );
    render::page(i18n::JA_NOTE_DELETE, &body)
}

pub async fn post_admin_hide_note(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
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
        token_purpose::ADMIN_HIDE_NOTE,
        &raw_token,
        Some(event_id),
    )
    .await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    if event_db::find_for_community(&db, event_id, community_id)
        .await?
        .is_none()
    {
        return render::not_found();
    }

    crate::db::event_note::admin_hide(&db, event_id, target_membership_id).await?;

    let _ = audit::write(
        &db,
        rid,
        Some(community_id),
        Some(&membership.membership_id),
        "event_note",
        Some(event_id),
        "admin_hidden",
        Some(serde_json::json!({ "target_membership_id": target_membership_id })),
    )
    .await;

    redirect(&format!(
        "/c/{community_id}/events/{event_id}?flash=Note+removed"
    ))
}
