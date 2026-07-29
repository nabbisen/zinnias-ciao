//! Admin member handlers — invite codes and member management (RFC-010).

use std::ops::ControlFlow;

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;

use crate::authz::require_admin;
use crate::crypto::random_token;
use crate::crypto::{hmac_hex, normalize_invite_code};
use crate::db::invite as invite_db;
use crate::db::{self, membership as membership_db};
use crate::form_token::ConsumeResult;
use crate::render;
use zinnias_ciao_contracts::i18n;

fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

enum InviteGetPreflight {
    Continue,
    CanonicalRedirect(String),
}

struct InviteCodeReveal(String);

impl InviteCodeReveal {
    fn new(code: String) -> Self {
        Self(code)
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

fn canonical_invites_path(community_id: &str) -> String {
    let mut encoded = String::with_capacity(community_id.len());
    for byte in community_id.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-') {
            encoded.push(*byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    format!("/c/{encoded}/admin/invites")
}

fn decoded_query_key_is_code(raw_key: &str) -> bool {
    let bytes = raw_key.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let Some(high) = (bytes[index + 1] as char).to_digit(16) else {
                return false;
            };
            let Some(low) = (bytes[index + 2] as char).to_digit(16) else {
                return false;
            };
            decoded.push(((high << 4) | low) as u8);
            index += 3;
        } else {
            decoded.push(if bytes[index] == b'+' {
                b' '
            } else {
                bytes[index]
            });
            index += 1;
        }
    }
    decoded == b"code"
}

fn invite_get_preflight(raw_query: Option<&str>, community_id: &str) -> InviteGetPreflight {
    let has_code = raw_query.is_some_and(|query| {
        query.split('&').any(|pair| {
            let key = pair.split_once('=').map_or(pair, |(key, _)| key);
            decoded_query_key_is_code(key)
        })
    });
    if has_code {
        InviteGetPreflight::CanonicalRedirect(canonical_invites_path(community_id))
    } else {
        InviteGetPreflight::Continue
    }
}

fn run_invite_get_preflight<F, T>(
    preflight: InviteGetPreflight,
    continuation: F,
) -> ControlFlow<String, T>
where
    F: FnOnce() -> T,
{
    match preflight {
        InviteGetPreflight::Continue => ControlFlow::Continue(continuation()),
        InviteGetPreflight::CanonicalRedirect(location) => ControlFlow::Break(location),
    }
}

fn legacy_query_redirect(location: &str) -> Result<Response> {
    let mut response = Response::empty()?.with_status(303);
    response.headers_mut().set("Location", location)?;
    response
        .headers_mut()
        .set("Referrer-Policy", "no-referrer")?;
    Ok(response)
}

// ── GET /c/:cid/admin/invites ────────────────────────────────────────────

pub async fn get_invites(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let url = req.url()?;
    match run_invite_get_preflight(invite_get_preflight(url.query(), community_id), || {
        let flash = url
            .query_pairs()
            .any(|(key, value)| key == "flash" && value == "Code revoked")
            .then_some(i18n::JA_ADMIN_INVITES_REVOKED);
        get_invites_authenticated(req, env, community_id, flash)
    }) {
        ControlFlow::Break(location) => legacy_query_redirect(&location),
        ControlFlow::Continue(response) => response.await,
    }
}

async fn get_invites_authenticated(
    req: Request,
    env: &Env,
    community_id: &str,
    flash: Option<&'static str>,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let _membership = require_admin(env, &auth, community_id).await?;
    render_invites_page(env, community_id, &auth.user_id, flash, None).await
}

async fn render_invites_page(
    env: &Env,
    community_id: &str,
    user_id: &str,
    flash: Option<&'static str>,
    reveal: Option<&InviteCodeReveal>,
) -> Result<Response> {
    let db = env.d1("DB")?;
    let gen_token =
        crate::codlet::issue_token(env, user_id, token_purpose::GENERATE_INVITE, None).await?;

    let communities_for_switcher = membership_db::list_communities_for_user(&db, user_id)
        .await
        .unwrap_or_default();
    let community_pairs: Vec<(String, String)> = communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();

    let reveal_html = reveal.map(invite_reveal_html).unwrap_or_default();

    let flash_html = flash.map(|message| format!(
        "<p role=\"status\" style=\"font-size:.875rem;color:#167A34;margin:.5rem 0\">{}</p>",
        render::escape_html(message)
    )).unwrap_or_default();

    // List active invite codes from the service-owned invite table.
    let active_codes = crate::codlet::list_active_invites(env, community_id).await;

    let mut code_rows = String::new();
    for inv in &active_codes {
        let revoke_tok =
            crate::codlet::issue_token(env, user_id, token_purpose::REVOKE_INVITE, Some(&inv.id))
                .await?;
        let role_label = if inv.grants_role == "admin" {
            i18n::JA_ROLE_ADMIN
        } else {
            ""
        };
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
        format!(
            "<p style=\"font-size:.875rem;color:#6e6e73\">{}</p>",
            i18n::JA_ADMIN_INVITES_NONE
        )
    } else {
        format!("<ul style=\"list-style:none;padding:0;margin:.75rem 0\">{code_rows}</ul>")
    };

    let nav = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <p style=\"margin:0 0 1rem\"><a href=\"/c/{cid}/admin/members\" \
            style=\"font-size:.875rem;color:#007AFF;text-decoration:none\">\
            {back_to_members}</a></p>\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">{title}</h1>\
         <p style=\"font-size:.875rem;color:#6e6e73\">{ib}</p>\
         {flash}{reveal}\
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
        header = render::header_with_switcher_next(
            i18n::JA_ADMIN_INVITES_TITLE,
            community_id,
            &community_pairs,
            "admin_invites"
        ),
        cid = render::escape_html(community_id),
        tok = render::escape_html(&gen_token),
        reveal = reveal_html,
        flash = flash_html,
        codes = codes_html,
        nav = nav,
        title = i18n::JA_ADMIN_INVITES_TITLE,
        ib = i18n::JA_ADMIN_INVITES_BODY,
        ig = i18n::JA_ADMIN_INVITES_GENERATE,
        active_lbl = i18n::JA_ADMIN_INVITES_ACTIVE,
        back_to_members = i18n::JA_ADMIN_INVITES_BACK_TO_MEMBERS,
    );
    render::page(i18n::JA_ADMIN_INVITES_TITLE, &body)
}

