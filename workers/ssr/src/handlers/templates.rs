//! Event template handlers (RFC-032).
//!
//! Routes:
//!   GET  /c/:cid/admin/templates              — list templates + create form
//!   POST /c/:cid/admin/templates              — save new template
//!   POST /c/:cid/admin/templates/:tid/delete  — soft-delete template

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;

use crate::audit;
use crate::authz::require_admin;
use crate::crypto::random_token;
use crate::db::{event_template as tmpl_db, membership as membership_db};
use zinnias_ciao_contracts::i18n;
use crate::form_token;
use crate::render;
use crate::session::require_auth;

fn pepper(env: &Env) -> String {
    env.var("HMAC_PEPPER")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "dev-pepper".to_owned())
}

fn redirect(location: &str) -> Result<Response> {
    let mut resp = Response::from_html("")?;
    resp.headers_mut().set("Location", location)?;
    Ok(resp.with_status(303))
}

// ── GET /c/:cid/admin/templates ───────────────────────────────────────────

pub async fn get_templates(
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
    let pp = pepper(env);

    let create_token = form_token::issue(
        &db, &pp, &auth.user_id,
        token_purpose::CREATE_TEMPLATE, Some(community_id),
    ).await.unwrap_or_default();

    let templates = tmpl_db::list_active(&db, community_id).await.unwrap_or_default();
    let communities = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await.unwrap_or_default();
    let community_pairs: Vec<(String, String)> = communities.iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone())).collect();

    let url = req.url()?;
    let flash: Option<String> = url.query_pairs()
        .find(|(k, _)| k == "flash").map(|(_, v)| v.to_string());
    let flash_html = flash.map(|f| format!(
        "<p role=\"status\" style=\"font-size:.875rem;color:#167A34;margin:.5rem 0\">{}</p>",
        render::escape_html(&f)
    )).unwrap_or_default();

    // Build template list rows
    let mut list_html = String::new();
    for t in &templates {
        let delete_tok = form_token::issue(
            &db, &pp, &auth.user_id,
            token_purpose::DELETE_TEMPLATE, Some(&t.id),
        ).await.unwrap_or_default();

        let dur_label = t.duration_minutes
            .map(|d| format!(" · {}min", d))
            .unwrap_or_default();
        let loc_label = t.location.as_deref()
            .map(|l| format!(" · {}", l))
            .unwrap_or_default();

        list_html.push_str(&format!(
            "<li style=\"display:flex;align-items:center;justify-content:space-between;\
             padding:.75rem 0;border-bottom:1px solid #f5f5f7;gap:.5rem\">\
             <div>\
               <span style=\"font-weight:600;font-size:.9375rem\">{title}</span>\
               <span style=\"font-size:.8125rem;color:#6e6e73\">{loc}{dur}</span>\
             </div>\
             <div style=\"display:flex;gap:.5rem;align-items:center\">\
               <a href=\"/c/{cid}/admin/events/new?template={tid}\" \
                  style=\"font-size:.875rem;color:#007AFF;text-decoration:none;\
                  padding:.375rem .625rem;min-height:44px;display:flex;align-items:center\">\
                  Use</a>\
               <form method=\"post\" \
                 action=\"/c/{cid}/admin/templates/{tid}/delete\" style=\"margin:0\">\
                 <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
                 <button type=\"submit\" \
                   style=\"font-size:.8125rem;color:#FF3B30;background:none;border:none;\
                   cursor:pointer;padding:.375rem .5rem;min-height:44px\"\
                   aria-label=\"Delete template\">\
                   Delete</button>\
               </form>\
             </div>\
             </li>",
            title = render::escape_html(&t.title),
            loc   = render::escape_html(&loc_label),
            dur   = render::escape_html(&dur_label),
            cid   = render::escape_html(community_id),
            tid   = render::escape_html(&t.id),
            tok   = render::escape_html(&delete_tok),
        ));
    }

    let empty_msg = if templates.is_empty() {
        "<p style=\"font-size:.875rem;color:#6e6e73\">No templates yet.</p>"
    } else { "" };

    let nav  = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.25rem\">Event Templates</h1>\
         <p style=\"font-size:.875rem;color:#6e6e73;margin-bottom:1rem\">\
           Save common event details as templates to create events faster.\
         </p>\
         {flash}\
         {empty}\
         {list}\
         <section style=\"margin-top:2rem\">\
           <h2 style=\"font-size:1rem;font-weight:600;margin-bottom:.75rem\">Save a template</h2>\
           <form method=\"post\" action=\"/c/{cid}/admin/templates\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <label style=\"display:block;margin-bottom:.75rem\">\
               <span style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">Title</span>\
               <input type=\"text\" name=\"title\" required maxlength=\"80\"\
                 style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
                 border-radius:12px;font-size:1rem\">\
             </label>\
             <label style=\"display:block;margin-bottom:.75rem\">\
               <span style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">\
               Location (optional)</span>\
               <input type=\"text\" name=\"location\" maxlength=\"120\"\
                 style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
                 border-radius:12px;font-size:1rem\">\
             </label>\
             <label style=\"display:block;margin-bottom:.75rem\">\
               <span style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">\
               Default duration in minutes (optional)</span>\
               <input type=\"number\" name=\"duration_minutes\" min=\"1\" max=\"1440\"\
                 style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
                 border-radius:12px;font-size:1rem\">\
             </label>\
             <button type=\"submit\"\
               style=\"width:100%;padding:.875rem;background:#007AFF;color:#fff;\
               border:none;border-radius:14px;font-size:1rem;font-weight:600;\
               min-height:44px;cursor:pointer\">\
               Save template</button>\
           </form>\
         </section>\
         </main>{nav}",
        header = render::header_with_switcher(i18n::EN_TEMPLATES_TITLE, community_id, &community_pairs),
        flash  = flash_html,
        empty  = empty_msg,
        list   = if list_html.is_empty() { String::new() }
                 else { format!("<ul style=\"list-style:none;padding:0;margin:0\">{list_html}</ul>") },
        cid    = render::escape_html(community_id),
        tok    = render::escape_html(&create_token),
        nav    = nav,
    );
    render::page(i18n::EN_TEMPLATES_TITLE, &body)
}

