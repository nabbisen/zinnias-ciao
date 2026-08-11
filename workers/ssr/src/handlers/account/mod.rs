//! The account surface — RFC-080 §6 / RFC-081 §6 (Handoff 055, external-
//! identity Slice 5a).
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
//! selects those columns.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::i18n;

use crate::authz;
use crate::db;
use crate::render::{self, escape_html};

pub async fn get_account(req: Request, env: &Env, rid: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    authz::require_account_surface(env, &auth, rid).await?;

    let db = env.d1("DB")?;
    let identities = db::identity::list_active_for_user(&db, &auth.user_id).await?;
    // Every session that reaches this line is unscoped (enforced by
    // `require_account_surface` above) — `scope_community_id` is always
    // `None` here, passed explicitly rather than read from `auth` again to
    // keep that invariant visible at the call site.
    let communities = db::membership::list_communities_for_user(&db, &auth.user_id, None).await?;

    let freshness_window_start =
        db::subtract_seconds_from_now(authz::ACCOUNT_OPERATION_FRESHNESS_SECONDS);
    let is_fresh = authz::is_fresh_for_account_operations(&auth, &freshness_window_start);

    let body = render_body(&identities, &communities, is_fresh);
    render::page(i18n::JA_ACCOUNT_PAGE_TITLE, &body)
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
                 </li>",
                namespace = escape_html(&identity.identity_namespace_id),
                prefix = i18n::JA_ACCOUNT_LINKED_AT_PREFIX,
                linked_at = escape_html(&identity.linked_at),
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

fn render_body(
    identities: &[db::identity::LinkedIdentitySummary],
    communities: &[db::membership::CommunitySummary],
    is_fresh: bool,
) -> String {
    format!(
        "<main class=\"cz-page-main cz-account-main\">\
           <h1 class=\"cz-account-title\">{title}</h1>\
           {freshness}\
           <section class=\"cz-account-section\">\
             <h2 class=\"cz-account-section-heading\">{identities_heading}</h2>\
             {identities}\
           </section>\
           <section class=\"cz-account-section\">\
             <h2 class=\"cz-account-section-heading\">{recovery_heading}</h2>\
             <p class=\"cz-account-empty\">{recovery_none}</p>\
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
        recovery_heading = i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_HEADING,
        recovery_none = i18n::JA_ACCOUNT_RECOVERY_CREDENTIAL_NONE,
        communities_heading = i18n::JA_ACCOUNT_COMMUNITIES_HEADING,
        communities = render_communities(communities),
        home = i18n::JA_NAV_HOME,
    )
}

#[cfg(test)]
mod tests;
