//! Me / profile handler (RFC-005 §6 / external-design §8.6).

use worker::{D1Result, Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;

use crate::audit::{self, AuditAction, AuditMetadata};
use crate::authz::require_membership;
use crate::crypto::hmac_hex;
use crate::db::{self, membership as membership_db};
use crate::form_token::ConsumeResult;
use crate::render;
use zinnias_ciao_contracts::i18n;
use zinnias_ciao_domain::{DisplayNameError, validate_display_name};

const DISPLAY_NAME_UPDATED_REF: &str = "display_name_updated";
const DISPLAY_NAME_UNCHANGED_REF: &str = "display_name_unchanged";

pub async fn get_me(req: Request, env: &Env, _rid: &str, community_id: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_membership(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let url = req.url()?;
    let flash_code = url
        .query_pairs()
        .find(|(k, _)| k == "flash")
        .map(|(_, v)| v.to_string());
    let flash_html = me_flash_message(flash_code.as_deref())
        .map(|message| {
            format!(
                "<p role=\"status\" style=\"font-size:.875rem;color:#167A34;margin:.5rem 0 1rem\">{}</p>",
                render::escape_html(message)
            )
        })
        .unwrap_or_default();

    let logout_token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::LOGOUT, None).await?;

    let community = db::community::find_active(&db, community_id).await?;
    let community_name = community.as_ref().map(|c| c.name.as_str()).unwrap_or("");
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await
        .unwrap_or_default();
    let _community_pairs: Vec<(String, String)> = _communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();
    let role_label = if membership.is_admin() {
        i18n::JA_ROLE_ADMIN
    } else {
        i18n::JA_ROLE_MEMBER
    };
    let can_create_community = crate::handlers::community_create::community_creation_enabled(env)
        && membership_db::find_first_admin_for_user(&db, &auth.user_id)
            .await?
            .is_some();

    // RFC-035: support diagnostics
    let app_version = env
        .var("BUILD_VERSION")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "dev".to_owned());
    // Short community reference (first 8 chars of community_id) for support context.
    let support_ref = community_id.get(..8).unwrap_or(community_id);

    let admin_tools_html: String = if membership.is_admin() {
        format!(
            "<section style=\"margin-top:1.5rem\"><h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">{admin_section}</h2><a href=\"/c/{cid}/admin/members\" style=\"display:block;font-size:.9375rem;color:#007AFF;padding:.375rem 0;min-height:44px;line-height:44px\">{members_lbl}</a><a href=\"/c/{cid}/admin/export\" style=\"display:block;font-size:.9375rem;color:#007AFF;padding:.375rem 0;min-height:44px;line-height:44px\">{export_lbl}</a></section>",
            cid = render::escape_html(community_id),
            admin_section = i18n::JA_ME_SECTION_ADMIN,
            members_lbl = i18n::JA_ME_MANAGE_MEMBERS,
            export_lbl = i18n::JA_ME_DATA_EXPORT,
        )
    } else {
        String::new()
    };
    let community_create_html = if can_create_community {
        format!(
            "<a href=\"/communities/new\" style=\"display:block;font-size:.9375rem;color:#007AFF;padding:.375rem 0;min-height:44px;line-height:44px\">{}</a>",
            i18n::JA_COMMUNITY_CREATE_LINK,
        )
    } else {
        String::new()
    };

    let nav = render::bottom_nav(community_id, "me");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           <section style=\"margin-bottom:1.5rem\">\
             <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
             text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">{lbl_name}</h2>\
             {flash_html}\
             <p style=\"font-size:1rem;margin:0\">{name}</p>\
             <a href=\"/c/{cid}/me/display-name\" \
               style=\"display:inline-block;font-size:.9375rem;color:#007AFF;\
               margin-top:.5rem;min-height:44px;line-height:44px;text-decoration:none\">\
               {change_name}</a>\
           </section>\
           <section style=\"margin-bottom:1.5rem\">\
             <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
             text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">\
             {lbl_community}</h2>\
             <p style=\"font-size:1rem;margin:0\">{community} · {role}</p>\
             {community_create}\
           </section>\
           <section style=\"margin-bottom:1.5rem\">\
             <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
             text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">{lbl_help}</h2>\
             <p style=\"font-size:.875rem;color:#6e6e73;margin:0\">\
             {help_body}</p>\
           </section>\
           <section style=\"margin-top:1.5rem\">\
             <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
               text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">{cal_section}</h2>\
             <a href=\"/c/{cid}/me/calendar\" \
               style=\"display:block;font-size:.9375rem;color:#007AFF;padding:.375rem 0;\
               min-height:44px;line-height:44px\">{cal_feed_lbl}</a>\
           </section>\
           {admin_tools}\
           <section style=\"margin-top:1.5rem\">\
             <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
               text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">{lbl_about}</h2>\
             <p style=\"font-size:.8125rem;color:#6e6e73;margin:0\">{lbl_version} {version}</p>\
             <p style=\"font-size:.8125rem;color:#6e6e73;margin:.25rem 0 0\">{lbl_ref}: {ref_code}</p>\
           </section>\
           <form method=\"post\" action=\"/logout\" style=\"margin-top:2rem\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               style=\"width:100%;padding:.875rem;background:#fff;\
               color:#FF3B30;border:2px solid #FF3B30;border-radius:14px;\
               font-size:1rem;font-weight:600;min-height:44px;cursor:pointer\">\
               {lbl_logout}</button>\
           </form>\
         </main>{nav}",
        header = render::header_with_switcher_next(
            i18n::JA_NAV_ME,
            community_id,
            &_community_pairs,
            "me"
        ),
        name = render::escape_html(&membership.display_name),
        flash_html = flash_html,
        change_name = i18n::JA_ME_CHANGE_DISPLAY_NAME,
        community = render::escape_html(community_name),
        role = role_label,
        community_create = community_create_html,
        cid = render::escape_html(community_id),
        cal_section = i18n::JA_CALENDAR_TITLE,
        cal_feed_lbl = i18n::JA_ME_CALENDAR_LABEL,
        lbl_name = i18n::JA_ME_SECTION_NAME,
        lbl_community = i18n::JA_ME_SECTION_COMMUNITY,
        lbl_help = i18n::JA_ME_SECTION_HELP,
        help_body = i18n::JA_ME_HELP_BODY,
        lbl_logout = i18n::JA_LOGOUT,
        lbl_about = i18n::JA_ME_SECTION_ABOUT,
        lbl_version = i18n::JA_ME_VERSION_LABEL,
        lbl_ref = i18n::JA_ME_REF_LABEL,
        version = render::escape_html(&app_version),
        ref_code = render::escape_html(support_ref),
        admin_tools = admin_tools_html,
        tok = render::escape_html(&logout_token),
        nav = nav,
    );
    render::page(i18n::JA_NAV_ME, &body)
}

