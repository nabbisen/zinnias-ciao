//! Admin help-signin handlers — RFC-024.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz::require_admin;
use crate::crypto::{hmac_hex, normalize_invite_code, random_token};
use crate::db::{membership as membership_db, relink as relink_db};
use crate::render;

fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

async fn community_pairs_for_user(db: &worker::D1Database, user_id: &str) -> Vec<(String, String)> {
    membership_db::list_communities_for_user(db, user_id)
        .await
        .unwrap_or_default()
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect()
}

// ── GET /c/:cid/admin/members/:mid/help-signin ───────────────────────────

pub async fn get_help_signin(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let _membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let target =
        match membership_db::find_active_summary(&db, target_membership_id, community_id).await? {
            Some(target) => target,
            None => return render::not_found(),
        };
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::HELP_SIGNIN,
        Some(target_membership_id),
    )
    .await?;
    let community_pairs = community_pairs_for_user(&db, &auth.user_id).await;
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">{title}</h1>\
         <p style=\"font-size:.9375rem;color:#6e6e73\">\
           <strong>{name}</strong><br>{consequence}\
         </p>\
         <div style=\"display:flex;gap:.75rem;margin-top:1.5rem\">\
           <a href=\"/c/{cid}/admin/members\" \
              style=\"flex:1;padding:.875rem;border:2px solid #e5e5ea;border-radius:14px;\
              text-align:center;text-decoration:none;color:#1D1D1F;font-weight:600\">\
              {keep}</a>\
           <form method=\"post\" action=\"/c/{cid}/admin/members/{mid}/help-signin\" style=\"flex:1\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               style=\"width:100%;padding:.875rem;background:#007AFF;color:#fff;\
               border:none;border-radius:14px;font-weight:600;min-height:44px;cursor:pointer\">\
               {confirm}</button>\
           </form>\
         </div></main>{nav}",
        header = render::header_with_switcher(
            i18n::JA_ADMIN_HELP_SIGNIN_TITLE,
            community_id,
            &community_pairs
        ),
        title = i18n::JA_ADMIN_HELP_SIGNIN_TITLE,
        name = render::escape_html(&target.display_name),
        consequence = i18n::JA_ADMIN_HELP_SIGNIN_CONSEQUENCE,
        cid = render::escape_html(community_id),
        mid = render::escape_html(target_membership_id),
        tok = render::escape_html(&token),
        keep = i18n::JA_ADMIN_REMOVE_KEEP,
        confirm = i18n::JA_ADMIN_HELP_SIGNIN_CREATE,
        nav = nav,
    );
    render::page(i18n::JA_ADMIN_HELP_SIGNIN_TITLE, &body)
}

// ── POST /c/:cid/admin/members/:mid/help-signin ──────────────────────────

pub async fn post_help_signin(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let target =
        match membership_db::find_active_summary(&db, target_membership_id, community_id).await? {
            Some(target) => target,
            None => return render::not_found(),
        };

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::HELP_SIGNIN,
        &raw_token,
        Some(target_membership_id),
    )
    .await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/admin/members"));
    }

    let code = random_token()[..16].to_ascii_uppercase();
    let normalized = normalize_invite_code(&code);
    let pepper = crate::crypto::pepper(env)?;
    let code_hmac = hmac_hex(pepper.as_str(), &normalized);
    let code_id = random_token()[..24].to_owned();
    let expires_at = relink_db::expires_at();

    if !relink_db::issue_required(
        &db,
        rid,
        &code_id,
        &code_hmac,
        community_id,
        target_membership_id,
        &membership.membership_id,
        &expires_at,
        None,
    )
    .await?
    {
        return render::not_found();
    }

    let nav = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">{title}</h1>\
         <p style=\"font-size:.9375rem;color:#6e6e73;margin-bottom:1rem\">\
           <strong>{name}</strong><br>{hint}</p>\
         <div style=\"background:#edfaf0;border:1px solid #34C759;border-radius:12px;\
             padding:1rem;margin:1rem 0\">\
           <div style=\"font-size:1.5rem;font-weight:700;letter-spacing:.16em;color:#1D1D1F;\
             overflow-wrap:anywhere\" aria-label=\"{code_label}\" data-copy-code-value=\"true\">{code}</div>\
           <button type=\"button\" data-copy-code-button=\"true\" hidden \
             data-copy-success=\"{copy_done}\" data-copy-error=\"{copy_failed}\" \
             style=\"margin-top:.75rem;padding:.625rem .875rem;background:#fff;color:#007AFF;\
             border:1px solid #007AFF;border-radius:8px;font-size:.9375rem;font-weight:600;\
             min-height:44px;cursor:pointer\">{copy_code}</button>\
           <span data-copy-code-status=\"true\" aria-live=\"polite\" \
             style=\"display:block;margin-top:.5rem;font-size:.8125rem;color:#167A34\"></span>\
         </div>\
         <p style=\"font-size:.9375rem;color:#6e6e73;margin:0 0 .75rem\">{relink_hint}</p>\
         <p><a href=\"/relink\" target=\"_blank\" rel=\"noopener\" \
           style=\"color:#007AFF;text-decoration:none\">{relink_link}</a></p>\
         <p><a href=\"/c/{cid}/admin/members\" \
           style=\"color:#007AFF;text-decoration:none\">{back}</a></p>\
         </main>{nav}",
        header = render::header_with_switcher(
            i18n::JA_ADMIN_HELP_SIGNIN_TITLE,
            community_id,
            &community_pairs_for_user(&db, &auth.user_id).await
        ),
        title = i18n::JA_ADMIN_HELP_SIGNIN_TITLE,
        name = render::escape_html(&target.display_name),
        hint = i18n::JA_ADMIN_HELP_SIGNIN_CODE_HINT,
        code_label = i18n::JA_RELINK_CODE_LABEL,
        code = render::escape_html(&code),
        copy_code = i18n::JA_ADMIN_HELP_SIGNIN_COPY_CODE,
        copy_done = i18n::JA_ADMIN_HELP_SIGNIN_COPY_DONE,
        copy_failed = i18n::JA_ADMIN_HELP_SIGNIN_COPY_FAILED,
        relink_hint = i18n::JA_ADMIN_HELP_SIGNIN_RELINK_HINT,
        relink_link = i18n::JA_ADMIN_HELP_SIGNIN_RELINK_LINK,
        cid = render::escape_html(community_id),
        back = i18n::JA_ADMIN_INVITES_BACK_TO_MEMBERS,
        nav = nav,
    );
    render::page(i18n::JA_ADMIN_HELP_SIGNIN_TITLE, &body)
}
