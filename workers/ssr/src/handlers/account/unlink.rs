//! Unlinking an external identity — RFC-081 §3.3 (Handoff 057 §5.3).
//!
//! GET  /account/unlink/:id — confirmation page, no application JavaScript.
//! POST /account/unlink/:id — consumes the confirmation token, performs
//!                             the unlink.
//!
//! The one legitimate exception to `no_unlink_path_exists_for_user_identities`
//! — see `db/identity.rs::unlink_required`'s own doc comment for how the
//! concurrency requirement (RFC-081 §3.3) is actually met.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::Locale;
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz;
use crate::db::identity as identity_db;
use crate::form_token::ConsumeResult;
use crate::render::{self, escape_html};

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}

fn account_redirect() -> Result<Response> {
    redirect("/account")
}

fn reauthenticate_redirect() -> Result<Response> {
    // Handoff 057 §5.4: unreachable from a stale session — rather than a
    // dead-end refusal, this sends the member down the same re-
    // authentication path the account page's own freshness banner offers,
    // so there is a working way forward, not just a wall.
    redirect("/identity/start?action=sign_in")
}

pub async fn get_unlink(req: Request, env: &Env, rid: &str, identity_id: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    authz::require_account_surface(env, &auth, rid).await?;

    let freshness_window_start =
        crate::db::subtract_seconds_from_now(authz::ACCOUNT_OPERATION_FRESHNESS_SECONDS);
    if !authz::is_fresh_for_account_operations(&auth, &freshness_window_start) {
        return reauthenticate_redirect();
    }

    // RFC-084 §5: this file makes zero membership calls otherwise, so this
    // is the one new D1 query per route the RFC's decision 2 authorized.
    let db = env.d1("DB")?;
    let community_rows =
        crate::db::membership::list_communities_with_locale_for_user(&db, &auth.user_id).await?;
    let locale = authz::resolve_account_locale(
        &req,
        community_rows.iter().map(|row| row.ui_language.as_deref()),
    );

    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::UNLINK_IDENTITY,
        Some(identity_id),
    )
    .await?;
    render_confirm(&token, identity_id, None, locale)
}

pub async fn post_unlink(
    mut req: Request,
    env: &Env,
    rid: &str,
    identity_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    authz::require_account_surface(env, &auth, rid).await?;

    let freshness_window_start =
        crate::db::subtract_seconds_from_now(authz::ACCOUNT_OPERATION_FRESHNESS_SECONDS);
    if !authz::is_fresh_for_account_operations(&auth, &freshness_window_start) {
        return reauthenticate_redirect();
    }

    // RFC-084 §5: this file's second, independent new query — `get_unlink`
    // and `post_unlink` are separate routes, so a request never pays both;
    // each pays exactly the one this decision authorized. Resolved before
    // body parsing so it is available on every response path below.
    let db = env.d1("DB")?;
    let community_rows =
        crate::db::membership::list_communities_with_locale_for_user(&db, &auth.user_id).await?;
    let locale = authz::resolve_account_locale(
        &req,
        community_rows.iter().map(|row| row.ui_language.as_deref()),
    );

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let consumed = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::UNLINK_IDENTITY,
        &raw_token,
        Some(identity_id),
    )
    .await?;
    match consumed {
        ConsumeResult::Replay(_) => return account_redirect(),
        ConsumeResult::Proceed => {}
    }

    let unlinked =
        identity_db::unlink_required(&db, rid, &auth.user_id, identity_id, &auth.session_id)
            .await?;
    if unlinked {
        return account_redirect();
    }

    // Handoff 057 §5.4 / §9: refused generically — no distinct message
    // for "no other method" vs. any other reason.
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::UNLINK_IDENTITY,
        Some(identity_id),
    )
    .await?;
    render_confirm(
        &token,
        identity_id,
        Some(i18n::t(locale, i18n::ACCOUNT_UNLINK_REFUSED)),
        locale,
    )
}

fn render_confirm(
    token: &str,
    identity_id: &str,
    error: Option<&str>,
    locale: Locale,
) -> Result<Response> {
    let error_html = error
        .map(|e| {
            format!(
                "<p role=\"alert\" class=\"cz-account-unlink-error-text\">{}</p>",
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let title = i18n::t(locale, i18n::ACCOUNT_UNLINK_TITLE);
    let body = format!(
        "<main class=\"cz-page-main cz-account-unlink-main\">\
           <h1 class=\"cz-account-unlink-title\">{title}</h1>\
           <p class=\"cz-account-unlink-body\">{body}</p>\
           {error_html}\
           <form method=\"post\" action=\"/account/unlink/{id}\" class=\"cz-account-unlink-form\">\
             <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
             <button type=\"submit\" class=\"cz-account-unlink-submit\">{submit}</button>\
           </form>\
           <a href=\"/account\" class=\"cz-account-unlink-cancel\">{cancel}</a>\
         </main>",
        title = title,
        body = i18n::t(locale, i18n::ACCOUNT_UNLINK_BODY),
        error_html = error_html,
        id = escape_html(identity_id),
        token = escape_html(token),
        submit = i18n::t(locale, i18n::ACCOUNT_UNLINK_SUBMIT),
        cancel = i18n::t(locale, i18n::ACCOUNT_UNLINK_CANCEL),
    );
    render::page_localized(locale, title, &body)
}
