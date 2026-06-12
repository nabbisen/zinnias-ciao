//! Admin handlers — event management, invite codes, member management (RFC-009/010).

use zinnias_ciao_contracts::auth::token_purpose;
use worker::{Env, Request, Response, Result};

use crate::audit;
use crate::authz::require_admin;
use crate::crypto::{hmac_hex, normalize_invite_code, random_token};
use crate::db::{self, event as event_db, event_write, invite as invite_db, membership as membership_db};
use crate::form_token;
use crate::render;
use crate::session::require_auth;
use crate::handlers::event::classify_day;
use zinnias_ciao_domain::{validate_event, DayInput, EventInput, RecurrenceFreq, expand_recurrence};
use zinnias_ciao_contracts::i18n;
use zinnias_ciao_domain::status::DayTimeState;


fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

// ── GET /c/:cid/admin/events/new ─────────────────────────────────────────

pub async fn get_create_event(
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
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::CREATE_EVENT, None).await.unwrap_or_default();

    let _community = db::community::find_active(&db, community_id).await?;
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    // RFC-032: pre-fill from template if ?template=TID is present.
    let url = req.url()?;
    let template_id = url.query_pairs().find(|(k,_)| k == "template").map(|(_,v)| v.to_string());
    let err_msg: Option<String> = url.query_pairs().find(|(k,_)| k == "err").map(|(_,v)| v.to_string());
    let (prefill_title, prefill_location) = if let Some(ref tid) = template_id {
        let tmpl = db::event_template::find_active(&db, tid, community_id).await.ok().flatten();
        (
            tmpl.as_ref().map(|t| t.title.clone()),
            tmpl.as_ref().and_then(|t| t.location.clone()),
        )
    } else {
        (None, None)
    };

    let templates_link = format!(
        "<a href=\"/c/{cid}/admin/templates\" \
           style=\"display:block;text-align:center;color:#007AFF;\
           font-size:.875rem;margin-top:1rem;min-height:44px;line-height:44px\">\
           Use a template</a>",
        cid = render::escape_html(community_id),
    );

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">Create Event</h1>\
         <form method=\"post\" action=\"/c/{cid}/admin/events\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {fields}\
           <button type=\"submit\" style=\"width:100%;padding:.875rem;background:#007AFF;\
           color:#fff;border:none;border-radius:14px;font-size:1rem;font-weight:600;\
           min-height:44px;cursor:pointer;margin-top:1rem\">{submit}</button>\
         </form>\
         {tmpl_link}\
         </main>{nav}",
        header    = render::header_with_switcher(i18n::EN_ADMIN_CREATE_EVENT_TITLE, community_id, &_community_pairs),
        cid       = render::escape_html(community_id),
        tok       = render::escape_html(&token),
        fields    = event_form_fields(prefill_title.as_deref(), prefill_location.as_deref(), None, err_msg.as_deref(), None, None, None, true),
        submit    = i18n::EN_ADMIN_CREATE_EVENT_SUBMIT,
        tmpl_link = templates_link,
        nav       = nav,
    );
    render::page(i18n::EN_ADMIN_CREATE_EVENT_TITLE, &body)
}

// ── POST /c/:cid/admin/events ────────────────────────────────────────────

