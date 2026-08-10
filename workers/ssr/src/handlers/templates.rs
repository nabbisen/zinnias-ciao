//! Event template handlers (RFC-032).
//!
//! Routes:
//!   GET  /c/:cid/admin/templates              — list templates + create form
//!   POST /c/:cid/admin/templates              — save new template
//!   POST /c/:cid/admin/templates/:tid/delete  — soft-delete template

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;

use crate::authz::require_admin;
use crate::crypto::random_token;
use crate::db::{event_template as tmpl_db, membership as membership_db};
use crate::render;
use zinnias_ciao_contracts::i18n;

fn redirect(location: &str) -> Result<Response> {
    let mut resp = Response::from_html("")?;
    resp.headers_mut().set("Location", location)?;
    Ok(resp.with_status(303))
}

/// Handoff 037: the `calendar_flash_message` pattern, admin-only Japanese
/// (RFC-072 Slice D — no locale to resolve). Unknown codes return `None`;
/// the caller must render no flash element in that case, not echo the code.
fn templates_flash_message(code: Option<&str>) -> Option<&'static str> {
    match code {
        Some("title_required") => Some(i18n::JA_ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH),
        Some("template_saved") => Some(i18n::JA_ADMIN_TEMPLATE_SAVED_FLASH),
        Some("template_deleted") => Some(i18n::JA_ADMIN_TEMPLATE_DELETED_FLASH),
        _ => None,
    }
}

// ── GET /c/:cid/admin/templates ───────────────────────────────────────────

pub async fn get_templates(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let _membership = require_admin(env, &auth, community_id, rid).await?;
    let db = env.d1("DB")?;

    let create_token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::CREATE_TEMPLATE,
        Some(community_id),
    )
    .await?;

    let templates = tmpl_db::list_active(&db, community_id)
        .await
        .unwrap_or_default();
    let communities = membership_db::list_communities_for_user(
        &db,
        &auth.user_id,
        auth.scope_community_id.as_deref(),
    )
    .await
    .unwrap_or_default();
    let community_pairs: Vec<(String, String)> = communities
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();

    let url = req.url()?;
    let flash_code: Option<String> = url
        .query_pairs()
        .find(|(k, _)| k == "flash")
        .map(|(_, v)| v.to_string());
    let flash_html = templates_flash_message(flash_code.as_deref())
        .map(|message| {
            format!(
                "<p role=\"status\" class=\"cz-admin-invite-flash\">{}</p>",
                render::escape_html(message)
            )
        })
        .unwrap_or_default();

    // Build template list rows
    let mut list_html = String::new();
    for t in &templates {
        let delete_tok = crate::codlet::issue_token(
            env,
            &auth.user_id,
            token_purpose::DELETE_TEMPLATE,
            Some(&t.id),
        )
        .await?;

        let dur_label = t
            .duration_minutes
            .map(|d| format!(" · {}min", d))
            .unwrap_or_default();
        let loc_label = t
            .location
            .as_deref()
            .map(|l| format!(" · {}", l))
            .unwrap_or_default();

        list_html.push_str(&format!(
            "<li class=\"cz-templates-row\">\
             <div>\
               <span class=\"cz-templates-item-title\">{title}</span>\
               <span class=\"cz-templates-meta\">{loc}{dur}</span>\
             </div>\
             <div class=\"cz-templates-row-actions\">\
               <a href=\"/c/{cid}/admin/events/new?template={tid}\" \
                  class=\"cz-templates-use-link\">\
                  {use_btn}</a>\
               <form method=\"post\" \
                 action=\"/c/{cid}/admin/templates/{tid}/delete\" class=\"cz-templates-delete-form\">\
                 <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
                 <button type=\"submit\" \
                   class=\"cz-templates-delete-button\"\
                   aria-label=\"{del_btn}\">\
                   {del_btn}</button>\
               </form>\
             </div>\
             </li>",
            title = render::escape_html(&t.title),
            loc = render::escape_html(&loc_label),
            dur = render::escape_html(&dur_label),
            cid = render::escape_html(community_id),
            tid = render::escape_html(&t.id),
            tok = render::escape_html(&delete_tok),
            use_btn = i18n::JA_TEMPLATES_USE_BTN,
            del_btn = i18n::JA_TEMPLATES_DELETE_BTN,
        ));
    }

    let empty_msg = if templates.is_empty() {
        &format!(
            "<p class=\"cz-admin-invites-body\">{}</p>",
            i18n::JA_TEMPLATES_EMPTY
        )
    } else {
        ""
    };

    let nav = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
         <h1 class=\"cz-admin-title cz-admin-title--tight\">{title_h1}</h1>\
         <p class=\"cz-templates-description\">\
           {desc}\
         </p>\
         {flash}\
         {empty}\
         {list}\
         <section class=\"cz-templates-save-section\">\
           <h2 class=\"cz-templates-save-heading\">{save_section_h2}</h2>\
           <form method=\"post\" action=\"/c/{cid}/admin/templates\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <label class=\"cz-templates-field\">\
               <span class=\"cz-admin-field-label\">{lbl_title}</span>\
               <input type=\"text\" name=\"title\" required maxlength=\"80\"\
                 class=\"cz-field-input\">\
             </label>\
             <label class=\"cz-templates-field\">\
               <span class=\"cz-admin-field-label\">{lbl_loc}</span>\
               <input type=\"text\" name=\"location\" maxlength=\"120\"\
                 class=\"cz-field-input\">\
             </label>\
             <label class=\"cz-templates-field\">\
               <span class=\"cz-admin-field-label\">{lbl_dur}</span>\
               <input type=\"number\" name=\"duration_minutes\" min=\"1\" max=\"1440\"\
                 class=\"cz-field-input\">\
             </label>\
             <button type=\"submit\"\
               class=\"cz-templates-save-button\">\
               {btn_save}</button>\
           </form>\
         </section>\
         </main>{nav}",
        title_h1 = i18n::JA_TEMPLATES_TITLE,
        desc = i18n::JA_TEMPLATES_DESCRIPTION,
        save_section_h2 = i18n::JA_TEMPLATES_SAVE_SECTION,
        lbl_title = i18n::JA_TEMPLATES_TITLE_LABEL,
        lbl_loc = i18n::JA_TEMPLATES_LOC_LABEL,
        lbl_dur = i18n::JA_TEMPLATES_DUR_LABEL,
        btn_save = i18n::JA_TEMPLATES_SAVE_BTN,
        header = render::header_with_switcher_next(
            i18n::JA_TEMPLATES_TITLE,
            community_id,
            &community_pairs,
            "admin_templates"
        ),
        flash = flash_html,
        empty = empty_msg,
        list = if list_html.is_empty() {
            String::new()
        } else {
            format!("<ul class=\"cz-templates-list\">{list_html}</ul>")
        },
        cid = render::escape_html(community_id),
        tok = render::escape_html(&create_token),
        nav = nav,
    );
    render::page(i18n::JA_TEMPLATES_TITLE, &body)
}