fn invite_reveal_html(reveal: &InviteCodeReveal) -> String {
    format!(
        "<section id=\"invite-code-reveal\" \
           style=\"background:#edfaf0;border-radius:12px;padding:1rem;margin:1rem 0;\
           border:1px solid #34C759\">\
           <p style=\"font-size:.8125rem;color:#167A34;margin:0 0 .5rem\">{warning}</p>\
           <p style=\"font-size:.8125rem;color:#167A34;margin:0 0 .5rem\">{hint}</p>\
           <div style=\"font-size:1.5rem;font-weight:700;letter-spacing:.2em;color:#1D1D1F\" \
             aria-label=\"{label}\">{code}</div>\
         </section>",
        warning = i18n::JA_ADMIN_INVITES_REVEAL_WARNING,
        hint = i18n::JA_ADMIN_INVITES_NEW_CODE_HINT,
        label = i18n::JA_ADMIN_INVITES_TITLE,
        code = render::escape_html(reveal.as_str()),
    )
}

// ── POST /c/:cid/admin/invites ───────────────────────────────────────────

pub async fn post_generate_invite(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env)?;

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let consume = crate::form_token::consume_detailed(
        &db,
        pp.as_str(),
        &auth.user_id,
        token_purpose::GENERATE_INVITE,
        &raw_token,
        None,
    )
    .await?;
    if matches!(consume, ConsumeResult::Replay(_)) {
        return redirect(&canonical_invites_path(community_id));
    }

    use zinnias_ciao_domain::invite::{INVITE_CODE_ALPHABET, INVITE_CODE_LEN};

    let alpha_len = INVITE_CODE_ALPHABET.len();
    let ceiling = 256 - (256 % alpha_len);
    let mut code = String::with_capacity(INVITE_CODE_LEN);
    while code.len() < INVITE_CODE_LEN {
        let mut buf = [0u8; 1];
        getrandom::fill(&mut buf).map_err(|e| worker::Error::RustError(format!("rng: {e}")))?;
        let b = buf[0] as usize;
        if b < ceiling {
            code.push(INVITE_CODE_ALPHABET[b % alpha_len] as char);
        }
    }
    let normalized = normalize_invite_code(&code);
    let code_hmac = hmac_hex(pp.as_str(), &normalized);
    let invite_id = random_token()[..24].to_owned();
    let expires_at = db::add_seconds_to_now(86_400);
    let created = match invite_db::insert_required(
        &db,
        rid,
        &invite_id,
        community_id,
        &code_hmac,
        &membership.membership_id,
        &expires_at,
        "member",
    )
    .await
    {
        Ok(created) => created,
        Err(_) => return render::service_unavailable(),
    };
    if !created {
        return render::not_found();
    }
    let reveal = InviteCodeReveal::new(code);
    let mut response =
        match render_invites_page(env, community_id, &auth.user_id, None, Some(&reveal)).await {
            Ok(response) => response,
            Err(_) => return render::service_unavailable(),
        };
    response
        .headers_mut()
        .set("Cache-Control", "no-store, private")?;
    response
        .headers_mut()
        .set("Referrer-Policy", "no-referrer")?;
    Ok(response)
}

