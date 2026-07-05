//! Me / profile handler (RFC-005 §6 / external-design §8.6).

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;

use crate::authz::require_membership;
use crate::db::{self, membership as membership_db};
use crate::render;
use crate::session::require_auth;
use zinnias_ciao_contracts::i18n;

pub async fn get_me(req: Request, env: &Env, _rid: &str, community_id: &str) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let logout_token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::LOGOUT, None).await;

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

    let admin_export_html: String = if membership.is_admin() {
        format!(
            "<section style=\"margin-top:1.5rem\"><h2 style=\"font-size:.8125rem;font-weight:600;color:#6e6e73;text-transform:uppercase;letter-spacing:.05em;margin-bottom:.5rem\">{data_section}</h2><a href=\"/c/{cid}/admin/export\" style=\"display:block;font-size:.9375rem;color:#007AFF;padding:.375rem 0;min-height:44px;line-height:44px\">{export_lbl}</a></section>",
            cid = render::escape_html(community_id),
            data_section = i18n::JA_ME_SECTION_DATA,
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
             <p style=\"font-size:1rem;margin:0\">{name}</p>\
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
        header = render::header_with_switcher(i18n::JA_NAV_ME, community_id, &_community_pairs),
        name = render::escape_html(&membership.display_name),
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
        admin_export = admin_export_html,
        tok = render::escape_html(&logout_token),
        nav = nav,
    );
    render::page(i18n::JA_NAV_ME, &body)
}
