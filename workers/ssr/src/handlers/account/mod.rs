//! The account surface — RFC-080 §6 / RFC-081 §6 (Handoff 055, external-
//! identity Slice 5a; Handoff 057 Slice 5c adds the recovery credential
//! and unlink).
//!
//! GET /account — read-only, no application JavaScript (AD-1). The first
//! member route in this application that is not scoped to `/c/:cid/` —
//! reachable by any account-tier session (RFC-081 §2: never `Relink`, never
//! community-scoped) regardless of freshness or membership count. RFC-081
//! §6: a principal with no active membership reaches this page and nothing
//! else, discloses no community it does not belong to.
//!
//! Japanese-only (matching `handlers/identity/mod.rs`'s own RFC-072 Slice D
//! convention): the account tier has no single community-scoped
//! `ui_language` to resolve a locale from.
//!
//! Discloses nothing about identity internals (Handoff 055 §10): a linked
//! identity's row (`db::identity::LinkedIdentitySummary`) structurally
//! cannot carry a subject or digest, since the query that produces it never
//! selects those columns. The recovery credential's own existence is
//! disclosed (RFC-081 §3.1 requires this), never the code or its HMAC.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz;
use crate::db;
use crate::render::{self, escape_html};

pub mod link;
pub mod recovery;
pub mod unlink;

/// Shared by `get_account`, `recovery::post_regenerate`, and the
/// identity-link callback's first-credential reveal
/// (`handlers/identity/mod.rs::link_outcome`) — the one place this page's
/// full body is assembled, so a reveal is always rendered inside the same
/// page a member would otherwise land on, never a bespoke one-off page.
pub(crate) async fn render_account_page(
    env: &Env,
    user_id: &str,
    is_fresh: bool,
    reveal: Option<&str>,
) -> Result<Response> {
    let db = env.d1("DB")?;
    let identities = db::identity::list_active_for_user(&db, user_id).await?;
    // `render_account_page` is only ever reached through an already-scope-
    // checked path (an account-tier session via `require_account_surface`,
    // or the identity callback minting one) — every caller's session is
    // unscoped, so `None` is passed explicitly here rather than threading
    // a `scope_community_id` through, keeping that invariant visible.
    let communities = db::membership::list_communities_for_user(&db, user_id, None).await?;
    let has_recovery_credential = db::recovery::exists_for_user(&db, user_id).await?;
    let regenerate_token =
        crate::codlet::issue_token(env, user_id, token_purpose::REGENERATE_RECOVERY, None).await?;

    let body = render_body(
        &identities,
        &communities,
        is_fresh,
        has_recovery_credential,
        &regenerate_token,
        reveal,
    );
    let mut resp = render::page(i18n::JA_ACCOUNT_PAGE_TITLE, &body)?;
    if reveal.is_some() {
        // Handoff 057 §5.1 / §10: the plaintext code appears in this one
        // response only — same discipline as the admin invite-code reveal
        // this pattern is modeled on.
        resp.headers_mut()
            .set("Cache-Control", "no-store, private")?;
        resp.headers_mut().set("Referrer-Policy", "no-referrer")?;
    }
    Ok(resp)
}

pub async fn get_account(req: Request, env: &Env, rid: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    authz::require_account_surface(env, &auth, rid).await?;

    let freshness_window_start =
        db::subtract_seconds_from_now(authz::ACCOUNT_OPERATION_FRESHNESS_SECONDS);
    let is_fresh = authz::is_fresh_for_account_operations(&auth, &freshness_window_start);

    render_account_page(env, &auth.user_id, is_fresh, None).await
}

fn render_identities(identities: &[db::identity::LinkedIdentitySummary]) -> String {
    if identities.is_empty() {
        return format!(
            "<p class=\"cz-account-empty\">{}</p>",
            i18n::JA_ACCOUNT_NO_LINKED_IDENTITIES
        );
    }
    let items: String = identities
        .iter()
        .map(|identity| {
            format!(
                "<li class=\"cz-account-identity\">\
                   <span class=\"cz-account-identity-namespace\">{namespace}</span>\
                   <span class=\"cz-account-identity-linked-at\">{prefix}{linked_at}</span>\
                   <a href=\"/account/unlink/{id}\" class=\"cz-account-unlink-link\">{unlink}</a>\
                 </li>",
                namespace = escape_html(&identity.identity_namespace_id),
                prefix = i18n::JA_ACCOUNT_LINKED_AT_PREFIX,
                linked_at = escape_html(&identity.linked_at),
                id = escape_html(&identity.id),
                unlink = i18n::JA_ACCOUNT_UNLINK_LABEL,
            )
        })
        .collect();
    format!("<ul class=\"cz-account-identity-list\">{items}</ul>")
}

