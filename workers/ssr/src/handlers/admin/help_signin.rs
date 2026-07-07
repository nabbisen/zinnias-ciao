//! Admin help-signin handlers — RFC-024.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::audit;
use crate::authz::require_admin;
use crate::crypto::{hmac_hex, normalize_invite_code, random_token};
use crate::db::{membership as membership_db, relink as relink_db};
use crate::render;
use crate::session::require_auth;

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
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
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
    .await;
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
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
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
    let code_hmac = hmac_hex(&crate::crypto::pepper(env), &normalized);
    let code_id = random_token()[..24].to_owned();
    let expires_at = relink_db::expires_at();

    relink_db::revoke_unused_for_membership(&db, target_membership_id).await?;
    relink_db::insert(
        &db,
        &code_id,
        &code_hmac,
        community_id,
        target_membership_id,
        &membership.membership_id,
        &expires_at,
    )
    .await?;

    let _ = audit::write(
        &db,
        rid,
        Some(community_id),
        Some(&membership.membership_id),
        "membership",
        Some(target_membership_id),
        "membership.relink_code_created",
        Some(serde_json::json!({
            "membership_id": target_membership_id,
            "created_by_membership_id": membership.membership_id,
            "community_id": community_id,
        })),
    )
    .await;

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
             overflow-wrap:anywhere\" aria-label=\"{code_label}\">{code}</div>\
         </div>\
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
        cid = render::escape_html(community_id),
        back = i18n::JA_ADMIN_INVITES_BACK_TO_MEMBERS,
        nav = nav,
    );
    render::page(i18n::JA_ADMIN_HELP_SIGNIN_TITLE, &body)
}
