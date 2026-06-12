//! Communities list + switcher (RFC-005 §6 / external-design §8.5).

use worker::{Env, Request, Response, Result};

use crate::db::{self, membership as membership_db};
use crate::render;
use crate::session::require_auth;

pub async fn get_communities(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let db = env.d1("DB")?;
    let memberships = membership_db::list_active_for_user(&db, &auth.user_id).await?;

    let current = memberships.iter().find(|m| m.community_id == community_id);
    let current_name = if let Some(m) = current {
        db::community::find_active(&db, &m.community_id).await?
            .map(|c| c.name).unwrap_or_default()
    } else { String::new() };

    let rows: String = memberships.iter().map(|m| {
        let is_current = m.community_id == community_id;
        let role_label = if m.role == "admin" { "Admin" } else { "Member" };
        let current_badge = if is_current {
            "<span style=\"font-size:.75rem;background:#007AFF;color:#fff;\
             border-radius:99px;padding:.125rem .5rem;margin-left:.5rem\">Current</span>"
        } else { "" };
        let cname = ""; // we'd fetch per community — kept simple here
        format!(
            "<li>\
             <a href=\"/c/{cid}/home\" \
                style=\"display:flex;align-items:center;justify-content:space-between;\
                padding:.875rem 0;border-bottom:1px solid #f5f5f7;text-decoration:none;color:inherit\">\
               <span>\
                 <span style=\"font-size:1rem;font-weight:{w}\">{cid_display}{badge}</span><br>\
                 <span style=\"font-size:.8125rem;color:#6e6e73\">{role}</span>\
               </span>\
               <span style=\"color:#c7c7cc;font-size:1.25rem\">\u{203A}</span>\
             </a></li>",
            cid         = render::escape_html(&m.community_id),
            cid_display = render::escape_html(&m.community_id), // replaced by name when fetched
            badge       = current_badge,
            role        = role_label,
            w           = if is_current { "600" } else { "400" },
        )
    }).collect();

    let nav  = render::bottom_nav(community_id, "communities");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           <ul style=\"list-style:none;padding:0;margin:0\">{rows}</ul>\
           <a href=\"/join\" \
              style=\"display:block;margin-top:1.5rem;text-align:center;\
              padding:.875rem;border:2px solid #007AFF;border-radius:14px;\
              color:#007AFF;text-decoration:none;font-weight:600\">\
              Join another community</a>\
         </main>{nav}",
        header = render::header("Communities", &current_name),
        rows   = rows,
        nav    = nav,
    );
    render::page("Communities", &body)
}