pub async fn post_create_event(
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
        token_purpose::CREATE_EVENT, &raw_token, None).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/home"));
    }

    let input = EventInput {
        title:       body.get_field("title").unwrap_or_default(),
        location:    Some(body.get_field("location").unwrap_or_default()),
        description: Some(body.get_field("description").unwrap_or_default()),
        days:        vec![DayInput {
            day_date:  body.get_field("day_date").unwrap_or_default(),
            starts_at: body.get_field("starts_at").unwrap_or_default(),
            ends_at:   body.get_field("ends_at").unwrap_or_default(),
        }],
    };

    // RFC-022: recurrence
    let freq_str  = body.get_field("repeat_rule").unwrap_or_default();
    let freq      = RecurrenceFreq::from_str(&freq_str);
    let rep_count = body.get_field("repeat_count")
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);

    let validated = match validate_event(input) {
        Ok(v)  => v,
        Err(e) => {
            let msg = render::escape_html(&e.to_string());
            return redirect(&format!("/c/{community_id}/admin/events/new?err={msg}"));
        }
    };

    // Expand recurrence from the single validated base day.
    let base_day = validated.days[0].clone();
    let expanded = match expand_recurrence(&base_day, freq, rep_count) {
        Ok(v)  => v,
        Err(e) => {
            let msg = render::escape_html(&e.to_string());
            return redirect(&format!("/c/{community_id}/admin/events/new?err={msg}"));
        }
    };

    // Convert community-local "HH:MM" on day_date to true UTC (RFC-018).
    // The community timezone determines the offset; unknown zones fall back
    // to UTC inside tz::offset_minutes (no silent wrong conversion).
    let community_tz = db::community::find_active(&db, community_id).await?
        .map(|c| c.timezone)
        .unwrap_or_else(|| "UTC".to_string());
    let off = zinnias_ciao_contracts::tz::offset_minutes(&community_tz);
    let days_utc: Vec<(String, String, String)> = expanded.iter().map(|d| {
        let starts = zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.starts_at, off);
        let ends   = zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.ends_at, off);
        (d.day_date.clone(), starts, ends)
    }).collect();

    let repeat_count_stored = if freq.is_recurring() { Some(expanded.len() as u32) } else { None };
    let event_id = event_write::create_event(
        &db, community_id, &membership.membership_id,
        &validated.title,
        validated.location.as_deref(),
        validated.description.as_deref(),
        &days_utc,
        freq.as_str(),
        repeat_count_stored,
    ).await?;

    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "event", Some(&event_id), "created",
        Some(serde_json::json!({ "title": validated.title })),
    ).await;

    redirect(&format!("/c/{community_id}/events/{event_id}"))
}

// ── GET /c/:cid/admin/events/:eid/cancel ─────────────────────────────────

pub async fn get_cancel_event(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::CANCEL_EVENT, Some(event_id)).await.unwrap_or_default();

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };
    let community = db::community::find_active(&db, community_id).await?;
    let _community_name = community.map(|c| c.name).unwrap_or_default();
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">Cancel this event?</h1>\
         <p style=\"font-size:.9375rem;color:#6e6e73\"><strong>{title}</strong></p>\
         <p style=\"font-size:.875rem;color:#6e6e73\">Members will still see that it was cancelled.</p>\
         <div style=\"display:flex;gap:.75rem;margin-top:1.5rem\">\
           <a href=\"/c/{cid}/events/{eid}\" \
              style=\"flex:1;padding:.875rem;border:2px solid #e5e5ea;border-radius:14px;\
              text-align:center;text-decoration:none;color:#1D1D1F;font-weight:600\">\
              Keep Event</a>\
           <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/cancel\" style=\"flex:1\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               style=\"width:100%;padding:.875rem;background:#FF3B30;color:#fff;\
               border:none;border-radius:14px;font-weight:600;min-height:44px;cursor:pointer\">\
               Cancel Event</button>\
           </form>\
         </div></main>{nav}",
        header = render::header_with_switcher(i18n::EN_ADMIN_CANCEL_EVENT_TITLE, community_id, &_community_pairs),
        title  = render::escape_html(&event.title),
        cid    = render::escape_html(community_id),
        eid    = render::escape_html(event_id),
        tok    = render::escape_html(&token),
        nav    = nav,
    );
    render::page("Cancel Event", &body)
}

// ── POST /c/:cid/admin/events/:eid/cancel ────────────────────────────────

