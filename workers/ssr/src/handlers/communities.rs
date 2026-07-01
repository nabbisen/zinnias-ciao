//! Communities list + switcher (RFC-005 §6 / external-design §8.5).

use worker::{Env, Request, Response, Result};

use crate::db::membership as membership_db;
use crate::render;
use zinnias_ciao_contracts::i18n;
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

    // list_communities_for_user gives both community_id and community_name.
    let summaries = membership_db::list_communities_for_user(&db, &auth.user_id).await?;

    // We also need roles — fetch memberships for the role check.
    let memberships = membership_db::list_active_for_user(&db, &auth.user_id).await?;
    let role_map: std::collections::HashMap<&str, &str> = memberships.iter()
        .map(|m| (m.community_id.as_str(), m.role.as_str()))
        .collect();

    let _current_name = summaries.iter()
        .find(|s| s.community_id == community_id)
        .map(|s| s.community_name.as_str())
        .unwrap_or("");

    let rows: String = summaries.iter().map(|s| {
        let is_current = s.community_id == community_id;
        let role = role_map.get(s.community_id.as_str()).copied().unwrap_or("member");
        let is_admin = role == "admin";
        let role_label = if is_admin { i18n::JA_ROLE_ADMIN } else { i18n::JA_ROLE_MEMBER };

        let current_badge = if is_current {
            format!("<span style=\"font-size:.75rem;background:#007AFF;color:#fff;\
             border-radius:99px;padding:.125rem .5rem;margin-left:.5rem\">{}</span>",
             i18n::JA_CURRENT_BADGE)
        } else { String::new() };

        // Admin management links — shown only for communities where user is admin.
        let admin_links = if is_admin {
            format!(
                "<div style=\"display:flex;gap:1rem;margin-top:.5rem\">\
                   <a href=\"/c/{cid}/admin/invites\" \
                      style=\"font-size:.8125rem;color:#007AFF;text-decoration:none\">\
                      {invite}</a>\
                   <a href=\"/c/{cid}/admin/members\" \
                      style=\"font-size:.8125rem;color:#007AFF;text-decoration:none\">\
                      {manage}</a>\
                 </div>",
                cid = render::escape_html(&s.community_id),
                invite = i18n::JA_ADMIN_INVITES_TITLE,
                manage = i18n::JA_ADMIN_MEMBERS_TITLE,
            )
        } else {
            String::new()
        };

        format!(
            "<li style=\"padding:.875rem 0;border-bottom:1px solid #f5f5f7\">\
               <a href=\"/c/{cid}/home\" \
                  style=\"display:flex;align-items:center;justify-content:space-between;\
                  text-decoration:none;color:inherit\">\
                 <span>\
                   <span style=\"font-size:1rem;font-weight:{w}\">{name}{badge}</span><br>\
                   <span style=\"font-size:.8125rem;color:#6e6e73\">{role}</span>\
                 </span>\
                 <span style=\"color:#c7c7cc;font-size:1.25rem\">\u{203A}</span>\
               </a>\
               {admin_links}\
             </li>",
            cid         = render::escape_html(&s.community_id),
            name        = render::escape_html(&s.community_name),
            badge       = current_badge,
            role        = role_label,
            w           = if is_current { "600" } else { "400" },
            admin_links = admin_links,
        )
    }).collect();

    // Header uses list_communities_for_user result as switcher pairs.
    let community_pairs: Vec<(String, String)> = summaries.iter()
        .map(|s| (s.community_id.clone(), s.community_name.clone()))
        .collect();

    let nav  = render::bottom_nav(community_id, "communities");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           <ul style=\"list-style:none;padding:0;margin:0\">{rows}</ul>\
           <a href=\"/join\" \
              style=\"display:block;margin-top:1.5rem;text-align:center;\
              padding:.875rem;border:2px solid #007AFF;border-radius:14px;\
              color:#007AFF;text-decoration:none;font-weight:600\">\
              {join_another}</a>\
         </main>{nav}",
        header       = render::header_with_switcher(i18n::JA_NAV_COMMUNITIES, community_id, &community_pairs),
        rows         = rows,
        join_another = i18n::JA_COMMUNITIES_JOIN_ANOTHER,
        nav          = nav,
    );
    render::page(i18n::JA_NAV_COMMUNITIES, &body)
}