// ── POST /c/:cid/admin/templates ──────────────────────────────────────────

pub async fn post_create_template(
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
    let pp = pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(
        &db, &pp, &auth.user_id,
        token_purpose::CREATE_TEMPLATE, &raw_token, Some(community_id),
    ).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/admin/templates"));
    }

    let title = body.get_field("title").unwrap_or_default();
    let title = title.trim();
    if title.is_empty() || title.len() > 80 {
        return redirect(&format!("/c/{community_id}/admin/templates?flash=Title+required"));
    }

    let location = body.get_field("location").unwrap_or_default();
    let location = location.trim();
    let location = if location.is_empty() { None } else { Some(location) };

    let duration_minutes: Option<u32> = body.get_field("duration_minutes")
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&d| d > 0 && d <= 1440);

    let template_id = random_token()[..24].to_owned();
    tmpl_db::insert(
        &db, &template_id, community_id, &membership.membership_id,
        title, location, None, duration_minutes,
    ).await?;

    let _ = audit::write(
        &db, rid, Some(community_id), Some(&membership.membership_id),
        "event_template", Some(&template_id), "created", None,
    ).await;

    redirect(&format!("/c/{community_id}/admin/templates?flash=Template+saved"))
}

// ── POST /c/:cid/admin/templates/:tid/delete ─────────────────────────────

pub async fn post_delete_template(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    template_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(
        &db, &pp, &auth.user_id,
        token_purpose::DELETE_TEMPLATE, &raw_token, Some(template_id),
    ).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/admin/templates"));
    }

    tmpl_db::soft_delete(&db, template_id, community_id).await?;

    let _ = audit::write(
        &db, rid, Some(community_id), Some(&membership.membership_id),
        "event_template", Some(template_id), "deleted", None,
    ).await;

    redirect(&format!("/c/{community_id}/admin/templates?flash=Template+deleted"))
}
