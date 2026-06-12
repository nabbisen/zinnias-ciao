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
use zinnias_ciao_domain::{validate_event, DayInput, EventInput};

fn pepper(env: &Env) -> String {
    env.secret("HMAC_PEPPER")
        .map(|s| s.to_string())
        .unwrap_or_else(|_| "dev-pepper-change-in-production".to_string())
}

fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

// ── GET /c/:cid/admin/events/new ─────────────────────────────────────────

pub async fn get_create_event(
    req: Request,
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
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::CREATE_EVENT, None).await.unwrap_or_default();

    let community = db::community::find_active(&db, community_id).await?;
    let community_name = community.map(|c| c.name).unwrap_or_default();
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">Create Event</h1>\
         <form method=\"post\" action=\"/c/{cid}/admin/events\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {fields}\
           <button type=\"submit\" style=\"width:100%;padding:.875rem;background:#007AFF;\
           color:#fff;border:none;border-radius:14px;font-size:1rem;font-weight:600;\
           min-height:44px;cursor:pointer;margin-top:1rem\">Create Event</button>\
         </form></main>{nav}",
        header = render::header_with_switcher("Create Event", community_id, &_community_pairs),
        cid    = render::escape_html(community_id),
        tok    = render::escape_html(&token),
        fields = event_form_fields(None, None, None, None),
        nav    = nav,
    );
    render::page("Create Event", &body)
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
    let pp = pepper(env);

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

    let validated = match validate_event(input) {
        Ok(v)  => v,
        Err(e) => {
            let msg = render::escape_html(&e.to_string());
            return redirect(&format!("/c/{community_id}/admin/events/new?err={msg}"));
        }
    };

    // Convert local "HH:MM" on day_date to a UTC-like ISO string.
    // In MVP we store times as entered (community TZ handling is RFC-018).
    let days_utc: Vec<(String, String, String)> = validated.days.iter().map(|d| {
        let starts = format!("{}T{}:00.000Z", d.day_date, d.starts_at);
        let ends   = format!("{}T{}:00.000Z", d.day_date, d.ends_at);
        (d.day_date.clone(), starts, ends)
    }).collect();

    let event_id = event_write::create_event(
        &db, community_id, &membership.membership_id,
        &validated.title,
        validated.location.as_deref(),
        validated.description.as_deref(),
        &days_utc,
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
    let pp = pepper(env);
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::CANCEL_EVENT, Some(event_id)).await.unwrap_or_default();

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };
    let community = db::community::find_active(&db, community_id).await?;
    let community_name = community.map(|c| c.name).unwrap_or_default();
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
        header = render::header_with_switcher("Cancel Event", community_id, &_community_pairs),
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
    let pp = pepper(env);

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
    let token = form_token::issue(&db, &pp, &auth.user_id,
        "generate_invite", None).await.unwrap_or_default();

    let community = db::community::find_active(&db, community_id).await?;
    let community_name = community.map(|c| c.name).unwrap_or_default();
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();

    // Check for a just-generated code in query param
    let url = req.url()?;
    let new_code: Option<String> = url.query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string());

    let code_html = new_code.as_deref().map(|c| format!(
        "<div style=\"background:#f5f5f7;border-radius:12px;padding:1rem;margin:1rem 0\">\
         <p style=\"font-size:.875rem;color:#6e6e73;margin-bottom:.5rem\">\
         Share this with one person. It expires in 24 hours.</p>\
         <div style=\"font-size:1.5rem;font-weight:700;letter-spacing:.2em;color:#1D1D1F\">{code}</div>\
         </div>",
        code = render::escape_html(c)
    )).unwrap_or_default();

    let nav  = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">Invite Members</h1>\
         <p style=\"font-size:.875rem;color:#6e6e73\">Generate a one-time invite code.</p>\
         {code_html}\
         <form method=\"post\" action=\"/c/{cid}/admin/invites\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           <button type=\"submit\" \
             style=\"width:100%;padding:.875rem;background:#007AFF;color:#fff;\
             border:none;border-radius:14px;font-size:1rem;font-weight:600;\
             min-height:44px;cursor:pointer;margin-top:1rem\">Generate Code</button>\
         </form></main>{nav}",
        header    = render::header_with_switcher("Invite Members", community_id, &_community_pairs),
        cid       = render::escape_html(community_id),
        tok       = render::escape_html(&token),
        code_html = code_html,
        nav       = nav,
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
    let pp = pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(&db, &pp, &auth.user_id,
        "generate_invite", &raw_token, None).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/admin/invites"));
    }

    // Generate code from the safe alphabet (no ambiguous chars)
    let code = generate_invite_code();
    let normalized = normalize_invite_code(&code);
    let code_hmac = hmac_hex(&pp, &normalized);

    let invite_id = random_token()[..24].to_owned();
    let expires_at = db::add_seconds_to_now(86_400); // 24 hours

    invite_db::insert(&db, &invite_id, community_id, &code_hmac,
        &membership.membership_id, &expires_at, "member").await?;

    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "invite_code", Some(&invite_id), "generated", None).await;

    // Show the plaintext code once via redirect query param (never stored)
    redirect(&format!("/c/{community_id}/admin/invites?code={code}"))
}

