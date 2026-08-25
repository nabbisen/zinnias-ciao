//! Linking an external identity — RFC-081 §4 (Handoff 056, external-
//! identity Slice 5b).
//!
//! GET  /account/link — confirmation page, no application JavaScript.
//! POST /account/link — consumes the confirmation token, starts the OIDC
//!                       round trip with `prompt=login` (RFC-080 §6 /
//!                       Handoff 056 §3.3 — the fresh provider
//!                       authentication is the step-up; freshness is not
//!                       otherwise required to reach this page, per
//!                       Handoff 055's review §5.3).
//!
//! Additive only: this module never writes an `UPDATE`/`DELETE` against
//! `user_identities` — see `release_gates.rs`'s
//! `no_unlink_path_exists_for_user_identities` for the structural proof
//! this depends on holding codebase-wide, not just here.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz;
use crate::form_token::ConsumeResult;
use crate::render::{self, escape_html};

pub async fn get_link(req: Request, env: &Env, rid: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    authz::require_account_surface(env, &auth, rid).await?;

    // RFC-084 §5: this file makes zero membership calls otherwise, so this
    // is the one new D1 query per route the RFC's decision 2 authorized.
    let db = env.d1("DB")?;
    let community_rows =
        crate::db::membership::list_communities_with_locale_for_user(&db, &auth.user_id).await?;
    let locale = authz::resolve_account_locale(
        &req,
        community_rows.iter().map(|row| row.ui_language.as_deref()),
    );

    let token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::LINK_IDENTITY, None).await?;
    let title = i18n::t(locale, i18n::ACCOUNT_LINK_TITLE);
    let body = format!(
        "<main class=\"cz-page-main cz-account-link-main\">\
           <h1 class=\"cz-account-link-title\">{title}</h1>\
           <p class=\"cz-account-link-body\">{body}</p>\
           <form method=\"post\" action=\"/account/link\" class=\"cz-account-link-form\">\
             <input type=\"hidden\" name=\"_token\" value=\"{token}\">\
             <button type=\"submit\" class=\"cz-account-link-submit\">{submit}</button>\
           </form>\
           <a href=\"/account\" class=\"cz-account-link-cancel\">{cancel}</a>\
         </main>",
        title = title,
        body = i18n::t(locale, i18n::ACCOUNT_LINK_BODY),
        token = escape_html(&token),
        submit = i18n::t(locale, i18n::ACCOUNT_LINK_SUBMIT),
        cancel = i18n::t(locale, i18n::ACCOUNT_LINK_CANCEL),
    );
    render::page_localized(locale, title, &body)
}

pub async fn post_link(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    authz::require_account_surface(env, &auth, rid).await?;

    let form = req.form_data().await?;
    let raw_token = form.get_field("_token").unwrap_or_default();
    let consumed = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::LINK_IDENTITY,
        &raw_token,
        None,
    )
    .await?;
    match consumed {
        // A replayed (already-consumed, or invalid) confirmation is not a
        // distinct failure mode worth its own page — land back on the
        // account page, same as an ordinary cancel.
        ConsumeResult::Replay(_) => return account_redirect(),
        ConsumeResult::Proceed => {}
    }

    // RFC-084 §5: this file's second, independent new query — `get_link`
    // and `post_link` are separate routes, so a request never pays both;
    // each pays exactly the one this decision authorized.
    let db = env.d1("DB")?;
    let community_rows =
        crate::db::membership::list_communities_with_locale_for_user(&db, &auth.user_id).await?;
    let locale = authz::resolve_account_locale(
        &req,
        community_rows.iter().map(|row| row.ui_language.as_deref()),
    );

    let origin = crate::handlers::identity::request_origin(&req)?;
    crate::handlers::identity::start_oidc_transaction(
        env,
        &origin,
        "link",
        None,
        Some(&auth.user_id),
        "/account",
        true,
        locale,
    )
    .await
}

fn account_redirect() -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", "/account")?;
    Ok(resp)
}
