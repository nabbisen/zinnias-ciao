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
use zinnias_ciao_contracts::Locale;
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz;
use crate::form_token::ConsumeResult;
use crate::render::{self, escape_html};

pub async fn get_link(req: Request, env: &Env, rid: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    authz::require_account_surface(env, &auth, rid).await?;

    let token =
        crate::codlet::issue_token(env, &auth.user_id, token_purpose::LINK_IDENTITY, None).await?;
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
        title = i18n::JA_ACCOUNT_LINK_TITLE,
        body = i18n::JA_ACCOUNT_LINK_BODY,
        token = escape_html(&token),
        submit = i18n::JA_ACCOUNT_LINK_SUBMIT,
        cancel = i18n::JA_ACCOUNT_LINK_CANCEL,
    );
    render::page(i18n::JA_ACCOUNT_LINK_TITLE, &body)
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

    // RFC-083 D2b (deferred to its own RFC, not this package): the account
    // tier has a session but no single community-scoped ui_language to
    // resolve a locale from — same reasoning as this file's own
    // LOCALIZATION_EXCEPTIONS entry. The literal `Locale::Ja` below matches
    // this call site's unchanged current behaviour exactly; it is not a
    // new resolution decision.
    let origin = crate::handlers::identity::request_origin(&req)?;
    crate::handlers::identity::start_oidc_transaction(
        env,
        &origin,
        "link",
        None,
        Some(&auth.user_id),
        "/account",
        true,
        Locale::Ja,
    )
    .await
}

fn account_redirect() -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", "/account")?;
    Ok(resp)
}