pub async fn post_cancel_event(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
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
        token_purpose::CANCEL_EVENT, &raw_token, Some(event_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    event_write::cancel_event(&db, event_id, &membership.membership_id).await?;
    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "event", Some(event_id), "cancelled", None).await;

    redirect(&format!("/c/{community_id}/events/{event_id}"))
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

    let new_code_html = new_code.as_deref().map(|c| format!(
        "<div style=\"background:#edfaf0;border-radius:12px;padding:1rem;margin:1rem 0;border:1px solid #34C759\">\
         <p style=\"font-size:.8125rem;color:#167A34;margin:0 0 .5rem\">Share with one person only — expires in 24 hours.</p>\
         <div style=\"font-size:1.5rem;font-weight:700;letter-spacing:.2em;color:#1D1D1F\" aria-label=\"Invite code\">{code}</div>\
         </div>",
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
        let role_label = if inv.grants_role == "admin" { " · admin invite" } else { "" };
        let exp_display = inv.expires_at.get(..16).unwrap_or(&inv.expires_at);
        code_rows.push_str(&format!(
            "<li style=\"display:flex;align-items:center;justify-content:space-between;\
             padding:.625rem 0;border-bottom:1px solid #f5f5f7;gap:.5rem\">\
             <span style=\"font-size:.875rem;color:#1D1D1F\">Expires {exp}{role}</span>\
             <form method=\"post\" action=\"/c/{cid}/admin/invites/{iid}/revoke\" style=\"margin:0\">\
               <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
               <button type=\"submit\" \
                 style=\"font-size:.8125rem;color:#FF3B30;background:none;border:none;\
                 padding:.375rem .5rem;cursor:pointer;min-height:44px\" \
                 aria-label=\"Revoke this invite code\">Revoke</button>\
             </form></li>",
            exp  = render::escape_html(exp_display),
            role = role_label,
            cid  = render::escape_html(community_id),
            iid  = render::escape_html(&inv.id),
            tok  = render::escape_html(&revoke_tok),
        ));
    }
    let codes_html = if active_codes.is_empty() {
        format!("<p style=\"font-size:.875rem;color:#6e6e73\">{}</p>", i18n::EN_ADMIN_INVITES_NONE)
    } else {
        format!("<ul style=\"list-style:none;padding:0;margin:.75rem 0\">{code_rows}</ul>")
    };

    let nav = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">Invite Members</h1>\
         <p style=\"font-size:.875rem;color:#6e6e73\">Generate a one-time code for one person.</p>\
         {flash}{new_code}\
         <form method=\"post\" action=\"/c/{cid}/admin/invites\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           <button type=\"submit\" \
             style=\"width:100%;padding:.875rem;background:#007AFF;color:#fff;\
             border:none;border-radius:14px;font-size:1rem;font-weight:600;\
             min-height:44px;cursor:pointer;margin-top:.5rem\">Generate Code</button>\
         </form>\
         <section style=\"margin-top:1.5rem\">\
           <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
             text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">Active codes</h2>\
           {codes}\
         </section>\
         </main>{nav}",
        header   = render::header_with_switcher(i18n::EN_ADMIN_INVITES_TITLE, community_id, &community_pairs),
        cid      = render::escape_html(community_id),
        tok      = render::escape_html(&gen_token),
        new_code = new_code_html,
        flash    = flash_html,
        codes    = codes_html,
        nav      = nav,
    );
    render::page("Invite Members", &body)
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

    let code = generate_invite_code();
    let normalized = normalize_invite_code(&code);
    let code_hmac = hmac_hex(&pp, &normalized);
    let invite_id = random_token()[..24].to_owned();
    let expires_at = db::add_seconds_to_now(86_400);

    invite_db::insert(&db, &invite_id, community_id, &code_hmac,
        &membership.membership_id, &expires_at, "member").await?;

    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "invite_code", Some(&invite_id), "generated", None).await;

    redirect(&format!("/c/{community_id}/admin/invites?code={code}"))
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
                 style=\"color:#FF3B30;font-size:.875rem\">Remove</a>",
                cid = render::escape_html(community_id),
                mid = render::escape_html(&m.id),
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
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">Members</h1>\
         <ul style=\"list-style:none;padding:0;margin:0\">{rows}</ul>\
         <a href=\"/c/{cid}/admin/invites\" \
            style=\"display:block;margin-top:1.5rem;text-align:center;\
            padding:.875rem;border:2px solid #007AFF;border-radius:14px;\
            color:#007AFF;text-decoration:none;font-weight:600\">\
            Generate invite code</a>\
         </main>{nav}",
        header = render::header_with_switcher(i18n::EN_ADMIN_MEMBERS_TITLE, community_id, &_community_pairs),
        rows   = member_rows,
        cid    = render::escape_html(community_id),
        nav    = nav,
    );
    render::page("Members", &body)
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
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">Remove member?</h1>\
         <p style=\"font-size:.9375rem;color:#6e6e73\">\
           Remove <strong>{name}</strong> from this community?<br>\
           They will no longer be able to see events or notes.\
         </p>\
         <div style=\"display:flex;gap:.75rem;margin-top:1.5rem\">\
           <a href=\"/c/{cid}/admin/members\" \
              style=\"flex:1;padding:.875rem;border:2px solid #e5e5ea;border-radius:14px;\
              text-align:center;text-decoration:none;color:#1D1D1F;font-weight:600\">\
              Keep Member</a>\
           <form method=\"post\" \
             action=\"/c/{cid}/admin/members/{mid}/remove\" style=\"flex:1\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               style=\"width:100%;padding:.875rem;background:#FF3B30;color:#fff;\
               border:none;border-radius:14px;font-weight:600;min-height:44px;cursor:pointer\">\
               Remove</button>\
           </form>\
         </div></main>{nav}",
        header = render::header_with_switcher("Remove Member", community_id, &_community_pairs),
        name   = render::escape_html(target_name),
        cid    = render::escape_html(community_id),
        mid    = render::escape_html(target_membership_id),
        tok    = render::escape_html(&token),
        nav    = nav,
    );
    render::page("Remove Member", &body)
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
        return render::page("Cannot remove",
            "<main style=\"padding:2rem\"><p>Cannot remove the last admin. \
             Transfer the admin role first.</p></main>");
    }

    membership_db::soft_remove(&db, target_membership_id, community_id).await?;
    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "membership", Some(target_membership_id), "removed", None).await;

    redirect(&format!("/c/{community_id}/admin/members"))
}

