//! Admin member handlers — invite codes and member management (RFC-010).

use zinnias_ciao_contracts::auth::token_purpose;
use worker::{Env, Request, Response, Result};

use crate::audit;
use crate::authz::require_admin;
#[cfg(not(target_arch = "wasm32"))]
use crate::crypto::{hmac_hex, normalize_invite_code};
use crate::crypto::random_token;
use crate::db::{self, invite as invite_db, membership as membership_db};
use crate::form_token;
use crate::render;
use crate::session::require_auth;
use zinnias_ciao_contracts::i18n;

fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

// ── GET /c/:cid/admin/invites ────────────────────────────────────────────

pub async fn get_invites(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);
    let gen_token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::GENERATE_INVITE, None).await.unwrap_or_default();

    let communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let community_pairs: Vec<(String,String)> = communities_for_switcher.iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone())).collect();

    let url = req.url()?;
    let new_code: Option<String> = url.query_pairs()
        .find(|(k, _)| k == "code").map(|(_, v)| v.to_string());
    let flash: Option<String> = url.query_pairs()
        .find(|(k, _)| k == "flash").map(|(_, v)| v.to_string());

    let _invite_code_lbl = i18n::JA_ADMIN_INVITES_TITLE;
    let _invite_share_hint = i18n::JA_ADMIN_INVITES_NEW_CODE_HINT;
    let new_code_html = new_code.as_deref().map(|c| format!(
        "<div style=\"background:#edfaf0;border-radius:12px;padding:1rem;margin:1rem 0;border:1px solid #34C759\">\
         <p style=\"font-size:.8125rem;color:#167A34;margin:0 0 .5rem\">{hint}</p>\
         <div style=\"font-size:1.5rem;font-weight:700;letter-spacing:.2em;color:#1D1D1F\" aria-label=\"{lbl}\">{code}</div>\
         </div>",
        hint = _invite_share_hint,
        lbl  = _invite_code_lbl,
        code = render::escape_html(c)
    )).unwrap_or_default();

    let flash_html = flash.map(|f| format!(
        "<p role=\"status\" style=\"font-size:.875rem;color:#167A34;margin:.5rem 0\">{}</p>",
        render::escape_html(&f)
    )).unwrap_or_default();

    // List active codes with per-row revoke tokens.
    let active_codes = invite_db::list_active_for_community(&db, community_id).await
        .unwrap_or_default();

    let mut code_rows = String::new();
    for inv in &active_codes {
        let revoke_tok = form_token::issue(&db, &pp, &auth.user_id,
            token_purpose::REVOKE_INVITE, Some(&inv.id)).await.unwrap_or_default();
        let role_label = if inv.grants_role == "admin" { i18n::JA_ROLE_ADMIN } else { "" };
        let rev = i18n::JA_ADMIN_INVITES_REVOKE;
        let exp_display = inv.expires_at.get(..16).unwrap_or(&inv.expires_at);
        code_rows.push_str(&format!(
            "<li style=\"display:flex;align-items:center;justify-content:space-between;\
             padding:.625rem 0;border-bottom:1px solid #f5f5f7;gap:.5rem\">\
             <span style=\"font-size:.875rem;color:#1D1D1F\">{exp}{role}</span>\
             <form method=\"post\" action=\"/c/{cid}/admin/invites/{iid}/revoke\" style=\"margin:0\">\
               <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
               <button type=\"submit\" \
                 style=\"font-size:.8125rem;color:#FF3B30;background:none;border:none;\
                 padding:.375rem .5rem;cursor:pointer;min-height:44px\" \
                 aria-label=\"{rev}\">{rev}</button>\
             </form></li>",
            exp  = render::escape_html(exp_display),
            role = role_label,
            cid  = render::escape_html(community_id),
            iid  = render::escape_html(&inv.id),
            tok  = render::escape_html(&revoke_tok),
        ));
    }
    let codes_html = if active_codes.is_empty() {
        format!("<p style=\"font-size:.875rem;color:#6e6e73\">{}</p>", i18n::JA_ADMIN_INVITES_NONE)
    } else {
        format!("<ul style=\"list-style:none;padding:0;margin:.75rem 0\">{code_rows}</ul>")
    };

    let nav = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">{title}</h1>\
         <p style=\"font-size:.875rem;color:#6e6e73\">{ib}</p>\
         {flash}{new_code}\
         <form method=\"post\" action=\"/c/{cid}/admin/invites\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           <button type=\"submit\" \
             style=\"width:100%;padding:.875rem;background:#007AFF;color:#fff;\
             border:none;border-radius:14px;font-size:1rem;font-weight:600;\
             min-height:44px;cursor:pointer;margin-top:.5rem\">{ig}</button>\
         </form>\
         <section style=\"margin-top:1.5rem\">\
           <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
             text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">{active_lbl}</h2>\
           {codes}\
         </section>\
         </main>{nav}",
        header      = render::header_with_switcher(i18n::JA_ADMIN_INVITES_TITLE, community_id, &community_pairs),
        cid         = render::escape_html(community_id),
        tok         = render::escape_html(&gen_token),
        new_code    = new_code_html,
        flash       = flash_html,
        codes       = codes_html,
        nav         = nav,
        title       = i18n::JA_ADMIN_INVITES_TITLE,
        ib          = i18n::JA_ADMIN_INVITES_BODY,
        ig          = i18n::JA_ADMIN_INVITES_GENERATE,
        active_lbl  = i18n::JA_ADMIN_INVITES_ACTIVE,
    );
    render::page(i18n::JA_ADMIN_INVITES_TITLE, &body)
}