// ── POST /c/:cid/admin/invites/:iid/revoke ───────────────────────────────

pub async fn post_revoke_invite(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    invite_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::REVOKE_INVITE,
        &raw_token,
        Some(invite_id),
    )
    .await?;
    if matches!(replay, ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/admin/invites"));
    }

    invite_db::revoke_required(&db, rid, invite_id, community_id, &membership.membership_id)
        .await?;

    redirect(&format!(
        "/c/{community_id}/admin/invites?flash=Code+revoked"
    ))
}

// ── GET /c/:cid/admin/members ────────────────────────────────────────────

pub async fn get_members(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let community = db::community::find_active(&db, community_id).await?;
    let _community_name = community.map(|c| c.name).unwrap_or_default();
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await
        .unwrap_or_default();
    let _community_pairs: Vec<(String, String)> = _communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();

    let members = membership_db::list_all_active(&db, community_id).await?;
    let member_rows: String = members
        .iter()
        .map(|m| {
            let is_self = m.id == membership.membership_id;
            let role_action = if is_self {
                String::new()
            } else if m.role == "admin" {
                format!(
                    "<a href=\"/c/{cid}/admin/members/{mid}/demote\" \
                 style=\"display:block;color:#007AFF;font-size:.875rem;min-height:44px;\
                 line-height:44px;text-align:right\">{label}</a>",
                    cid = render::escape_html(community_id),
                    mid = render::escape_html(&m.id),
                    label = i18n::JA_ADMIN_DEMOTE_ACTION,
                )
            } else {
                format!(
                    "<a href=\"/c/{cid}/admin/members/{mid}/promote\" \
                 style=\"display:block;color:#007AFF;font-size:.875rem;min-height:44px;\
                 line-height:44px;text-align:right\">{label}</a>",
                    cid = render::escape_html(community_id),
                    mid = render::escape_html(&m.id),
                    label = i18n::JA_ADMIN_PROMOTE_ACTION,
                )
            };
            let help_action = format!(
                "<a href=\"/c/{cid}/admin/members/{mid}/help-signin\" \
                 style=\"display:block;color:#007AFF;font-size:.875rem;min-height:44px;\
                 line-height:44px;text-align:right\">{label}</a>",
                cid = render::escape_html(community_id),
                mid = render::escape_html(&m.id),
                label = i18n::JA_ADMIN_HELP_SIGNIN_ACTION,
            );
            let remove_action = if is_self {
                String::new()
            } else {
                format!(
                    "<a href=\"/c/{cid}/admin/members/{mid}/remove\" \
                 style=\"display:block;color:#FF3B30;font-size:.875rem;min-height:44px;\
                 line-height:44px;text-align:right\">{rc}</a>",
                    cid = render::escape_html(community_id),
                    mid = render::escape_html(&m.id),
                    rc = i18n::JA_ADMIN_REMOVE_CONFIRM,
                )
            };
            let role_label = if m.role == "admin" {
                i18n::JA_ROLE_ADMIN
            } else {
                i18n::JA_ROLE_MEMBER
            };
            let self_label = if is_self {
                format!(" · {}", i18n::JA_ADMIN_MEMBERS_CURRENT_USER)
            } else {
                String::new()
            };
            format!(
                "<li style=\"display:flex;align-items:center;justify-content:space-between;\
             padding:.75rem 0;border-bottom:1px solid #f5f5f7;gap:.75rem\">\
             <span style=\"min-width:0\">\
             <span style=\"display:block;font-size:.9375rem;overflow-wrap:anywhere\">{name}</span>\
             <span style=\"display:block;font-size:.8125rem;color:#6e6e73;margin-top:.125rem\">{role}{self_label}</span>\
             </span>\
             <span style=\"flex:0 0 auto\">{role_action}{help_action}{remove_action}</span>\
             </li>",
                name = render::escape_html(&m.display_name),
                role = role_label,
                self_label = self_label,
                role_action = role_action,
                help_action = help_action,
                remove_action = remove_action,
            )
        })
        .collect();

    let nav = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">{members_h1}</h1>\
         <ul style=\"list-style:none;padding:0;margin:0\">{rows}</ul>\
         <a href=\"/c/{cid}/admin/invites\" \
            style=\"display:block;margin-top:1.5rem;text-align:center;\
            padding:.875rem;border:2px solid #007AFF;border-radius:14px;\
            color:#007AFF;text-decoration:none;font-weight:600\">\
            {invite_label}</a>\
         </main>{nav}",
        header = render::header_with_switcher_next(
            i18n::JA_ADMIN_MEMBERS_TITLE,
            community_id,
            &_community_pairs,
            "admin_members"
        ),
        rows = member_rows,
        cid = render::escape_html(community_id),
        nav = nav,
        members_h1 = i18n::JA_ADMIN_MEMBERS_TITLE,
        invite_label = i18n::JA_ADMIN_MEMBERS_GENERATE_INVITE,
    );
    render::page(i18n::JA_ADMIN_MEMBERS_TITLE, &body)
}