// ── GET /c/:cid/admin/events/:eid/edit ───────────────────────────────────

pub async fn get_edit_event(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };
    if event.status == "cancelled" {
        return render::page("Cannot edit",
            "<main style=\"padding:2rem\"><p>Cancelled events cannot be edited.</p>\
             <p><a href=\"javascript:history.back()\">Back</a></p></main>");
    }
    // RFC-018: editing is only allowed while the event is still upcoming (before first day starts).
    let days = event_db::days_for_event(&db, event_id).await?;
    let now_utc = db::now_utc();
    let already_started = days.iter().any(|d| {
        classify_day(&d.starts_at_utc, &d.ends_at_utc, &now_utc) != DayTimeState::Upcoming
    });
    if already_started {
        return render::page("Cannot edit",
            "<main style=\"padding:2rem\"><p>This event has already started and cannot be edited.</p>\
             <p><a href=\"javascript:history.back()\">Back</a></p></main>");
    }

    let pp = crate::crypto::pepper(env);
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::EDIT_EVENT, Some(event_id)).await.unwrap_or_default();

    // Prefill date/time from the existing day, converted UTC → community-local.
    // Only single-day events support time editing; multi-day events edit details only.
    let is_single_day = days.len() == 1;
    let (prefill_date, prefill_start, prefill_end) = if is_single_day {
        let community_tz = db::community::find_active(&db, community_id).await?
            .map(|c| c.timezone)
            .unwrap_or_else(|| "UTC".to_string());
        let off = zinnias_ciao_contracts::tz::offset_minutes(&community_tz);
        let d = &days[0];
        let (date, start) = zinnias_ciao_contracts::tz::to_local_parts(&d.starts_at_utc, off);
        let (_, end)      = zinnias_ciao_contracts::tz::to_local_parts(&d.ends_at_utc, off);
        (Some(date), Some(start), Some(end))
    } else {
        (None, None, None)
    };

    let communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let community_pairs: Vec<(String,String)> = communities_for_switcher.iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    // Pull flash / error from query string
    let url = req.url()?;
    let err: Option<String> = url.query_pairs().find(|(k,_)| k == "err").map(|(_,v)| v.to_string());

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">Edit Event</h1>\
         <p style=\"font-size:.8125rem;color:{muted};margin-bottom:1rem\">\
           Members will see the updated event details.</p>\
         <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/edit\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {fields}\
           <button type=\"submit\" style=\"width:100%;padding:.875rem;background:{going};\
           color:#fff;border:none;border-radius:14px;font-size:1rem;font-weight:600;\
           min-height:44px;cursor:pointer;margin-top:1rem\">Save Changes</button>\
         </form>\
         <div style=\"margin-top:1.5rem\">\
           <a href=\"/c/{cid}/events/{eid}\" \
              style=\"color:{muted};font-size:.875rem\">Back to event</a>\
         </div>\
         </main>{nav}",
        header = render::header_with_switcher("Edit Event", community_id, &community_pairs),
        cid    = render::escape_html(community_id),
        eid    = render::escape_html(event_id),
        tok    = render::escape_html(&token),
        muted  = "#6E6E73",
        going  = "#007AFF",
        fields = event_form_fields(
            Some(&event.title),
            event.location.as_deref(),
            event.description.as_deref(),
            err.as_deref(),
            prefill_date.as_deref(),
            prefill_start.as_deref(),
            prefill_end.as_deref(),
            false, // edit hides recurrence
        ),
        nav = nav,
    );
    render::page("Edit Event", &body)
}

