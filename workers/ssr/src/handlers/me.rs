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

pub async fn get_me(req: Request, env: &Env, rid: &str, community_id: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_membership(env, &auth, community_id, rid).await?;
    let locale = membership.locale;
    let db = env.d1("DB")?;
    let url = req.url()?;
    let flash_code = url
        .query_pairs()
        .find(|(k, _)| k == "flash")
        .map(|(_, v)| v.to_string());
    let flash_html = me_flash_message(locale, flash_code.as_deref())
        .map(|message| {
            format!(
                "<p role=\"status\" class=\"cz-me-flash\">{}</p>",
                render::escape_html(message)
            )
        })
        .unwrap_or_default();

    let logout_token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::LOGOUT, None).await?;

    let community = db::community::find_active(&db, community_id).await?;
    let community_name = community.as_ref().map(|c| c.name.as_str()).unwrap_or("");
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
    let role_label = i18n::t(
        locale,
        if membership.is_admin() {
            i18n::ROLE_ADMIN
        } else {
            i18n::ROLE_MEMBER
        },
    );
    // RFC-081 §2.1a / Handoff 048 §7.4: a community-bound session must not
    // even be shown this link — `require_active_admin_somewhere` (the
    // route this points to) refuses it unconditionally, and rendering a
    // link that always 404s would itself leak "this account is an admin
    // somewhere else" through a session scoped to just this community.
    let can_create_community = auth.scope_community_id.is_none()
        && crate::handlers::community_create::community_creation_enabled(env)
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
            "<section class=\"cz-me-section--mt\"><h2 class=\"cz-me-section-heading\">{admin_section}</h2><a href=\"/c/{cid}/admin/members\" class=\"cz-me-link cz-me-link--padded\">{members_lbl}</a><a href=\"/c/{cid}/admin/export\" class=\"cz-me-link cz-me-link--padded\">{export_lbl}</a></section>",
            cid = render::escape_html(community_id),
            admin_section = i18n::t(locale, i18n::ME_SECTION_ADMIN),
            members_lbl = i18n::t(locale, i18n::ME_MANAGE_MEMBERS),
            export_lbl = i18n::t(locale, i18n::ME_DATA_EXPORT),
        )
    } else {
        String::new()
    };
    let community_create_html = if can_create_community {
        format!(
            "<a href=\"/communities/new\" class=\"cz-me-link cz-me-link--padded\">{}</a>",
            i18n::t(locale, i18n::COMMUNITY_CREATE_LINK),
        )
    } else {
        String::new()
    };

    let nav = render::bottom_nav_localized(community_id, "me", locale);
    let title = i18n::t(locale, i18n::NAV_ME);
    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
           <section class=\"cz-me-section--mb\">\
             <h2 class=\"cz-me-section-heading\">{lbl_name}</h2>\
             {flash_html}\
             <p class=\"cz-me-value-line\">{name}</p>\
             <a href=\"/c/{cid}/me/display-name\" \
               class=\"cz-me-link cz-me-link--inline\">\
               {change_name}</a>\
             <a href=\"/c/{cid}/me/language\" \
               class=\"cz-me-link cz-me-link--block\">\
               {change_language}</a>\
           </section>\
           <section class=\"cz-me-section--mb\">\
             <h2 class=\"cz-me-section-heading\">\
             {lbl_community}</h2>\
             <p class=\"cz-me-value-line\">{community} · {role}</p>\
             {community_create}\
           </section>\
           <section class=\"cz-me-section--mb\">\
             <h2 class=\"cz-me-section-heading\">{lbl_help}</h2>\
             <p class=\"cz-hint\">\
             {help_body}</p>\
           </section>\
           <section class=\"cz-me-section--mt\">\
             <h2 class=\"cz-me-section-heading\">{cal_section}</h2>\
             <a href=\"/c/{cid}/me/calendar\" \
               class=\"cz-me-link cz-me-link--padded\">{cal_feed_lbl}</a>\
           </section>\
           {admin_tools}\
           <section class=\"cz-me-section--mt\">\
             <h2 class=\"cz-me-section-heading\">{lbl_about}</h2>\
             <p class=\"cz-me-about-line\">{lbl_version} {version}</p>\
             <p class=\"cz-me-about-line cz-me-about-line--gap-top\">{lbl_ref}: {ref_code}</p>\
           </section>\
           <form method=\"post\" action=\"/logout\" class=\"cz-me-logout-form\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               class=\"cz-me-logout-button\">\
               {lbl_logout}</button>\
           </form>\
         </main>{nav}",
        header = render::header_with_switcher_next_localized(
            title,
            community_id,
            &_community_pairs,
            "me",
            locale
        ),
        name = render::escape_html(&membership.display_name),
        flash_html = flash_html,
        change_name = i18n::t(locale, i18n::ME_CHANGE_DISPLAY_NAME),
        change_language = i18n::t(locale, i18n::ME_LANGUAGE_TITLE),
        community = render::escape_html(community_name),
        role = role_label,
        community_create = community_create_html,
        cid = render::escape_html(community_id),
        cal_section = i18n::t(locale, i18n::CALENDAR_TITLE),
        cal_feed_lbl = i18n::t(locale, i18n::ME_CALENDAR_LABEL),
        lbl_name = i18n::t(locale, i18n::ME_SECTION_NAME),
        lbl_community = i18n::t(locale, i18n::ME_SECTION_COMMUNITY),
        lbl_help = i18n::t(locale, i18n::ME_SECTION_HELP),
        help_body = i18n::t(locale, i18n::ME_HELP_BODY),
        lbl_logout = i18n::t(locale, i18n::LOGOUT),
        lbl_about = i18n::t(locale, i18n::ME_SECTION_ABOUT),
        lbl_version = i18n::t(locale, i18n::ME_VERSION_LABEL),
        lbl_ref = i18n::t(locale, i18n::ME_REF_LABEL),
        version = render::escape_html(&app_version),
        ref_code = render::escape_html(support_ref),
        admin_tools = admin_tools_html,
        tok = render::escape_html(&logout_token),
        nav = nav,
    );
    render::page_localized(locale, title, &body)
}