pub async fn get_display_name(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_membership(env, &auth, community_id).await?;
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::CHANGE_DISPLAY_NAME,
        Some(&membership.membership_id),
    )
    .await?;
    render_display_name_form(&membership, &token, &membership.display_name, None)
}

pub async fn post_display_name(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_membership(env, &auth, community_id).await?;

    let form = req.form_data().await?;
    let raw_token = form.get_field("_token").unwrap_or_default();
    let raw_display_name = form.get_field("display_name").unwrap_or_default();

    let display_name = match validate_display_name(&raw_display_name) {
        Ok(name) => name,
        Err(err) => {
            return refresh_display_name_form(
                env,
                &auth.user_id,
                &membership,
                &raw_display_name,
                Some(display_name_error(err)),
            )
            .await;
        }
    };

    let db = env.d1("DB")?;
    let pepper = crate::crypto::pepper(env)?;
    let consume = crate::form_token::consume_detailed(
        &db,
        pepper.as_str(),
        &auth.user_id,
        token_purpose::CHANGE_DISPLAY_NAME,
        &raw_token,
        Some(&membership.membership_id),
    )
    .await?;

    match consume {
        ConsumeResult::Replay(Some(result_ref)) if result_ref == DISPLAY_NAME_UPDATED_REF => {
            return redirect(&format!(
                "/c/{community_id}/me?flash={DISPLAY_NAME_UPDATED_REF}"
            ));
        }
        ConsumeResult::Replay(Some(result_ref)) if result_ref == DISPLAY_NAME_UNCHANGED_REF => {
            return redirect(&format!("/c/{community_id}/me"));
        }
        ConsumeResult::Replay(_) => {
            return redirect(&format!("/c/{community_id}/me"));
        }
        ConsumeResult::Proceed => {}
    }

    if display_name == membership.display_name {
        crate::form_token::set_result(&db, pepper.as_str(), &raw_token, DISPLAY_NAME_UNCHANGED_REF)
            .await?;
        return redirect(&format!("/c/{community_id}/me"));
    }

    update_display_name_with_audit_and_result(
        &db,
        rid,
        community_id,
        &auth.user_id,
        &membership.membership_id,
        &display_name,
        pepper.as_str(),
        &raw_token,
    )
    .await?;

    redirect(&format!(
        "/c/{community_id}/me?flash={DISPLAY_NAME_UPDATED_REF}"
    ))
}