fn render_communities(communities: &[db::membership::CommunitySummary]) -> String {
    if communities.is_empty() {
        return format!(
            "<p class=\"cz-account-empty\">{}</p>",
            i18n::JA_ACCOUNT_NO_COMMUNITIES
        );
    }
    let items: String = communities
        .iter()
        .map(|community| {
            format!(
                "<li class=\"cz-account-community\">{name}</li>",
                name = escape_html(&community.community_name),
            )
        })
        .collect();
    format!("<ul class=\"cz-account-community-list\">{items}</ul>")
}

fn render_freshness(is_fresh: bool) -> String {
    if is_fresh {
        format!(
            "<p class=\"cz-account-freshness cz-account-freshness--fresh\">{}</p>",
            i18n::JA_ACCOUNT_FRESH_CAN_MANAGE
        )
    } else {
        format!(
            "<p class=\"cz-account-freshness cz-account-freshness--stale\">{msg} \
             <a href=\"/identity/start?action=sign_in\" class=\"cz-account-sign-in-again-link\">{link}</a></p>",
            msg = i18n::JA_ACCOUNT_STALE_SIGN_IN_AGAIN,
            link = i18n::JA_IDENTITY_SIGN_IN_LINK,
        )
    }
}

/// The one-time plaintext reveal, shown only in the response that just
/// generated or regenerated a code — same shape as
/// `handlers/admin/members.rs::invite_reveal_html`.
fn render_recovery_reveal(code: &str) -> String {
    format!(
        "<section id=\"recovery-code-reveal\" class=\"cz-account-reveal-box\">\
           <p class=\"cz-account-reveal-text\">{warning}</p>\
           <p class=\"cz-account-reveal-text\">{hint}</p>\
           <div class=\"cz-account-recovery-code-display\" \
             aria-label=\"{label}\">{code}</div>\
         </section>",
        warning = i18n::JA_ACCOUNT_RECOVERY_REVEAL_WARNING,
        hint = i18n::JA_ACCOUNT_RECOVERY_REVEAL_HINT,
        label = i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_HEADING,
        code = escape_html(code),
    )
}

fn render_recovery(
    has_recovery_credential: bool,
    regenerate_token: &str,
    reveal: Option<&str>,
) -> String {
    let status = if has_recovery_credential {
        i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_EXISTS
    } else {
        i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_NONE
    };
    let reveal_html = reveal.map(render_recovery_reveal).unwrap_or_default();
    format!(
        "<p class=\"cz-account-empty\">{status}</p>\
         {reveal_html}\
         <form method=\"post\" action=\"/account/recovery/regenerate\" class=\"cz-account-recovery-form\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           <button type=\"submit\" class=\"cz-account-recovery-regenerate-button\">{label}</button>\
         </form>",
        tok = escape_html(regenerate_token),
        label = i18n::JA_ACCOUNT_RECOVERY_REGENERATE_LABEL,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_body(
    identities: &[db::identity::LinkedIdentitySummary],
    communities: &[db::membership::CommunitySummary],
    is_fresh: bool,
    has_recovery_credential: bool,
    regenerate_token: &str,
    reveal: Option<&str>,
) -> String {
    format!(
        "<main class=\"cz-page-main cz-account-main\">\
           <h1 class=\"cz-account-title\">{title}</h1>\
           {freshness}\
           <section class=\"cz-account-section\">\
             <h2 class=\"cz-account-section-heading\">{identities_heading}</h2>\
             {identities}\
             <a href=\"/account/link\" class=\"cz-account-link-entry-link\">{link_entry}</a>\
           </section>\
           <section class=\"cz-account-section\">\
             <h2 class=\"cz-account-section-heading\">{recovery_heading}</h2>\
             {recovery}\
           </section>\
           <section class=\"cz-account-section\">\
             <h2 class=\"cz-account-section-heading\">{communities_heading}</h2>\
             {communities}\
           </section>\
           <a href=\"/\" class=\"cz-account-home-link\">{home}</a>\
         </main>",
        title = i18n::JA_ACCOUNT_PAGE_TITLE,
        freshness = render_freshness(is_fresh),
        identities_heading = i18n::JA_ACCOUNT_LINKED_IDENTITIES_HEADING,
        identities = render_identities(identities),
        link_entry = i18n::JA_ACCOUNT_LINK_ENTRY_LABEL,
        recovery_heading = i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_HEADING,
        recovery = render_recovery(has_recovery_credential, regenerate_token, reveal),
        communities_heading = i18n::JA_ACCOUNT_COMMUNITIES_HEADING,
        communities = render_communities(communities),
        home = i18n::JA_NAV_HOME,
    )
}

#[cfg(test)]
mod tests;