// ── POST /c/:cid/admin/invites ───────────────────────────────────────────

pub async fn post_generate_invite(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(&db, &pp, &auth.user_id,
        token_purpose::GENERATE_INVITE, &raw_token, None).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/admin/invites"));
    }

    // ── codlet path (wasm32) ───────────────────────────────────────────────
    #[cfg(target_arch = "wasm32")]
    {
        use codlet_core::secret::CodeId;

        let mut mgrs = crate::codlet::build(env)
            .await
            .map_err(|e| worker::Error::RustError(format!("codlet: {e}")))?;

        // Generate a random CodeId; codlet generates the code internally.
        let invite_id = &random_token()[..24];
        let code_id   = CodeId::new(invite_id.to_owned().into());

        // issue_code: generates code, hashes it, inserts into codlet_codes.
        // scope = community_id; grant = "role:member" (admin invites are member by default).
        let (_record, plain_code) = mgrs.code_auth.issue_code(
            &mut mgrs.rng,
            code_id,
            Some("invite".to_owned()),
            Some(community_id.to_owned()),            // scope
            Some("role:member".to_owned()),           // grant_payload
        ).await.map_err(|e| worker::Error::RustError(format!("issue_code: {e}")))?;

        let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
            "invite_code", Some(invite_id), "generated", None).await;

        let code = plain_code.expose().to_owned();
        return redirect(&format!("/c/{community_id}/admin/invites?code={code}"));
    }

    // ── legacy fallback (non-wasm / native tests) ──────────────────────────
    #[cfg(not(target_arch = "wasm32"))]
    {
        use zinnias_ciao_domain::invite::{INVITE_CODE_ALPHABET, INVITE_CODE_LEN};

        // Inline rejection-sampling generator for the non-wasm path (tests).
        let alpha_len = INVITE_CODE_ALPHABET.len();
        let ceiling   = 256 - (256 % alpha_len);
        let mut code  = String::with_capacity(INVITE_CODE_LEN);
        while code.len() < INVITE_CODE_LEN {
            let mut buf = [0u8; 1];
            getrandom::fill(&mut buf)
                .map_err(|e| worker::Error::RustError(format!("rng: {e}")))?;
            let b = buf[0] as usize;
            if b < ceiling {
                code.push(INVITE_CODE_ALPHABET[b % alpha_len] as char);
            }
        }
        let normalized = normalize_invite_code(&code);
        let code_hmac  = hmac_hex(&pp, &normalized);
        let invite_id  = random_token()[..24].to_owned();
        let expires_at = db::add_seconds_to_now(86_400);
        invite_db::insert(&db, &invite_id, community_id, &code_hmac,
            &membership.membership_id, &expires_at, "member").await?;
        let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
            "invite_code", Some(&invite_id), "generated", None).await;
        redirect(&format!("/c/{community_id}/admin/invites?code={code}"))
    }
}

// ── POST /c/:cid/admin/invites/:iid/revoke ───────────────────────────────

pub async fn post_revoke_invite(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    invite_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(&db, &pp, &auth.user_id,
        token_purpose::REVOKE_INVITE, &raw_token, Some(invite_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/admin/invites"));
    }

    invite_db::revoke(&db, invite_id, community_id).await?;

    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "invite_code", Some(invite_id), "revoked", None).await;

    redirect(&format!("/c/{community_id}/admin/invites?flash=Code+revoked"))
}

// ── GET /c/:cid/admin/members ────────────────────────────────────────────