async fn refresh_display_name_form(
    env: &Env,
    user_id: &str,
    membership: &crate::authz::MembershipContext,
    display_name: &str,
    error: Option<&str>,
) -> Result<Response> {
    let token = crate::codlet::issue_token(
        env,
        user_id,
        token_purpose::CHANGE_DISPLAY_NAME,
        Some(&membership.membership_id),
    )
    .await?;
    render_display_name_form(membership, &token, display_name, error)
}

fn render_display_name_form(
    membership: &crate::authz::MembershipContext,
    token: &str,
    display_name: &str,
    error: Option<&str>,
) -> Result<Response> {
    let body = display_name_form_body(membership, token, display_name, error);
    render::page(i18n::JA_ME_DISPLAY_NAME_EDIT_TITLE, &body)
}

fn display_name_form_body(
    membership: &crate::authz::MembershipContext,
    token: &str,
    display_name: &str,
    error: Option<&str>,
) -> String {
    let error_html = error
        .map(|message| {
            format!(
                "<p role=\"alert\" style=\"color:#FF3B30;margin:.75rem 0 0\">{}</p>",
                render::escape_html(message)
            )
        })
        .unwrap_or_default();
    let cid = render::escape_html(&membership.community_id);
    format!(
        "{header}<main style=\"padding:1rem 1rem 5rem;max-width:560px;margin:0 auto\">\
           {error_html}\
           <form method=\"post\" action=\"/c/{cid}/me/display-name\" style=\"margin-top:1rem\">\
             <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
             <label style=\"display:block;font-size:.875rem;font-weight:600;margin-bottom:.375rem\" for=\"display_name\">{label}</label>\
             <input id=\"display_name\" name=\"display_name\" value=\"{display}\" required maxlength=\"40\" autocomplete=\"name\" \
               style=\"width:100%;box-sizing:border-box;font-size:1rem;padding:.75rem;border:1px solid #D1D1D6;border-radius:8px;min-height:44px\">\
             <button type=\"submit\" style=\"width:100%;margin-top:1.25rem;padding:.875rem;background:#007AFF;color:#fff;border:none;border-radius:8px;font-size:1rem;font-weight:600;min-height:44px;cursor:pointer\">{submit}</button>\
           </form>\
           <a href=\"/c/{cid}/me\" style=\"display:inline-block;margin-top:.75rem;color:#007AFF;min-height:44px;line-height:44px;text-decoration:none\">{cancel}</a>\
         </main>{nav}",
        header = render::header(i18n::JA_ME_DISPLAY_NAME_EDIT_TITLE, ""),
        error_html = error_html,
        cid = cid,
        token = render::escape_html(token),
        label = i18n::JA_ME_SECTION_NAME,
        display = render::escape_html(display_name),
        submit = i18n::JA_ME_DISPLAY_NAME_EDIT_SUBMIT,
        cancel = i18n::JA_ME_DISPLAY_NAME_EDIT_CANCEL,
        nav = render::bottom_nav(&membership.community_id, "me"),
    )
}

