//! Community creation flow (RFC-057).

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;
use zinnias_ciao_domain::{
    CommunityNameError, DisplayNameError, validate_community_name, validate_display_name,
};

use crate::authz::{MembershipContext, require_active_admin_somewhere};
use crate::crypto::random_token;
use crate::rate_limit;
use crate::render::{self, escape_html};
use crate::session::require_auth;

const COMMUNITY_CREATE_PATH: &str = "/communities/new";
const SUPPORTED_TIMEZONE: &str = "Asia/Tokyo";

pub(crate) fn community_creation_enabled(env: &Env) -> bool {
    env.var("COMMUNITY_CREATION_ENABLED")
        .map(|v| {
            matches!(
                v.to_string().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub async fn get_new_community(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let admin = require_active_admin_somewhere(env, &auth).await?;

    if !community_creation_enabled(env) {
        return render_disabled(&admin);
    }

    let token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::CREATE_COMMUNITY, None).await;
    render_form(
        &admin,
        &token,
        "",
        &admin.display_name,
        SUPPORTED_TIMEZONE,
        None,
    )
}

pub async fn post_new_community(mut req: Request, env: &Env, _rid: &str) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let admin = require_active_admin_somewhere(env, &auth).await?;

    if !community_creation_enabled(env) {
        return render_disabled(&admin);
    }

    let client_ip = rate_limit::client_ip(&req);
    let form = req.form_data().await?;
    let raw_token = form.get_field("_token").unwrap_or_default();
    let raw_name = form.get_field("community_name").unwrap_or_default();
    let raw_display_name = form.get_field("display_name").unwrap_or_default();
    let timezone = form
        .get_field("timezone")
        .unwrap_or_else(|| SUPPORTED_TIMEZONE.to_owned());

    if rate_limit::is_community_creation_limited(env, &auth.user_id, &auth.session_id, &client_ip)
        .await
    {
        return refresh_form(
            env,
            &auth.user_id,
            &admin,
            &raw_name,
            &raw_display_name,
            &timezone,
            Some(i18n::JA_COMMUNITY_CREATE_RATE_LIMITED),
        )
        .await;
    }

    let community_name = match validate_community_name(&raw_name) {
        Ok(name) => name,
        Err(err) => {
            return refresh_form(
                env,
                &auth.user_id,
                &admin,
                &raw_name,
                &raw_display_name,
                &timezone,
                Some(community_name_error(err)),
            )
            .await;
        }
    };

    let display_name = match validate_display_name(&raw_display_name) {
        Ok(name) => name,
        Err(err) => {
            return refresh_form(
                env,
                &auth.user_id,
                &admin,
                &raw_name,
                &raw_display_name,
                &timezone,
                Some(display_name_error(err)),
            )
            .await;
        }
    };

    if timezone != SUPPORTED_TIMEZONE {
        return refresh_form(
            env,
            &auth.user_id,
            &admin,
            &raw_name,
            &raw_display_name,
            &timezone,
            Some(i18n::JA_COMMUNITY_CREATE_TIMEZONE_ERROR),
        )
        .await;
    }

    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::CREATE_COMMUNITY,
        &raw_token,
        None,
    )
    .await?;
    if let Some(community_id) = replay {
        return redirect(&format!("/c/{community_id}/home"));
    }

    let db = env.d1("DB")?;
    let community_id = format!("com_{}", &random_token()[..24]);
    let membership_id = format!("mem_{}", &random_token()[..24]);
    crate::db::community::create_with_first_admin(
        &db,
        &community_id,
        &community_name,
        SUPPORTED_TIMEZONE,
        &membership_id,
        &auth.user_id,
        &display_name,
    )
    .await?;

    let pepper = crate::crypto::pepper(env);
    crate::form_token::set_result(&db, &pepper, &raw_token, &community_id).await?;
    rate_limit::record_community_creation(env, &auth.user_id, &auth.session_id, &client_ip).await;

    redirect(&format!("/c/{community_id}/home"))
}

async fn refresh_form(
    env: &Env,
    user_id: &str,
    admin: &MembershipContext,
    community_name: &str,
    display_name: &str,
    timezone: &str,
    error: Option<&str>,
) -> Result<Response> {
    let token =
        crate::codlet::issue_token(env, user_id, token_purpose::CREATE_COMMUNITY, None).await;
    render_form(admin, &token, community_name, display_name, timezone, error)
}

fn render_disabled(admin: &MembershipContext) -> Result<Response> {
    let body = format!(
        "{header}<main style=\"padding:1rem 1rem 5rem;max-width:560px;margin:0 auto\">\
           <p style=\"font-size:.9375rem;color:#6e6e73;margin:0 0 1rem\">{msg}</p>\
           <a href=\"/c/{cid}/me\" style=\"display:inline-block;color:#007AFF;\
             min-height:44px;line-height:44px;text-decoration:none\">{cancel}</a>\
         </main>{nav}",
        header = render::header(i18n::JA_COMMUNITY_CREATE_TITLE, ""),
        msg = i18n::JA_COMMUNITY_CREATE_DISABLED,
        cid = escape_html(&admin.community_id),
        cancel = i18n::JA_COMMUNITY_CREATE_CANCEL,
        nav = render::bottom_nav(&admin.community_id, "me"),
    );
    render::page(i18n::JA_COMMUNITY_CREATE_TITLE, &body)
}

fn render_form(
    admin: &MembershipContext,
    token: &str,
    community_name: &str,
    display_name: &str,
    timezone: &str,
    error: Option<&str>,
) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" style=\"color:#FF3B30;margin:.75rem 0 0\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();

    let body = format!(
        "{header}<main style=\"padding:1rem 1rem 5rem;max-width:560px;margin:0 auto\">\
           <p style=\"font-size:.9375rem;color:#6e6e73;margin:0 0 1rem\">{body}</p>\
           {error_html}\
           <form method=\"post\" action=\"{path}\" style=\"margin-top:1rem\">\
             <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
             <input type=\"hidden\" name=\"timezone\" value=\"{tz}\">\
             <label style=\"display:block;font-size:.875rem;font-weight:600;margin-bottom:.375rem\" for=\"community_name\">{name_label}</label>\
             <input id=\"community_name\" name=\"community_name\" value=\"{name}\" required maxlength=\"80\" autocomplete=\"organization\" \
               style=\"width:100%;box-sizing:border-box;font-size:1rem;padding:.75rem;border:1px solid #D1D1D6;border-radius:8px;min-height:44px\">\
             <label style=\"display:block;font-size:.875rem;font-weight:600;margin:1rem 0 .375rem\" for=\"display_name\">{display_label}</label>\
             <input id=\"display_name\" name=\"display_name\" value=\"{display}\" required maxlength=\"40\" autocomplete=\"name\" \
               style=\"width:100%;box-sizing:border-box;font-size:1rem;padding:.75rem;border:1px solid #D1D1D6;border-radius:8px;min-height:44px\">\
             <div style=\"margin-top:1rem\">\
               <span style=\"display:block;font-size:.875rem;font-weight:600;margin-bottom:.375rem\">{tz_label}</span>\
               <span style=\"display:inline-block;font-size:.9375rem;color:#1D1D1F;padding:.625rem .75rem;background:#F5F5F7;border-radius:8px\">{tz_name}</span>\
             </div>\
             <button type=\"submit\" style=\"width:100%;margin-top:1.25rem;padding:.875rem;background:#007AFF;color:#fff;border:none;border-radius:8px;font-size:1rem;font-weight:600;min-height:44px;cursor:pointer\">{submit}</button>\
           </form>\
           <a href=\"/c/{cid}/me\" style=\"display:inline-block;margin-top:.75rem;color:#007AFF;min-height:44px;line-height:44px;text-decoration:none\">{cancel}</a>\
         </main>{nav}",
        header = render::header(i18n::JA_COMMUNITY_CREATE_TITLE, ""),
        body = i18n::JA_COMMUNITY_CREATE_BODY,
        error_html = error_html,
        path = COMMUNITY_CREATE_PATH,
        token = escape_html(token),
        tz = escape_html(timezone),
        name_label = i18n::JA_COMMUNITY_CREATE_NAME_LABEL,
        name = escape_html(community_name),
        display_label = i18n::JA_COMMUNITY_CREATE_DISPLAY_NAME_LABEL,
        display = escape_html(display_name),
        tz_label = i18n::JA_COMMUNITY_CREATE_TIMEZONE_LABEL,
        tz_name = i18n::JA_COMMUNITY_CREATE_TIMEZONE_JAPAN,
        submit = i18n::JA_COMMUNITY_CREATE_SUBMIT,
        cid = escape_html(&admin.community_id),
        cancel = i18n::JA_COMMUNITY_CREATE_CANCEL,
        nav = render::bottom_nav(&admin.community_id, "me"),
    );
    render::page(i18n::JA_COMMUNITY_CREATE_TITLE, &body)
}

fn community_name_error(err: CommunityNameError) -> &'static str {
    match err {
        CommunityNameError::Empty => i18n::JA_COMMUNITY_CREATE_NAME_ERROR,
        CommunityNameError::TooLong => i18n::JA_COMMUNITY_CREATE_NAME_TOO_LONG,
        CommunityNameError::InvalidCharacter => i18n::JA_COMMUNITY_CREATE_NAME_INVALID,
    }
}

fn display_name_error(err: DisplayNameError) -> &'static str {
    match err {
        DisplayNameError::Empty | DisplayNameError::TooLong | DisplayNameError::InvalidChars => {
            i18n::JA_COMMUNITY_CREATE_DISPLAY_NAME_ERROR
        }
    }
}

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}