pub async fn get_display_name(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_membership(env, &auth, community_id, rid).await?;
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
    let membership = require_membership(env, &auth, community_id, rid).await?;

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
                Some(display_name_error(membership.locale, err)),
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
    let title = i18n::t(membership.locale, i18n::ME_DISPLAY_NAME_EDIT_TITLE);
    render::page_localized(membership.locale, title, &body)
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
                "<p role=\"alert\" class=\"cz-me-form-error\">{}</p>",
                render::escape_html(message)
            )
        })
        .unwrap_or_default();
    let cid = render::escape_html(&membership.community_id);
    let locale = membership.locale;
    format!(
        "{header}<main class=\"cz-page-main cz-me-form-main\">\
           {error_html}\
           <form method=\"post\" action=\"/c/{cid}/me/display-name\" class=\"cz-me-form\">\
             <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
             <label class=\"cz-me-form-label\" for=\"display_name\">{label}</label>\
             <input id=\"display_name\" name=\"display_name\" value=\"{display}\" required maxlength=\"40\" autocomplete=\"name\" \
               class=\"cz-me-form-input\">\
             <button type=\"submit\" class=\"cz-me-form-submit\">{submit}</button>\
           </form>\
           <a href=\"/c/{cid}/me\" class=\"cz-me-form-cancel-link\">{cancel}</a>\
         </main>{nav}",
        header = render::header(i18n::t(locale, i18n::ME_DISPLAY_NAME_EDIT_TITLE), ""),
        error_html = error_html,
        cid = cid,
        token = render::escape_html(token),
        label = i18n::t(locale, i18n::ME_SECTION_NAME),
        display = render::escape_html(display_name),
        submit = i18n::t(locale, i18n::ME_DISPLAY_NAME_EDIT_SUBMIT),
        cancel = i18n::t(locale, i18n::ME_DISPLAY_NAME_EDIT_CANCEL),
        nav = render::bottom_nav_localized(&membership.community_id, "me", locale),
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

// ── Language settings (RFC-072 Slice B) ───────────────────────────────────
// Not linked from anywhere yet (Slice C links it from My Page). Copy is
// `me.rs`, `post_display_name`'s shape exactly: same membership binding,
// same no-JS POST-and-303 shape, same replay handling via `ConsumeResult`.
//
// Ordering note: the RFC's own POST Contract lists "accept only ja/en,
// reject without writing" (items 4-5) *before* "consume a form token"
// (item 6) — matching `post_display_name`'s actual shape (validate, then
// consume). Handoff 022 §7.3's numbered list has this reversed (consume
// before validate). Followed the RFC and the reference implementation,
// not the handoff's list order, per "the RFC wins."

const UI_LANGUAGE_UPDATED_REF: &str = "ui_language_updated";
const UI_LANGUAGE_UNCHANGED_REF: &str = "ui_language_unchanged";

pub async fn get_language(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_membership(env, &auth, community_id, rid).await?;
    let url = req.url()?;
    let flash_code = url
        .query_pairs()
        .find(|(k, _)| k == "flash")
        .map(|(_, v)| v.to_string());
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::CHANGE_UI_LANGUAGE,
        Some(&membership.membership_id),
    )
    .await?;
    let body = language_form_body(&membership, &token, None, flash_code.as_deref());
    let title = i18n::t(membership.locale, i18n::ME_LANGUAGE_TITLE);
    render::page_localized(membership.locale, title, &body)
}

