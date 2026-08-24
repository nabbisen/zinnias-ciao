//! Admin member suspend/unsuspend handlers — RFC-082.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::Locale;
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

fn last_admin_suspend_page(community_id: &str, locale: Locale) -> Result<Response> {
    render::page_localized(
        locale,
        i18n::t(locale, i18n::GENERAL_ERROR),
        &format!(
            "<main class=\"cz-admin-error-main\"><p>{}</p>\
             <p><a href=\"/c/{cid}/admin/members\" \
             class=\"cz-plain-link\">{back}</a></p></main>",
            i18n::t(locale, i18n::ADMIN_LAST_ADMIN_SUSPEND),
            cid = render::escape_html(community_id),
            back = i18n::t(locale, i18n::ADMIN_INVITES_BACK_TO_MEMBERS),
        ),
    )
}

struct SuspensionConfirm<'a> {
    title: i18n::Localized,
    consequence: i18n::Localized,
    confirm: i18n::Localized,
    action: &'a str,
    token_purpose: &'a str,
    /// The target must currently be suspended (unsuspend) or not (suspend)
    /// — mirrors `role_transfer.rs`'s `expected_role` filter.
    expect_suspended: bool,
}

enum SuspensionMutation {
    Suspend,
    Unsuspend,
}

// ── GET /c/:cid/admin/members/:mid/suspend|unsuspend ────────────────────

async fn get_suspension_confirm(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
    cfg: SuspensionConfirm<'_>,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
    let locale = membership.locale;

    if target_membership_id == membership.membership_id {
        return render::not_found();
    }

    let db = env.d1("DB")?;
    let target = membership_db::find_present_summary(&db, target_membership_id, community_id)
        .await?
        .filter(|m| m.suspended_at.is_some() == cfg.expect_suspended);
    let target = match target {
        Some(target) => target,
        None => return render::not_found(),
    };
    if !cfg.expect_suspended
        && target.role == "admin"
        && membership_db::count_admins(&db, community_id).await? <= 1
    {
        return last_admin_suspend_page(community_id, locale);
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
    let nav = render::bottom_nav_localized(community_id, "home", locale);

    let title = i18n::t(locale, cfg.title);
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
        header = render::header_with_switcher_next_localized(
            title,
            community_id,
            &community_pairs,
            "admin_members",
            locale,
        ),
        title = title,
        name = render::escape_html(&target.display_name),
        consequence = i18n::t(locale, cfg.consequence),
        cid = render::escape_html(community_id),
        action = render::escape_html(cfg.action),
        tok = render::escape_html(&token),
        keep = i18n::t(locale, i18n::ADMIN_REMOVE_KEEP),
        confirm = i18n::t(locale, cfg.confirm),
        nav = nav,
    );
    render::page_localized(locale, title, &body)
}

pub async fn get_suspend_member(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let action = format!(
        "/c/{}/admin/members/{}/suspend",
        community_id, target_membership_id
    );
    get_suspension_confirm(
        req,
        env,
        rid,
        community_id,
        target_membership_id,
        SuspensionConfirm {
            title: i18n::ADMIN_SUSPEND_TITLE,
            consequence: i18n::ADMIN_SUSPEND_CONSEQUENCE,
            confirm: i18n::ADMIN_SUSPEND_ACTION,
            action: &action,
            token_purpose: token_purpose::SUSPEND_MEMBER,
            expect_suspended: false,
        },
    )
    .await
}

pub async fn get_unsuspend_member(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let action = format!(
        "/c/{}/admin/members/{}/unsuspend",
        community_id, target_membership_id
    );
    get_suspension_confirm(
        req,
        env,
        rid,
        community_id,
        target_membership_id,
        SuspensionConfirm {
            title: i18n::ADMIN_UNSUSPEND_TITLE,
            consequence: i18n::ADMIN_UNSUSPEND_CONSEQUENCE,
            confirm: i18n::ADMIN_UNSUSPEND_ACTION,
            action: &action,
            token_purpose: token_purpose::UNSUSPEND_MEMBER,
            expect_suspended: true,
        },
    )
    .await
}

// ── POST /c/:cid/admin/members/:mid/suspend|unsuspend ───────────────────

async fn post_suspension(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
    mutation: SuspensionMutation,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;

    if target_membership_id == membership.membership_id {
        return render::not_found();
    }

    let db = env.d1("DB")?;
    let purpose = match mutation {
        SuspensionMutation::Suspend => token_purpose::SUSPEND_MEMBER,
        SuspensionMutation::Unsuspend => token_purpose::UNSUSPEND_MEMBER,
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

    match mutation {
        SuspensionMutation::Suspend => {
            match membership_db::suspend_required(
                &db,
                rid,
                target_membership_id,
                community_id,
                &membership.membership_id,
            )
            .await?
            {
                membership_db::SuspendResult::Suspended
                | membership_db::SuspendResult::AlreadySuspended => {
                    redirect(&format!("/c/{community_id}/admin/members"))
                }
                membership_db::SuspendResult::LastAdminBlocked => {
                    last_admin_suspend_page(community_id, membership.locale)
                }
                membership_db::SuspendResult::InvalidTarget => render::not_found(),
            }
        }
        SuspensionMutation::Unsuspend => {
            match membership_db::unsuspend_required(
                &db,
                rid,
                target_membership_id,
                community_id,
                &membership.membership_id,
            )
            .await?
            {
                membership_db::UnsuspendResult::Unsuspended
                | membership_db::UnsuspendResult::AlreadyActive => {
                    redirect(&format!("/c/{community_id}/admin/members"))
                }
                membership_db::UnsuspendResult::InvalidTarget => render::not_found(),
            }
        }
    }
}

pub async fn post_suspend_member(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    post_suspension(
        req,
        env,
        rid,
        community_id,
        target_membership_id,
        SuspensionMutation::Suspend,
    )
    .await
}

pub async fn post_unsuspend_member(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    post_suspension(
        req,
        env,
        rid,
        community_id,
        target_membership_id,
        SuspensionMutation::Unsuspend,
    )
    .await
}