#[allow(clippy::too_many_arguments)]
async fn update_display_name_with_audit_and_result(
    db: &worker::D1Database,
    request_id: &str,
    community_id: &str,
    user_id: &str,
    membership_id: &str,
    display_name: &str,
    pepper: &str,
    raw_token: &str,
) -> Result<()> {
    let token_hmac = hmac_hex(pepper, raw_token);
    let audit = audit::required_record(
        request_id,
        Some(community_id),
        Some(membership_id),
        Some(membership_id),
        AuditAction::MembershipDisplayNameUpdated,
        AuditMetadata::DisplayNameChanged,
    )?;

    let update_stmt = db
        .prepare(
            "UPDATE community_memberships \
             SET display_name = ?1 \
             WHERE id = ?2 \
               AND community_id = ?3 \
               AND user_id = ?4 \
               AND removed_at IS NULL \
               AND display_name != ?1",
        )
        .bind(&[
            display_name.into(),
            membership_id.into(),
            community_id.into(),
            user_id.into(),
        ])?;

    let audit_stmt = audit.statement_after_one_change(db)?;

    let result_stmt = db
        .prepare(
            "UPDATE form_tokens \
             SET result_ref = ?1 \
             WHERE token_hmac = ?2 \
               AND user_id = ?3 \
               AND purpose = ?4 \
               AND consumed_at IS NOT NULL \
               AND EXISTS ( \
                 SELECT 1 FROM community_memberships \
                 WHERE id = ?5 AND community_id = ?6 AND user_id = ?3 \
                   AND removed_at IS NULL AND display_name = ?7 \
               )",
        )
        .bind(&[
            DISPLAY_NAME_UPDATED_REF.into(),
            token_hmac.as_str().into(),
            user_id.into(),
            token_purpose::CHANGE_DISPLAY_NAME.into(),
            membership_id.into(),
            community_id.into(),
            display_name.into(),
        ])?;

    let results = db.batch(vec![update_stmt, audit_stmt, result_stmt]).await?;
    require_changed(&results, 0, "display name update")?;
    require_changed(&results, 1, "display name audit")?;
    require_changed(&results, 2, "display name replay result")?;

    audit.log_success();
    Ok(())
}

fn require_changed(results: &[D1Result], index: usize, label: &str) -> Result<()> {
    let changed = results
        .get(index)
        .and_then(|result| result.meta().ok().flatten())
        .and_then(|meta| meta.changes)
        .unwrap_or(0);
    if changed == 1 {
        Ok(())
    } else {
        Err(worker::Error::RustError(format!(
            "{label} affected {changed} rows"
        )))
    }
}

fn display_name_error(err: DisplayNameError) -> &'static str {
    match err {
        DisplayNameError::Empty | DisplayNameError::TooLong | DisplayNameError::InvalidChars => {
            i18n::JA_ME_DISPLAY_NAME_ERROR
        }
    }
}

fn me_flash_message(code: Option<&str>) -> Option<&'static str> {
    match code {
        Some(DISPLAY_NAME_UPDATED_REF) => Some(i18n::JA_ME_DISPLAY_NAME_UPDATED),
        _ => None,
    }
}

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}

#[cfg(test)]
mod tests;
