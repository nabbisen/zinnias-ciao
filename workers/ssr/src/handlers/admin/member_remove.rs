//! Admin member-removal handlers — RFC-010 / RFC-062 guarded writes.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz::require_admin;
use crate::db::{self, membership as membership_db};
use crate::render;

fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

// ── GET /c/:cid/admin/members/:mid/remove ────────────────────────────────

pub async fn get_remove_member(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
    let locale = membership.locale;

    // Cannot remove yourself.
    if target_membership_id == membership.membership_id {
        return render::not_found();
    }

    let db = env.d1("DB")?;
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::REMOVE_MEMBER,
        Some(target_membership_id),
    )
    .await?;

    // RFC-082 §1: suspended → removed is a valid transition, so the target
    // lookup here is present-based, not active-based — an already-suspended
    // member must still be reachable for removal.
    let target =
        match membership_db::find_present_summary(&db, target_membership_id, community_id).await? {
            Some(target) => target,
            None => return render::not_found(),
        };

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
    let nav = render::bottom_nav_localized(community_id, "home", locale);

    let rmt = i18n::t(locale, i18n::ADMIN_REMOVE_TITLE);
    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
         <h1 class=\"cz-admin-title cz-admin-title--snug\">{rmt}</h1>\
         <p class=\"cz-admin-confirm-subtitle\">\
           <strong>{name}</strong><br>{consequence}\
         </p>\
         <div class=\"cz-admin-role-actions\">\
           <a href=\"/c/{cid}/admin/members\" \
              class=\"cz-admin-role-keep-link\">\
              {keep}</a>\
           <form method=\"post\" \
             action=\"/c/{cid}/admin/members/{mid}/remove\" class=\"cz-confirm-delete-form\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               class=\"cz-confirm-delete-button\">\
               {confirm}</button>\
           </form>\
         </div></main>{nav}",
        header = render::header_with_switcher_next_localized(
            rmt,
            community_id,
            &_community_pairs,
            "admin_members",
            locale,
        ),
        name = render::escape_html(&target.display_name),
        cid = render::escape_html(community_id),
        mid = render::escape_html(target_membership_id),
        tok = render::escape_html(&token),
        nav = nav,
        rmt = rmt,
        consequence = i18n::t(locale, i18n::ADMIN_REMOVE_CONSEQUENCE),
        keep = i18n::t(locale, i18n::ADMIN_REMOVE_KEEP),
        confirm = i18n::t(locale, i18n::ADMIN_REMOVE_CONFIRM),
    );
    render::page_localized(locale, rmt, &body)
}

// ── POST /c/:cid/admin/members/:mid/remove ───────────────────────────────

pub async fn post_remove_member(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;

    if target_membership_id == membership.membership_id {
        return render::not_found();
    }

    let db = env.d1("DB")?;

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::REMOVE_MEMBER,
        &raw_token,
        Some(target_membership_id),
    )
    .await?;
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/admin/members"));
    }

    match membership_db::soft_remove_guarded_required(
        &db,
        rid,
        target_membership_id,
        community_id,
        &membership.membership_id,
    )
    .await?
    {
        membership_db::RemoveMemberResult::Removed => {
            redirect(&format!("/c/{community_id}/admin/members"))
        }
        membership_db::RemoveMemberResult::LastAdminBlocked => render::page_localized(
            membership.locale,
            i18n::t(membership.locale, i18n::GENERAL_ERROR),
            &format!(
                "<main class=\"cz-admin-error-main\"><p>{}</p></main>",
                i18n::t(membership.locale, i18n::ADMIN_LAST_ADMIN)
            ),
        ),
        membership_db::RemoveMemberResult::InvalidTarget => render::not_found(),
    }
}