pub async fn post_language(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_membership(env, &auth, community_id, rid).await?;

    let form = req.form_data().await?;
    let raw_token = form.get_field("_token").unwrap_or_default();
    let raw_ui_language = form.get_field("ui_language").unwrap_or_default();

    // Attacker-controlled/out-of-allow-list value: reject without writing,
    // and without consuming the token — mirrors `post_display_name`, which
    // lets a legitimate mistaken resubmission retry with the same token.
    // This can only happen via a tampered request (the rendered form only
    // ever offers "ja"/"en"), so the app's existing generic error copy
    // applies; no new copy was needed for this path.
    let Some(submitted) = zinnias_ciao_contracts::Locale::parse(&raw_ui_language) else {
        return refresh_language_form(
            env,
            &auth.user_id,
            &membership,
            Some(i18n::t(membership.locale, i18n::GENERAL_ERROR)),
        )
        .await;
    };

    let db = env.d1("DB")?;
    let pepper = crate::crypto::pepper(env)?;
    let consume = crate::form_token::consume_detailed(
        &db,
        pepper.as_str(),
        &auth.user_id,
        token_purpose::CHANGE_UI_LANGUAGE,
        &raw_token,
        Some(&membership.membership_id),
    )
    .await?;

    match consume {
        ConsumeResult::Replay(Some(result_ref)) if result_ref == UI_LANGUAGE_UPDATED_REF => {
            return redirect(&format!(
                "/c/{community_id}/me/language?flash={UI_LANGUAGE_UPDATED_REF}"
            ));
        }
        ConsumeResult::Replay(Some(result_ref)) if result_ref == UI_LANGUAGE_UNCHANGED_REF => {
            return redirect(&format!("/c/{community_id}/me/language"));
        }
        ConsumeResult::Replay(_) => {
            return redirect(&format!("/c/{community_id}/me/language"));
        }
        ConsumeResult::Proceed => {}
    }

    if submitted == membership.locale {
        crate::form_token::set_result(&db, pepper.as_str(), &raw_token, UI_LANGUAGE_UNCHANGED_REF)
            .await?;
        return redirect(&format!("/c/{community_id}/me/language"));
    }

    update_ui_language_with_result(
        &db,
        community_id,
        &auth.user_id,
        &membership.membership_id,
        submitted,
        pepper.as_str(),
        &raw_token,
    )
    .await?;

    redirect(&format!(
        "/c/{community_id}/me/language?flash={UI_LANGUAGE_UPDATED_REF}"
    ))
}

async fn refresh_language_form(
    env: &Env,
    user_id: &str,
    membership: &crate::authz::MembershipContext,
    error: Option<&str>,
) -> Result<Response> {
    let token = crate::codlet::issue_token(
        env,
        user_id,
        token_purpose::CHANGE_UI_LANGUAGE,
        Some(&membership.membership_id),
    )
    .await?;
    render_language_form(membership, &token, error)
}

fn render_language_form(
    membership: &crate::authz::MembershipContext,
    token: &str,
    error: Option<&str>,
) -> Result<Response> {
    let body = language_form_body(membership, token, error, None);
    let title = i18n::t(membership.locale, i18n::ME_LANGUAGE_TITLE);
    render::page_localized(membership.locale, title, &body)
}