// ── POST /c/:cid/admin/events/:eid/edit ──────────────────────────────────

pub async fn post_edit_event(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
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
        token_purpose::EDIT_EVENT, &raw_token, Some(event_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    // Verify event exists and belongs to community
    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };
    if event.status == "cancelled" {
        return render::not_found();
    }
    // RFC-018: reject POST edits if the event has already started.
    let days_check = event_db::days_for_event(&db, event_id).await?;
    let now_check  = db::now_utc();
    if days_check.iter().any(|d| classify_day(&d.starts_at_utc, &d.ends_at_utc, &now_check) != DayTimeState::Upcoming) {
        return render::not_found(); // same generic response — consistent with GET guard
    }

    let input = EventInput {
        title:       body.get_field("title").unwrap_or_default(),
        location:    Some(body.get_field("location").unwrap_or_default()),
        description: Some(body.get_field("description").unwrap_or_default()),
        days:        vec![zinnias_ciao_domain::DayInput {
            day_date:  body.get_field("day_date").unwrap_or_default(),
            starts_at: body.get_field("starts_at").unwrap_or_default(),
            ends_at:   body.get_field("ends_at").unwrap_or_default(),
        }],
    };

    let validated = match validate_event(input) {
        Ok(v)  => v,
        Err(e) => {
            let msg = render::escape_html(&e.to_string());
            return redirect(&format!("/c/{community_id}/admin/events/{event_id}/edit?err={msg}"));
        }
    };

    // Determine whether this is a single-day event. Per-day time editing is
    // only supported for single-day events; multi-day/recurring events edit
    // details only (RFC-040 will define multi-day edit semantics).
    let existing_days = event_db::days_for_event(&db, event_id).await?;
    let day_utc: Option<(String, String, String)> = if existing_days.len() == 1 {
        let community_tz = db::community::find_active(&db, community_id).await?
            .map(|c| c.timezone)
            .unwrap_or_else(|| "UTC".to_string());
        let off = zinnias_ciao_contracts::tz::offset_minutes(&community_tz);
        let d = &validated.days[0];
        Some((
            d.day_date.clone(),
            zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.starts_at, off),
            zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.ends_at, off),
        ))
    } else {
        None
    };

    event_write::edit_event(
        &db, event_id,
        &validated.title,
        validated.location.as_deref(),
        validated.description.as_deref(),
        day_utc.as_ref().map(|(d, s, e)| (d.as_str(), s.as_str(), e.as_str())),
    ).await?;

    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "event", Some(event_id), "edited",
        Some(serde_json::json!({ "title": validated.title })),
    ).await;

    redirect(&format!("/c/{community_id}/events/{event_id}"))
}

// ── GET /c/:cid/admin/events/:eid/attendance ─────────────────────────────

