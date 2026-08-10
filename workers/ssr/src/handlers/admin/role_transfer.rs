//! Admin role-transfer handlers — RFC-062.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz::require_admin;
use crate::db::membership as membership_db;
use crate::render;

fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

async fn community_pairs_for_user(
    db: &worker::D1Database,
    user_id: &str,
    scope_community_id: Option<&str>,
) -> Vec<(String, String)> {
    membership_db::list_communities_for_user(db, user_id, scope_community_id)
        .await
        .unwrap_or_default()
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect()
}

fn last_admin_demote_page(community_id: &str) -> Result<Response> {
    render::page(
        i18n::JA_GENERAL_ERROR,
        &format!(
            "<main class=\"cz-admin-error-main\"><p>{}</p>\
             <p><a href=\"/c/{cid}/admin/members\" \
             class=\"cz-plain-link\">{back}</a></p></main>",
            i18n::JA_ADMIN_LAST_ADMIN_DEMOTE,
            cid = render::escape_html(community_id),
            back = i18n::JA_ADMIN_INVITES_BACK_TO_MEMBERS,
        ),
    )
}

struct RoleChangeConfirm<'a> {
    title: &'a str,
    consequence: &'a str,
    confirm: &'a str,
    action: &'a str,
    token_purpose: &'a str,
    expected_role: &'a str,
}

enum RoleMutation {
    Promote,
    Demote,
}

// ── GET /c/:cid/admin/members/:mid/promote|demote ───────────────────────

async fn get_role_change_confirm(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
    cfg: RoleChangeConfirm<'_>,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;

    if target_membership_id == membership.membership_id {
        return render::not_found();
    }

    let db = env.d1("DB")?;
    let target = membership_db::find_active_summary(&db, target_membership_id, community_id)
        .await?
        .filter(|m| m.role == cfg.expected_role);
    let target = match target {
        Some(target) => target,
        None => return render::not_found(),
    };
    if cfg.expected_role == "admin" && membership_db::count_admins(&db, community_id).await? <= 1 {
        return last_admin_demote_page(community_id);
    }

    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        cfg.token_purpose,
        Some(target_membership_id),
    )
    .await?;
    let community_pairs =
        community_pairs_for_user(&db, &auth.user_id, auth.scope_community_id.as_deref()).await;
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
         <h1 class=\"cz-admin-title cz-admin-title--snug\">{title}</h1>\
         <p class=\"cz-admin-confirm-subtitle\">\
           <strong>{name}</strong><br>{consequence}\
         </p>\
         <div class=\"cz-admin-role-actions\">\
           <a href=\"/c/{cid}/admin/members\" \
              class=\"cz-admin-role-keep-link\">\
              {keep}</a>\
           <form method=\"post\" action=\"{action}\" class=\"cz-confirm-delete-form\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               class=\"cz-admin-role-confirm-button\">\
               {confirm}</button>\
           </form>\
         </div></main>{nav}",
        header = render::header_with_switcher_next(
            cfg.title,
            community_id,
            &community_pairs,
            "admin_members"
        ),
        title = cfg.title,
        name = render::escape_html(&target.display_name),
        consequence = cfg.consequence,
        cid = render::escape_html(community_id),
        action = render::escape_html(cfg.action),
        tok = render::escape_html(&token),
        keep = i18n::JA_ADMIN_REMOVE_KEEP,
        confirm = cfg.confirm,
        nav = nav,
    );
    render::page(cfg.title, &body)
}

pub async fn get_promote_member(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let action = format!(
        "/c/{}/admin/members/{}/promote",
        community_id, target_membership_id
    );
    get_role_change_confirm(
        req,
        env,
        rid,
        community_id,
        target_membership_id,
        RoleChangeConfirm {
            title: i18n::JA_ADMIN_PROMOTE_TITLE,
            consequence: i18n::JA_ADMIN_PROMOTE_CONSEQUENCE,
            confirm: i18n::JA_ADMIN_PROMOTE_ACTION,
            action: &action,
            token_purpose: token_purpose::PROMOTE_MEMBER,
            expected_role: "member",
        },
    )
    .await
}

pub async fn get_demote_member(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let action = format!(
        "/c/{}/admin/members/{}/demote",
        community_id, target_membership_id
    );
    get_role_change_confirm(
        req,
        env,
        rid,
        community_id,
        target_membership_id,
        RoleChangeConfirm {
            title: i18n::JA_ADMIN_DEMOTE_TITLE,
            consequence: i18n::JA_ADMIN_DEMOTE_CONSEQUENCE,
            confirm: i18n::JA_ADMIN_DEMOTE_ACTION,
            action: &action,
            token_purpose: token_purpose::DEMOTE_MEMBER,
            expected_role: "admin",
        },
    )
    .await
}

// ── POST /c/:cid/admin/members/:mid/promote|demote ──────────────────────

async fn post_role_change(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
    mutation: RoleMutation,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;

    if target_membership_id == membership.membership_id {
        return render::not_found();
    }

    let db = env.d1("DB")?;
    let purpose = match mutation {
        RoleMutation::Promote => token_purpose::PROMOTE_MEMBER,
        RoleMutation::Demote => token_purpose::DEMOTE_MEMBER,
    };
    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        purpose,
        &raw_token,
        Some(target_membership_id),
    )
    .await?;
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/admin/members"));
    }

    let result = match mutation {
        RoleMutation::Promote => {
            membership_db::promote_to_admin_required(
                &db,
                rid,
                target_membership_id,
                community_id,
                &membership.membership_id,
            )
            .await?
        }
        RoleMutation::Demote => {
            membership_db::demote_to_member_required(
                &db,
                rid,
                target_membership_id,
                community_id,
                &membership.membership_id,
            )
            .await?
        }
    };

    match result {
        membership_db::RoleUpdateResult::Changed => {
            redirect(&format!("/c/{community_id}/admin/members"))
        }
        membership_db::RoleUpdateResult::AlreadyApplied => {
            redirect(&format!("/c/{community_id}/admin/members"))
        }
        membership_db::RoleUpdateResult::LastAdminBlocked => last_admin_demote_page(community_id),
        membership_db::RoleUpdateResult::InvalidTarget => render::not_found(),
    }
}

pub async fn post_promote_member(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    post_role_change(
        req,
        env,
        rid,
        community_id,
        target_membership_id,
        RoleMutation::Promote,
    )
    .await
}

pub async fn post_demote_member(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    post_role_change(
        req,
        env,
        rid,
        community_id,
        target_membership_id,
        RoleMutation::Demote,
    )
    .await
}