fn language_form_body(
    membership: &crate::authz::MembershipContext,
    token: &str,
    error: Option<&str>,
    flash_code: Option<&str>,
) -> String {
    let locale = membership.locale;
    let error_html = error
        .map(|message| {
            format!(
                "<p role=\"alert\" class=\"cz-me-form-error\">{}</p>",
                render::escape_html(message)
            )
        })
        .unwrap_or_default();
    let flash_html = language_flash_message(locale, flash_code)
        .map(|message| {
            format!(
                "<p role=\"status\" class=\"cz-me-flash\">{}</p>",
                render::escape_html(message)
            )
        })
        .unwrap_or_default();
    let cid = render::escape_html(&membership.community_id);
    let ja_checked = if locale == zinnias_ciao_contracts::Locale::Ja {
        " checked"
    } else {
        ""
    };
    let en_checked = if locale == zinnias_ciao_contracts::Locale::En {
        " checked"
    } else {
        ""
    };
    format!(
        "{header}<main class=\"cz-page-main cz-me-form-main\">\
           {error_html}{flash_html}\
           <form method=\"post\" action=\"/c/{cid}/me/language\" class=\"cz-me-form\">\
             <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
             <fieldset class=\"cz-me-fieldset\">\
               <legend class=\"cz-me-legend\">{title}</legend>\
               <label class=\"cz-me-radio-label\">\
                 <input type=\"radio\" name=\"ui_language\" value=\"ja\"{ja_checked}> {ja_label}\
               </label>\
               <label class=\"cz-me-radio-label\">\
                 <input type=\"radio\" name=\"ui_language\" value=\"en\"{en_checked}> {en_label}\
               </label>\
             </fieldset>\
             <button type=\"submit\" class=\"cz-me-form-submit\">{submit}</button>\
           </form>\
           <a href=\"/c/{cid}/me\" class=\"cz-me-form-cancel-link\">{cancel}</a>\
         </main>{nav}",
        header = render::header(i18n::t(locale, i18n::ME_LANGUAGE_TITLE), ""),
        error_html = error_html,
        flash_html = flash_html,
        cid = cid,
        token = render::escape_html(token),
        title = i18n::t(locale, i18n::ME_LANGUAGE_TITLE),
        ja_checked = ja_checked,
        ja_label = i18n::LANGUAGE_OPTION_JA,
        en_checked = en_checked,
        en_label = i18n::LANGUAGE_OPTION_EN,
        submit = i18n::t(locale, i18n::ME_LANGUAGE_SUBMIT),
        cancel = i18n::t(locale, i18n::ME_LANGUAGE_CANCEL),
        nav = render::bottom_nav_localized(&membership.community_id, "me", locale),
    )
}

#[allow(clippy::too_many_arguments)]
async fn update_ui_language_with_result(
    db: &worker::D1Database,
    community_id: &str,
    user_id: &str,
    membership_id: &str,
    locale: zinnias_ciao_contracts::Locale,
    pepper: &str,
    raw_token: &str,
) -> Result<()> {
    let token_hmac = hmac_hex(pepper, raw_token);

    let update_stmt = db
        .prepare(
            "UPDATE community_memberships \
             SET ui_language = ?1 \
             WHERE id = ?2 \
               AND community_id = ?3 \
               AND user_id = ?4 \
               AND removed_at IS NULL",
        )
        .bind(&[
            locale.code().into(),
            membership_id.into(),
            community_id.into(),
            user_id.into(),
        ])?;

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
                   AND removed_at IS NULL AND ui_language = ?7 \
               )",
        )
        .bind(&[
            UI_LANGUAGE_UPDATED_REF.into(),
            token_hmac.as_str().into(),
            user_id.into(),
            token_purpose::CHANGE_UI_LANGUAGE.into(),
            membership_id.into(),
            community_id.into(),
            locale.code().into(),
        ])?;

    let results = db.batch(vec![update_stmt, result_stmt]).await?;
    require_changed(&results, 0, "ui_language update")?;
    require_changed(&results, 1, "ui_language replay result")?;

    Ok(())
}

fn language_flash_message(
    locale: zinnias_ciao_contracts::Locale,
    code: Option<&str>,
) -> Option<&'static str> {
    match code {
        Some(UI_LANGUAGE_UPDATED_REF) => Some(i18n::t(locale, i18n::ME_LANGUAGE_UPDATED)),
        _ => None,
    }
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

fn display_name_error(
    locale: zinnias_ciao_contracts::Locale,
    err: DisplayNameError,
) -> &'static str {
    match err {
        DisplayNameError::Empty | DisplayNameError::TooLong | DisplayNameError::InvalidChars => {
            i18n::t(locale, i18n::ME_DISPLAY_NAME_ERROR)
        }
    }
}

fn me_flash_message(
    locale: zinnias_ciao_contracts::Locale,
    code: Option<&str>,
) -> Option<&'static str> {
    match code {
        Some(DISPLAY_NAME_UPDATED_REF) => Some(i18n::t(locale, i18n::ME_DISPLAY_NAME_UPDATED)),
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