pub async fn get_attendance(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };
    // Only allow attendance correction after the event (status=ended or any non-scheduled)
    // For MVP we allow it for any non-cancelled event (the admin controls when to correct).
    if event.status == "cancelled" {
        return render::page("Not available",
            "<main style=\"padding:2rem\"><p>Attendance cannot be corrected for a cancelled event.</p>\
             <p><a href=\"javascript:history.back()\">Back</a></p></main>");
    }

    let days = event_db::days_for_event(&db, event_id).await?;
    let members = membership_db::list_all_active(&db, community_id).await?;

    let pp = crate::crypto::pepper(env);
    // One token per (event, admin) covers the whole batch form.
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::ATTENDANCE_OVERRIDE, Some(event_id)).await.unwrap_or_default();

    let communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let community_pairs: Vec<(String,String)> = communities_for_switcher.iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    // Build one table per day (MVP events are almost always single-day)
    let mut days_html = String::new();
    for day in &days {
        let attendances = crate::db::attendance::list_for_day(&db, &day.id).await?;
        let att_map: std::collections::HashMap<&str, Option<&str>> = attendances.iter()
            .map(|a| (a.membership_id.as_str(), a.status.as_deref()))
            .collect();

        let day_label = render::escape_html(&day.day_date);
        days_html.push_str(&format!(
            "<h3 style=\"font-size:.9375rem;font-weight:600;margin:1rem 0 .5rem\">{day_label}</h3>"
        ));

        for m in &members {
            let current = att_map.get(m.id.as_str()).copied().flatten();
            let sel = |v: &str| if current == Some(v) { " selected" } else { "" };
            days_html.push_str(&format!(
                "<div style=\"display:flex;align-items:center;gap:.75rem;padding:.5rem 0;\
                 border-bottom:1px solid #F5F5F7\">\
                 <span style=\"flex:1;font-size:.9375rem\">{name}</span>\
                 <select name=\"att_{day_id}_{mid}\" \
                   style=\"font-size:.875rem;padding:.375rem .5rem;border:1px solid #E5E5EA;\
                   border-radius:8px;min-height:44px\" \
                   aria-label=\"Attendance for {name_raw}\">\
                   <option value=\"\"{no_ans}>No answer</option>\
                   <option value=\"going\"{going}>Going</option>\
                   <option value=\"not_going\"{notgoing}>Not going</option>\
                   <option value=\"attended\"{attended}>Attended</option>\
                 </select>\
                 </div>",
                name     = render::escape_html(&m.display_name),
                name_raw = render::escape_html(&m.display_name),
                day_id   = render::escape_html(&day.id),
                mid      = render::escape_html(&m.id),
                no_ans   = if current.is_none() { " selected" } else { "" },
                going    = sel("going"),
                notgoing = sel("not_going"),
                attended = sel("attended"),
            ));
        }
    }

    let flash: Option<String> = req.url()?.query_pairs()
        .find(|(k,_)| k == "flash").map(|(_,v)| v.to_string());
    let flash_html = flash.map(|f| format!(
        "<p role=\"status\" style=\"color:#167A34;font-size:.875rem;margin-bottom:1rem\">{}</p>",
        render::escape_html(&f)
    )).unwrap_or_default();

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.25rem\">Mark Attendance</h1>\
         <p style=\"font-size:.875rem;color:#6E6E73;margin-bottom:1rem\">{title}</p>\
         {flash}\
         <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/attendance\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {days}\
           <button type=\"submit\" \
             style=\"width:100%;padding:.875rem;background:#007AFF;color:#fff;\
             border:none;border-radius:14px;font-size:1rem;font-weight:600;\
             min-height:44px;cursor:pointer;margin-top:1.5rem\">Save Attendance</button>\
         </form>\
         <div style=\"margin-top:1rem\">\
           <a href=\"/c/{cid}/events/{eid}\" style=\"color:#6E6E73;font-size:.875rem\">\
             Back to event</a>\
         </div>\
         </main>{nav}",
        header = render::header_with_switcher(i18n::EN_ADMIN_ATTEND_TITLE, community_id, &community_pairs),
        title  = render::escape_html(&event.title),
        cid    = render::escape_html(community_id),
        eid    = render::escape_html(event_id),
        tok    = render::escape_html(&token),
        days   = days_html,
        flash  = flash_html,
        nav    = nav,
    );
    render::page("Mark Attendance", &body)
}

// ── POST /c/:cid/admin/events/:eid/attendance ────────────────────────────

