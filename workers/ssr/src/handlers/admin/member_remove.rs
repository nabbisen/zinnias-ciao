//! Admin member-removal handlers — RFC-010 / RFC-062 guarded writes.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::audit;
use crate::authz::require_admin;
use crate::db::{self, membership as membership_db};
use crate::render;
use crate::session::require_auth;

fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

// ── GET /c/:cid/admin/members/:mid/remove ────────────────────────────────

pub async fn get_remove_member(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(env, &auth, community_id).await?;

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
    .await;

    let target =
        match membership_db::find_active_summary(&db, target_membership_id, community_id).await? {
            Some(target) => target,
            None => return render::not_found(),
        };

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
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">{rmt}</h1>\
         <p style=\"font-size:.9375rem;color:#6e6e73\">\
           <strong>{name}</strong><br>{consequence}\
         </p>\
         <div style=\"display:flex;gap:.75rem;margin-top:1.5rem\">\
           <a href=\"/c/{cid}/admin/members\" \
              style=\"flex:1;padding:.875rem;border:2px solid #e5e5ea;border-radius:14px;\
              text-align:center;text-decoration:none;color:#1D1D1F;font-weight:600\">\
              {keep}</a>\
           <form method=\"post\" \
             action=\"/c/{cid}/admin/members/{mid}/remove\" style=\"flex:1\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               style=\"width:100%;padding:.875rem;background:#FF3B30;color:#fff;\
               border:none;border-radius:14px;font-weight:600;min-height:44px;cursor:pointer\">\
               {confirm}</button>\
           </form>\
         </div></main>{nav}",
        header = render::header_with_switcher(
            i18n::JA_ADMIN_REMOVE_TITLE,
            community_id,
            &_community_pairs
        ),
        name = render::escape_html(&target.display_name),
        cid = render::escape_html(community_id),
        mid = render::escape_html(target_membership_id),
        tok = render::escape_html(&token),
        nav = nav,
        rmt = i18n::JA_ADMIN_REMOVE_TITLE,
        consequence = i18n::JA_ADMIN_REMOVE_CONSEQUENCE,
        keep = i18n::JA_ADMIN_REMOVE_KEEP,
        confirm = i18n::JA_ADMIN_REMOVE_CONFIRM,
    );
    render::page(i18n::JA_ADMIN_REMOVE_TITLE, &body)
}

// ── POST /c/:cid/admin/members/:mid/remove ───────────────────────────────

pub async fn post_remove_member(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(env, &auth, community_id).await?;

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
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/admin/members"));
    }

    match membership_db::soft_remove_guarded(&db, target_membership_id, community_id).await? {
        membership_db::RemoveMemberResult::Removed => {
            let _ = audit::write(
                &db,
                rid,
                Some(community_id),
                Some(&membership.membership_id),
                "membership",
                Some(target_membership_id),
                "removed",
                None,
            )
            .await;
            redirect(&format!("/c/{community_id}/admin/members"))
        }
        membership_db::RemoveMemberResult::LastAdminBlocked => render::page(
            i18n::JA_GENERAL_ERROR,
            &format!(
                "<main style=\"padding:2rem\"><p>{}</p></main>",
                i18n::JA_ADMIN_LAST_ADMIN
            ),
        ),
        membership_db::RemoveMemberResult::InvalidTarget => render::not_found(),
    }
}
