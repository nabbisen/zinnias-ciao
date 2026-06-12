//! Me / profile handler (RFC-005 §6 / external-design §8.6).

use zinnias_ciao_contracts::auth::token_purpose;
use worker::{Env, Request, Response, Result};

use crate::authz::require_membership;
use zinnias_ciao_contracts::i18n;
use crate::db::{self, membership as membership_db};
use crate::form_token;
use crate::render;
use crate::session::require_auth;

pub async fn get_me(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = env.secret("HMAC_PEPPER").map(|s| s.to_string())
        .unwrap_or_else(|_| "dev-pepper-change-in-production".to_string());

    let logout_token = form_token::issue(
        &db, &pp, &auth.user_id, token_purpose::LOGOUT, None,
    ).await.unwrap_or_default();

    let community = db::community::find_active(&db, community_id).await?;
    let community_name = community.as_ref().map(|c| c.name.as_str()).unwrap_or("");
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let role_label = if membership.is_admin() { i18n::EN_ROLE_ADMIN } else { i18n::EN_ROLE_MEMBER };

    // RFC-035: support diagnostics
    let app_version = env.var("BUILD_VERSION")
        .map(|v| v.to_string()).unwrap_or_else(|_| "dev".to_owned());
    // Short community reference (first 8 chars of community_id) for support context.
    let support_ref = community_id.get(..8).unwrap_or(community_id);

    let admin_export_html: String = if membership.is_admin() {
        format!(
            "<section style=\"margin-top:1.5rem\"><h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">Data</h2><a href=\"/c/{cid}/admin/export\" style=\"display:block;font-size:.9375rem;color:#007AFF;padding:.375rem 0;min-height:44px;line-height:44px\">Export community data</a></section>",
            cid = render::escape_html(community_id)
        )
    } else { String::new() };

    let nav  = render::bottom_nav(community_id, "me");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           <section style=\"margin-bottom:1.5rem\">\
             <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
             text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">{lbl_name}</h2>\
             <p style=\"font-size:1rem;margin:0\">{name}</p>\
           </section>\
           <section style=\"margin-bottom:1.5rem\">\
             <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
             text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">\
             {lbl_community}</h2>\
             <p style=\"font-size:1rem;margin:0\">{community} · {role}</p>\
           </section>\
           <section style=\"margin-bottom:1.5rem\">\
             <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
             text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">{lbl_help}</h2>\
             <p style=\"font-size:.875rem;color:#6e6e73;margin:0\">\
             {help_body}</p>\
           </section>\
           <section style=\"margin-top:1.5rem\">\
             <h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;\
               text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">Calendar</h2>\
             <a href=\"/c/{cid}/me/calendar\" \
               style=\"display:block;font-size:.9375rem;color:#007AFF;padding:.375rem 0;\
               min-height:44px;line-height:44px\">Calendar feed</a>\
           </section>\
           {admin_export}\
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
        header    = render::header_with_switcher(i18n::EN_NAV_ME, community_id, &_community_pairs),
        name      = render::escape_html(&membership.display_name),
        community = render::escape_html(community_name),
        role      = role_label,
        cid       = render::escape_html(community_id),
        lbl_name      = i18n::EN_ME_SECTION_NAME,
        lbl_community = i18n::EN_ME_SECTION_COMMUNITY,
        lbl_help      = i18n::EN_ME_SECTION_HELP,
        help_body     = i18n::EN_ME_HELP_BODY,
        lbl_logout    = i18n::EN_LOGOUT,
        lbl_about     = i18n::EN_ME_SECTION_ABOUT,
        lbl_version   = i18n::EN_ME_VERSION_LABEL,
        lbl_ref       = i18n::EN_ME_REF_LABEL,
        version       = render::escape_html(&app_version),
        ref_code      = render::escape_html(support_ref),
        admin_export  = admin_export_html,
        tok           = render::escape_html(&logout_token),
        nav       = nav,
    );
    render::page(i18n::EN_NAV_ME, &body)
}