pub async fn get_members(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let community = db::community::find_active(&db, community_id).await?;
    let _community_name = community.map(|c| c.name).unwrap_or_default();
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();

    let members = membership_db::list_all_active(&db, community_id).await?;
    let member_rows: String = members.iter().map(|m| {
        let is_self = m.id == membership.membership_id;
        let remove_btn = if is_self {
            String::new() // cannot remove yourself
        } else {
            format!(
                "<a href=\"/c/{cid}/admin/members/{mid}/remove\" \
                 style=\"color:#FF3B30;font-size:.875rem\">{rc}</a>",
                cid = render::escape_html(community_id),
                mid = render::escape_html(&m.id),
                rc  = i18n::JA_ADMIN_REMOVE_CONFIRM,
            )
        };
        format!(
            "<li style=\"display:flex;align-items:center;justify-content:space-between;\
             padding:.75rem 0;border-bottom:1px solid #f5f5f7\">\
             <span style=\"font-size:.9375rem\">{name}</span>\
             {remove}\
             </li>",
            name   = render::escape_html(&m.display_name),
            remove = remove_btn,
        )
    }).collect();

    let nav  = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">{members_h1}</h1>\
         <ul style=\"list-style:none;padding:0;margin:0\">{rows}</ul>\
         <a href=\"/c/{cid}/admin/invites\" \
            style=\"display:block;margin-top:1.5rem;text-align:center;\
            padding:.875rem;border:2px solid #007AFF;border-radius:14px;\
            color:#007AFF;text-decoration:none;font-weight:600\">\
            Generate invite code</a>\
         </main>{nav}",
        header = render::header_with_switcher(i18n::JA_ADMIN_MEMBERS_TITLE, community_id, &_community_pairs),
        rows   = member_rows,
        cid    = render::escape_html(community_id),
        nav    = nav,
        members_h1 = i18n::JA_ADMIN_MEMBERS_TITLE,
    );
    render::page(i18n::JA_ADMIN_MEMBERS_TITLE, &body)
}

// ── GET /c/:cid/admin/members/:mid/remove ────────────────────────────────

pub async fn get_remove_member(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;

    // Cannot remove yourself
    if target_membership_id == membership.membership_id {
        return render::not_found();
    }

    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::REMOVE_MEMBER, Some(target_membership_id)).await.unwrap_or_default();

    // Find the target member name
    let all = membership_db::list_all_active(&db, community_id).await?;
    let target_name = all.iter()
        .find(|m| m.id == target_membership_id)
        .map(|m| m.display_name.as_str())
        .unwrap_or("this member");

    let community = db::community::find_active(&db, community_id).await?;
    let _community_name = community.map(|c| c.name).unwrap_or_default();
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
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
        header      = render::header_with_switcher(i18n::JA_ADMIN_REMOVE_TITLE, community_id, &_community_pairs),
        name        = render::escape_html(target_name),
        cid         = render::escape_html(community_id),
        mid         = render::escape_html(target_membership_id),
        tok         = render::escape_html(&token),
        nav         = nav,
        rmt         = i18n::JA_ADMIN_REMOVE_TITLE,
        consequence = i18n::JA_ADMIN_REMOVE_CONSEQUENCE,
        keep        = i18n::JA_ADMIN_REMOVE_KEEP,
        confirm     = i18n::JA_ADMIN_REMOVE_CONFIRM,
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
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;

    if target_membership_id == membership.membership_id {
        return render::not_found();
    }

    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(&db, &pp, &auth.user_id,
        token_purpose::REMOVE_MEMBER, &raw_token, Some(target_membership_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/admin/members"));
    }

    // Last-admin guard (RFC-010 §5)
    let admin_count = membership_db::count_admins(&db, community_id).await?;
    let target_role = membership_db::get_role(&db, target_membership_id, community_id).await?;
    if admin_count <= 1 && target_role.as_deref() == Some("admin") {
        return render::page(i18n::JA_GENERAL_ERROR,
            &format!("<main style=\"padding:2rem\"><p>{}</p></main>",
                i18n::JA_ADMIN_LAST_ADMIN));
    }

    membership_db::soft_remove(&db, target_membership_id, community_id).await?;
    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "membership", Some(target_membership_id), "removed", None).await;

    redirect(&format!("/c/{community_id}/admin/members"))
}

// ── Helpers ───────────────────────────────────────────────────────────────

// generate_invite_code() removed — codlet CodeAuth::issue_code() handles
// generation with fail-closed RNG and rejection sampling (INV-3, RFC-003 §4).
// Called via crate::codlet::build(env) in post_generate_invite.
