use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz::require_admin;
use crate::db::{event as event_db, membership as membership_db};
use crate::render;

use super::forms::render_recreate_event_create_fields;
use super::policy::event_can_seed_recreate;

pub async fn get_recreate_event(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let _membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let source_event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(event) if event_can_seed_recreate(&event) => event,
        _ => return render::not_found(),
    };
    let token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::CREATE_EVENT, None).await?;

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
         <main class=\"cz-page-main\">\
         <h1 class=\"cz-admin-title cz-admin-title--snug\">{title}</h1>\
         <form method=\"post\" action=\"/c/{cid}/admin/events\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {fields}\
           <button type=\"submit\" class=\"cz-admin-submit-button\">{submit}</button>\
         </form>\
         <div class=\"cz-admin-back-row\">\
           <a href=\"/c/{cid}/events/{eid}\" class=\"cz-admin-back-link\">{back}</a>\
         </div>\
         </main>{nav}",
        header = render::header_with_switcher_next(
            i18n::JA_ADMIN_CREATE_EVENT_TITLE,
            community_id,
            &community_pairs,
            "admin_events_new",
        ),
        title = i18n::JA_ADMIN_CREATE_EVENT_TITLE,
        cid = render::escape_html(community_id),
        eid = render::escape_html(event_id),
        tok = render::escape_html(&token),
        fields = render_recreate_event_create_fields(&source_event, None),
        submit = i18n::JA_ADMIN_CREATE_EVENT_SUBMIT,
        back = i18n::JA_NAV_BACK,
        nav = nav,
    );
    render::page(i18n::JA_ADMIN_CREATE_EVENT_TITLE, &body)
}