pub async fn post_attendance(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let form = req.form_data().await?;
    let raw_token = form.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(&db, &pp, &auth.user_id,
        token_purpose::ATTENDANCE_OVERRIDE, &raw_token, Some(event_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    // Verify event is in scope
    if event_db::find_for_community(&db, event_id, community_id).await?.is_none() {
        return render::not_found();
    }

    let days = event_db::days_for_event(&db, event_id).await?;
    let members = membership_db::list_all_active(&db, community_id).await?;

    let mut changes: u32 = 0;
    for day in &days {
        for m in &members {
            let field_name = format!("att_{}_{}", day.id, m.id);
            let value = form.get_field(&field_name).unwrap_or_default();
            let status: Option<&str> = match value.as_str() {
                "going"     => Some("going"),
                "not_going" => Some("not_going"),
                "attended"  => Some("attended"),
                _           => None, // "" → clear to No answer
            };
            crate::db::attendance::upsert(&db, &day.id, &m.id, status).await?;
            changes += 1;
        }
    }

    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "attendance", Some(event_id), "admin_override",
        Some(serde_json::json!({ "changes": changes })),
    ).await;

    redirect(&format!("/c/{community_id}/admin/events/{event_id}/attendance?flash=Saved"))
}

// ── GET /c/:cid/admin/events/:eid/notes/:mid/hide ────────────────────────
// No-JS confirmation page for admin note removal (RFC-043).

pub async fn get_admin_hide_note_confirm(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::ADMIN_HIDE_NOTE, Some(event_id)).await.unwrap_or_default();

    // Resolve the target member's display name for the confirmation copy.
    let all = membership_db::list_all_active(&db, community_id).await?;
    let target_name = all.iter()
        .find(|m| m.id == target_membership_id)
        .map(|m| m.display_name.as_str())
        .unwrap_or("this member");

    let communities = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await.unwrap_or_default();
    let pairs: Vec<(String, String)> = communities.iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">Remove note?</h1>\
           <p style=\"font-size:.9375rem;color:#6E6E73;margin-bottom:1.5rem\">\
             Remove the note from {name}? This cannot be undone.</p>\
           <div style=\"display:flex;gap:.75rem\">\
             <a href=\"/c/{cid}/events/{eid}\" \
                style=\"flex:1;padding:.875rem;border:2px solid #e5e5ea;border-radius:14px;\
                text-align:center;text-decoration:none;color:#1D1D1F;font-weight:600;min-height:44px;\
                display:flex;align-items:center;justify-content:center\">Keep note</a>\
             <form method=\"post\" \
                   action=\"/c/{cid}/admin/events/{eid}/notes/{mid}/hide\" style=\"flex:1\">\
               <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
               <button type=\"submit\" \
                 style=\"width:100%;padding:.875rem;background:#FF3B30;color:#fff;\
                 border:none;border-radius:14px;font-weight:600;min-height:44px;cursor:pointer\">\
                 Remove note</button>\
             </form>\
           </div>\
         </main>{nav}",
        header = render::header_with_switcher("Remove note", community_id, &pairs),
        name   = render::escape_html(target_name),
        cid    = render::escape_html(community_id),
        eid    = render::escape_html(event_id),
        mid    = render::escape_html(target_membership_id),
        tok    = render::escape_html(&token),
        nav    = nav,
    );
    render::page("Remove note", &body)
}

// ── POST /c/:cid/admin/events/:eid/notes/:mid/hide ───────────────────────