// ── GET /c/:cid/admin/members ────────────────────────────────────────────

pub async fn get_members(
    req: Request,
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
    let community = db::community::find_active(&db, community_id).await?;
    let community_name = community.map(|c| c.name).unwrap_or_default();
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
        header = render::header_with_switcher("Members", community_id, &_community_pairs),
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
    rid: &str,
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
    let pp = pepper(env);
    let token = form_token::issue(&db, &pp, &auth.user_id,
        "remove_member", Some(target_membership_id)).await.unwrap_or_default();

    // Find the target member name
    let all = membership_db::list_all_active(&db, community_id).await?;
    let target_name = all.iter()
        .find(|m| m.id == target_membership_id)
        .map(|m| m.display_name.as_str())
        .unwrap_or("this member");

    let community = db::community::find_active(&db, community_id).await?;
    let community_name = community.map(|c| c.name).unwrap_or_default();
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
    let pp = pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(&db, &pp, &auth.user_id,
        "remove_member", &raw_token, Some(target_membership_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/admin/members"));
    }

    // Last-admin guard (RFC-010 §5)
    let admin_count = membership_db::count_admins(&db, community_id).await?;
    let target_role = membership_db::get_role(&db, target_membership_id).await?;
    if admin_count <= 1 && target_role.as_deref() == Some("admin") {
        return render::page("Cannot remove",
            "<main style=\"padding:2rem\"><p>Cannot remove the last admin. \
             Transfer the admin role first.</p></main>");
    }

    membership_db::soft_remove(&db, target_membership_id).await?;
    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "membership", Some(target_membership_id), "removed", None).await;

    redirect(&format!("/c/{community_id}/admin/members"))
}

// ── GET /c/:cid/admin/events/:eid/edit ───────────────────────────────────

pub async fn get_edit_event(
    req: Request,
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

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };
    if event.status == "cancelled" {
        return render::page("Cannot edit",
            "<main style=\"padding:2rem\"><p>Cancelled events cannot be edited.</p>\
             <p><a href=\"javascript:history.back()\">Back</a></p></main>");
    }

    let pp = pepper(env);
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::EDIT_EVENT, Some(event_id)).await.unwrap_or_default();

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
    let pp = pepper(env);

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

    event_write::edit_event(
        &db, event_id,
        &validated.title,
        validated.location.as_deref(),
        validated.description.as_deref(),
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

    let pp = pepper(env);
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
        header = render::header_with_switcher("Mark Attendance", community_id, &community_pairs),
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
    let pp = pepper(env);

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
    let pp = pepper(env);

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
fn event_form_fields(
    title: Option<&str>,
    location: Option<&str>,
    description: Option<&str>,
    error: Option<&str>,
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

    format!(
        "{err}\
         {title}\
         {date}\
         {start}\
         {end}\
         {loc}\
         {desc}",
        err   = err_html,
        title = field("Title", "title", "text", title.unwrap_or(""), true),
        date  = field("Date", "day_date", "date", "", true),
        start = field("Start time", "starts_at", "time", "", true),
        end   = field("End time", "ends_at", "time", "", true),
        loc   = field("Location (optional)", "location", "text",
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