// ── POST /c/:cid/admin/templates ──────────────────────────────────────────

pub async fn post_create_template(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
    let db = env.d1("DB")?;

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::CREATE_TEMPLATE,
        &raw_token,
        Some(community_id),
    )
    .await?;
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/admin/templates"));
    }

    let title = body.get_field("title").unwrap_or_default();
    let title = title.trim();
    if title.is_empty() || title.len() > 80 {
        return redirect(&format!(
            "/c/{community_id}/admin/templates?flash=title_required"
        ));
    }

    let location = body.get_field("location").unwrap_or_default();
    let location = location.trim();
    let location = if location.is_empty() {
        None
    } else {
        Some(location)
    };

    let duration_minutes: Option<u32> = body
        .get_field("duration_minutes")
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&d| d > 0 && d <= 1440);

    let template_id = random_token()[..24].to_owned();
    let created = tmpl_db::insert_required(
        &db,
        rid,
        &template_id,
        community_id,
        &membership.membership_id,
        title,
        location,
        None,
        duration_minutes,
    )
    .await?;
    if !created {
        return render::not_found();
    }

    redirect(&format!(
        "/c/{community_id}/admin/templates?flash=template_saved"
    ))
}

// ── POST /c/:cid/admin/templates/:tid/delete ─────────────────────────────

pub async fn post_delete_template(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    template_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
    let db = env.d1("DB")?;

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::DELETE_TEMPLATE,
        &raw_token,
        Some(template_id),
    )
    .await?;
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/admin/templates"));
    }

    tmpl_db::soft_delete_required(
        &db,
        rid,
        template_id,
        community_id,
        &membership.membership_id,
    )
    .await?;

    redirect(&format!(
        "/c/{community_id}/admin/templates?flash=template_deleted"
    ))
}

#[cfg(test)]
mod tests;
