use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz::require_admin;
use crate::db::{event as event_db, membership as membership_db};
use crate::render;

use super::support::redirect;

pub async fn get_admin_hide_note_confirm(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
    let locale = membership.locale;
    let db = env.d1("DB")?;

    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::ADMIN_HIDE_NOTE,
        Some(event_id),
    )
    .await?;

    let all = membership_db::list_all_active(&db, community_id).await?;
    let target_name = all
        .iter()
        .find(|m| m.id == target_membership_id)
        .map(|m| m.display_name.as_str())
        .unwrap_or("this member");

    let communities = membership_db::list_communities_for_user(
        &db,
        &auth.user_id,
        auth.scope_community_id.as_deref(),
    )
    .await
    .unwrap_or_default();
    let pairs: Vec<(String, String)> = communities
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();
    let nav = render::bottom_nav_localized(community_id, "home", locale);

    let nd = i18n::t(locale, i18n::NOTE_DELETE);
    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
           <h1 class=\"cz-confirm-title\">{nd}</h1>\
           <p class=\"cz-confirm-body\">\
             {consequence} {name}</p>\
           <div class=\"cz-confirm-actions\">\
             <a href=\"/c/{cid}/events/{eid}\" \
                class=\"cz-confirm-keep-link\">{keep}</a>\
             <form method=\"post\" \
                   action=\"/c/{cid}/admin/events/{eid}/notes/{mid}/hide\" class=\"cz-confirm-delete-form\">\
               <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
               <button type=\"submit\" \
                 class=\"cz-confirm-delete-button\">\
                 {nd}</button>\
             </form>\
           </div>\
         </main>{nav}",
        header = render::header_with_switcher_localized(nd, community_id, &pairs, locale),
        name = render::escape_html(target_name),
        cid = render::escape_html(community_id),
        eid = render::escape_html(event_id),
        mid = render::escape_html(target_membership_id),
        tok = render::escape_html(&token),
        nav = nav,
        nd = nd,
        keep = i18n::t(locale, i18n::NOTE_KEEP_ACTION),
        consequence = i18n::t(locale, i18n::ADMIN_REMOVE_CONSEQUENCE),
    );
    render::page_localized(locale, nd, &body)
}

pub async fn post_admin_hide_note(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
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
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    if event_db::find_for_community(&db, event_id, community_id)
        .await?
        .is_none()
    {
        return render::not_found();
    }

    crate::db::event_note::admin_hide_required(
        &db,
        rid,
        community_id,
        &membership.membership_id,
        event_id,
        target_membership_id,
    )
    .await?;

    redirect(&format!(
        "/c/{community_id}/events/{event_id}?flash=note_hidden"
    ))
}