pub async fn post_admin_hide_note(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
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
        token_purpose::ADMIN_HIDE_NOTE, &raw_token, Some(event_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    // Verify event belongs to this community
    if event_db::find_for_community(&db, event_id, community_id).await?.is_none() {
        return render::not_found();
    }

    crate::db::event_note::admin_hide(&db, event_id, target_membership_id).await?;

    // Audit without note body content (RFC-014)
    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "event_note", Some(event_id), "admin_hidden",
        Some(serde_json::json!({ "target_membership_id": target_membership_id })),
    ).await;

    redirect(&format!("/c/{community_id}/events/{event_id}?flash=Note+removed"))
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Generate a 6-char invite code from the safe alphabet (no ambiguous chars).
fn generate_invite_code() -> String {
    use zinnias_ciao_domain::invite::INVITE_CODE_ALPHABET;
    let mut bytes = [0u8; 6];
    getrandom::getrandom(&mut bytes).unwrap_or_default();
    bytes.iter()
        .map(|&b| INVITE_CODE_ALPHABET[b as usize % INVITE_CODE_ALPHABET.len()] as char)
        .collect()
}

/// Event form shared fields (create and edit).
/// `day_date`/`starts_at`/`ends_at` prefill the date and time inputs (edit case).
/// `show_recurrence` renders the repeat selector (create only); edit hides it.
fn event_form_fields(
    title: Option<&str>,
    location: Option<&str>,
    description: Option<&str>,
    error: Option<&str>,
    day_date: Option<&str>,
    starts_at: Option<&str>,
    ends_at: Option<&str>,
    show_recurrence: bool,
) -> String {
    let err_html = error.map(|e| format!(
        "<p role=\"alert\" style=\"color:#FF3B30;font-size:.875rem\">{}</p>",
        render::escape_html(e)
    )).unwrap_or_default();

    let field = |label: &str, name: &str, ftype: &str, val: &str, required: bool| {
        let req_attr = if required { " required" } else { "" };
        format!(
            "<label style=\"display:block;margin-bottom:1rem\">\
             <span style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">{label}</span>\
             <input type=\"{ftype}\" name=\"{name}\" value=\"{val}\" \
               style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
               border-radius:12px;font-size:1rem\"{req_attr}>\
             </label>",
            label = label,
            ftype = ftype,
            name  = name,
            val   = render::escape_html(val),
        )
    };

    // RFC-022: repeat fields (create only — edit hides recurrence).
    let repeat_html = if show_recurrence {
        format!(
        "<div style=\"margin-bottom:1rem\">\
         <label style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">{repeat_lbl}</label>\
         <div style=\"display:flex;gap:.75rem;align-items:center\">\
           <select name=\"repeat_rule\" style=\"padding:.625rem;border:1px solid #e5e5ea;\
             border-radius:12px;font-size:1rem;flex:1\">\
             <option value=\"none\">{opt_none}</option>\
             <option value=\"weekly\">{opt_weekly}</option>\
             <option value=\"biweekly\">{opt_biweekly}</option>\
             <option value=\"monthly\">{opt_monthly}</option>\
           </select>\
           <input type=\"number\" name=\"repeat_count\" value=\"8\" min=\"1\" max=\"52\"\
             style=\"width:5rem;padding:.625rem;border:1px solid #e5e5ea;\
             border-radius:12px;font-size:1rem\">\
           <span style=\"font-size:.875rem;color:#6e6e73\">{unit}</span>\
         </div>\
         <p style=\"font-size:.75rem;color:#6e6e73;margin:.25rem 0 0\">{hint}</p>\
         </div>",
        repeat_lbl = i18n::EN_REPEAT_LABEL,
        opt_none   = i18n::EN_REPEAT_NONE,
        opt_weekly = i18n::EN_REPEAT_WEEKLY,
        opt_biweekly = i18n::EN_REPEAT_BIWEEKLY,
        opt_monthly  = i18n::EN_REPEAT_MONTHLY,
        unit       = i18n::EN_REPEAT_COUNT_UNIT,
        hint       = i18n::EN_REPEAT_COUNT_HINT,
        )
    } else {
        String::new()
    };

    format!(
        "{err}\
         {title}\
         {date}\
         {start}\
         {end}\
         {loc}\
         {repeat}\
         {desc}",
        err    = err_html,
        title  = field("Title", "title", "text", title.unwrap_or(""), true),
        date   = field("Date", "day_date", "date", day_date.unwrap_or(""), true),
        start  = field("Start time", "starts_at", "time", starts_at.unwrap_or(""), true),
        end    = field("End time", "ends_at", "time", ends_at.unwrap_or(""), true),
        repeat = repeat_html,
        loc    = field("Location (optional)", "location", "text",
                      location.unwrap_or(""), false),
        desc  = {
            let dval = render::escape_html(description.unwrap_or(""));
            format!(
                "<label style=\"display:block;margin-bottom:1rem\">\
                 <span style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">\
                 Description (optional)</span>\
                 <textarea name=\"description\" rows=\"3\" \
                   style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
                   border-radius:12px;font-size:1rem\">{dval}</textarea>\
                 </label>"
            )
        },
    )
}