// ── Helpers ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_invite_path_encodes_every_non_allowlisted_byte() {
        assert_eq!(
            canonical_invites_path("com_0123456789abcdef01234567"),
            "/c/com_0123456789abcdef01234567/admin/invites"
        );
        assert_eq!(
            canonical_invites_path("com_smoke_invite_redemption"),
            "/c/com_smoke_invite_redemption/admin/invites"
        );
        assert_eq!(
            canonical_invites_path("../a\\b"),
            "/c/%2E%2E%2Fa%5Cb/admin/invites"
        );
        assert_eq!(
            canonical_invites_path("%?#\r\n snow 雪"),
            "/c/%25%3F%23%0D%0A%20snow%20%E9%9B%AA/admin/invites"
        );
        for path in [
            canonical_invites_path("a/b"),
            canonical_invites_path("a?b"),
            canonical_invites_path("a#b"),
            canonical_invites_path("a\r\nb"),
        ] {
            assert!(path.starts_with("/c/") && path.ends_with("/admin/invites"));
            assert_eq!(path.matches('/').count(), 4);
            assert!(!path.contains('?'));
            assert!(!path.contains('#'));
            assert!(!path.contains('\r'));
            assert!(!path.contains('\n'));
        }
    }

    #[test]
    fn code_query_preflight_matches_empty_repeated_and_encoded_keys() {
        for query in [
            "code",
            "code=",
            "x=1&code=synthetic",
            "code=first&code=second",
            "%63ode=synthetic",
            "c%6Fde=synthetic",
        ] {
            assert!(matches!(
                invite_get_preflight(Some(query), "com_safe"),
                InviteGetPreflight::CanonicalRedirect(ref location)
                    if location == "/c/com_safe/admin/invites"
            ));
        }
        for query in [
            "",
            "Code=synthetic",
            "flash=Code+revoked",
            "xcode=synthetic",
            "%ZZcode=synthetic",
        ] {
            assert!(matches!(
                invite_get_preflight(Some(query), "com_safe"),
                InviteGetPreflight::Continue
            ));
        }
        assert!(matches!(
            invite_get_preflight(None, "com_safe"),
            InviteGetPreflight::Continue
        ));
    }

    #[test]
    fn legacy_query_preflight_never_invokes_authenticated_continuation() {
        let mut calls = 0;
        let result = run_invite_get_preflight(
            invite_get_preflight(Some("code=synthetic"), "com_safe"),
            || {
                calls += 1;
                "authenticated continuation"
            },
        );
        assert!(matches!(
            result,
            ControlFlow::Break(location) if location == "/c/com_safe/admin/invites"
        ));
        assert_eq!(calls, 0);

        let result = run_invite_get_preflight(invite_get_preflight(None, "com_safe"), || {
            calls += 1;
            "authenticated continuation"
        });
        assert!(matches!(
            result,
            ControlFlow::Continue("authenticated continuation")
        ));
        assert_eq!(calls, 1);
    }

    #[test]
    fn reveal_html_contains_code_once_and_only_as_text() {
        let code = "ACDEFG";
        let html = invite_reveal_html(&InviteCodeReveal::new(code.to_owned()));
        assert_eq!(html.matches(code).count(), 1);
        assert!(html.contains(&format!(">{code}</div>")));
        assert!(!html.contains(&format!("=\"{code}\"")));
        assert!(!html.contains("data-"));
        assert!(!html.contains("<script"));
        assert!(html.contains(i18n::JA_ADMIN_INVITES_REVEAL_WARNING));
    }
}
