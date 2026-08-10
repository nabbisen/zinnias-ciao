//! Every community-scoped route calls `require_membership` before acting.
//! A missing or removed membership returns the same generic not-found response
//! as a nonexistent resource, so private resource existence is never revealed.

use worker::{Env, Result};

use crate::db::membership as membership_db;
use crate::session::AuthContext;
use zinnias_ciao_contracts::Locale;

pub struct MembershipContext {
    pub membership_id: String,
    pub community_id: String,
    /// Populated for completeness (RFC-004 context object), but not read —
    /// most handlers address the community via the validated URL parameter
    /// directly. Handoff 044 audit verdict: `deliberate`.
    #[allow(dead_code)]
    pub user_id: String,
    pub role: String,
    pub display_name: String,
    /// Resolved by `db::membership::find_active` (RFC-072), the only
    /// query that reads `ui_language` — never re-derived downstream.
    pub locale: Locale,
}

impl MembershipContext {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

fn not_found() -> worker::Error {
    worker::Error::RustError("Not found.".to_string())
}

/// RFC-081 §2 / §2.1a (Handoff 048): the pure fail-closed decision behind
/// `require_membership`, extracted so it is natively unit-testable —
/// `require_membership` itself needs a real D1 handle and cannot run
/// outside the wasm/worker harness (same reasoning as
/// `abuse_control::classify_ingress` vs. `canonical_client_network`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeDecision<'a> {
    Allowed,
    RefusedNullProvenance,
    RefusedOutOfScope { granting_community_id: &'a str },
}

/// RFC-081 §2 (Handoff 049): the provenance branch shared by
/// `decide_membership_scope` and `handlers/community.rs::get_switch`.
/// `get_switch` has no single target community to check scope against — it
/// *chooses* one from an already scope-filtered list — but a `NULL`
/// provenance session must still be refused before that choice is made,
/// since scope-filtering answers "which communities may this session see,"
/// not "is this session valid at all." Factored out so that refusal isn't
/// duplicated as a second, driftable copy of the same check.
pub(crate) fn has_provenance(auth: &AuthContext) -> bool {
    auth.provenance.is_some()
}

pub(crate) fn decide_membership_scope<'a>(
    auth: &'a AuthContext,
    requested_community_id: &str,
) -> ScopeDecision<'a> {
    if !has_provenance(auth) {
        return ScopeDecision::RefusedNullProvenance;
    }
    if let Some(scope) = auth.scope_community_id.as_deref()
        && scope != requested_community_id
    {
        return ScopeDecision::RefusedOutOfScope {
            granting_community_id: scope,
        };
    }
    ScopeDecision::Allowed
}

/// Verify the authenticated user has an active membership in `community_id`.
/// Returns `Err(not_found)` (generic, RFC-004) if absent or removed.
///
/// RFC-081 §2 / Handoff 048: also enforces session provenance and
/// community binding, fail-closed, before any membership lookup. Both
/// refusals below are indistinguishable from an ordinary missing
/// membership — the caller never learns whether the session exists, is
/// scoped, or to what.
pub async fn require_membership(
    env: &Env,
    auth: &AuthContext,
    community_id: &str,
    request_id: &str,
) -> Result<MembershipContext> {
    let db = env.d1("DB")?;

    match decide_membership_scope(auth, community_id) {
        ScopeDecision::Allowed => {}
        // No session should reach this after migration 0012 revoked every
        // pre-existing row — an assertion in behaviour, not a live branch.
        // Not audited: there is no producer of a NULL-provenance session
        // to catch in the act, so an audit row here would only ever be
        // dead evidence.
        ScopeDecision::RefusedNullProvenance => return Err(not_found()),
        ScopeDecision::RefusedOutOfScope {
            granting_community_id,
        } => {
            crate::audit::write_session_scope_refused(
                &db,
                request_id,
                granting_community_id,
                Some(community_id),
            )
            .await;
            return Err(not_found());
        }
    }

    let row = membership_db::find_active(&db, &auth.user_id, community_id)
        .await?
        .ok_or_else(not_found)?; // generic: no resource existence leak

    Ok(MembershipContext {
        membership_id: row.id,
        community_id: row.community_id,
        user_id: row.user_id,
        role: row.role,
        display_name: row.display_name,
        locale: row.locale,
    })
}

/// Like `require_membership`, but also checks that the user is an admin.
pub async fn require_admin(
    env: &Env,
    auth: &AuthContext,
    community_id: &str,
    request_id: &str,
) -> Result<MembershipContext> {
    let ctx = require_membership(env, auth, community_id, request_id).await?;
    if !ctx.is_admin() {
        return Err(not_found()); // same response as not-found
    }
    Ok(ctx)
}

/// RFC-081 §2.1a (Handoff 048): the pure fail-closed decision behind
/// `require_active_admin_somewhere` — see `decide_membership_scope` for why
/// this is extracted. There is no target community here, so the only
/// question is whether the session is bound at all: a bound session is
/// refused unconditionally, since this function's whole point is to work
/// without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnscopedAdminDecision<'a> {
    Allowed,
    RefusedNullProvenance,
    RefusedBound { granting_community_id: &'a str },
}

pub(crate) fn decide_unscoped_admin_access(auth: &AuthContext) -> UnscopedAdminDecision<'_> {
    if auth.provenance.is_none() {
        return UnscopedAdminDecision::RefusedNullProvenance;
    }
    if let Some(scope) = auth.scope_community_id.as_deref() {
        return UnscopedAdminDecision::RefusedBound {
            granting_community_id: scope,
        };
    }
    UnscopedAdminDecision::Allowed
}

/// Require that the user is an active admin in at least one community.
/// This supports guarded bootstrap flows that are not scoped to an existing
/// community URL, without granting access to anonymous or member-only users.
/// `/communities/new` (Handoff 030 §7.2) resolves its locale from this same
/// admin membership — the one that authorized access to the page — rather
/// than from a `:cid` it does not have.
///
/// RFC-081 §2.1a / Handoff 048: a community-bound session may **never** use
/// this path — there is no target community here to bind it to, and
/// `/communities/new` in particular would let a bound session mint a
/// brand-new community it then admins, exactly the escape this package
/// closes. Refused fail-closed, before the lookup, same as
/// `require_membership`.
pub async fn require_active_admin_somewhere(
    env: &Env,
    auth: &AuthContext,
    request_id: &str,
) -> Result<MembershipContext> {
    let db = env.d1("DB")?;

    match decide_unscoped_admin_access(auth) {
        UnscopedAdminDecision::Allowed => {}
        UnscopedAdminDecision::RefusedNullProvenance => return Err(not_found()),
        UnscopedAdminDecision::RefusedBound {
            granting_community_id,
        } => {
            crate::audit::write_session_scope_refused(&db, request_id, granting_community_id, None)
                .await;
            return Err(not_found());
        }
    }

    let row = membership_db::find_first_admin_for_user(&db, &auth.user_id)
        .await?
        .ok_or_else(not_found)?;

    Ok(MembershipContext {
        membership_id: row.id,
        community_id: row.community_id,
        user_id: row.user_id,
        role: row.role,
        display_name: row.display_name,
        locale: row.locale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth(provenance: Option<&str>, scope_community_id: Option<&str>) -> AuthContext {
        AuthContext {
            session_id: "sess_test".to_owned(),
            user_id: "usr_test".to_owned(),
            provenance: provenance.map(str::to_owned),
            scope_community_id: scope_community_id.map(str::to_owned),
        }
    }

    // ── require_membership's decision (§7.3) ────────────────────────────

    #[test]
    fn null_provenance_is_refused_regardless_of_scope() {
        assert_eq!(
            decide_membership_scope(&auth(None, None), "com_a"),
            ScopeDecision::RefusedNullProvenance
        );
        assert_eq!(
            decide_membership_scope(&auth(None, Some("com_a")), "com_a"),
            ScopeDecision::RefusedNullProvenance,
            "a NULL-provenance session must be refused even when the (meaningless) \
             scope happens to match the requested community"
        );
    }

    #[test]
    fn first_class_session_reaches_any_community() {
        // provenance = 'invite_redemption', scope_community_id = NULL.
        assert_eq!(
            decide_membership_scope(&auth(Some("invite_redemption"), None), "com_a"),
            ScopeDecision::Allowed
        );
        assert_eq!(
            decide_membership_scope(&auth(Some("invite_redemption"), None), "com_b"),
            ScopeDecision::Allowed,
            "an unscoped session is not restricted to any particular community"
        );
    }

    #[test]
    fn bound_session_reaches_only_its_granting_community() {
        let a = auth(Some("relink"), Some("com_a"));
        assert_eq!(decide_membership_scope(&a, "com_a"), ScopeDecision::Allowed);
        assert_eq!(
            decide_membership_scope(&a, "com_b"),
            ScopeDecision::RefusedOutOfScope {
                granting_community_id: "com_a"
            },
            "the exact gap this package closes: a relink-derived session must not \
             reach a second community the same user_id happens to belong to"
        );
    }

    // ── require_active_admin_somewhere's decision (§7.4) ────────────────

    #[test]
    fn unscoped_admin_access_null_provenance_is_refused() {
        assert_eq!(
            decide_unscoped_admin_access(&auth(None, None)),
            UnscopedAdminDecision::RefusedNullProvenance
        );
    }

    #[test]
    fn unscoped_admin_access_allows_only_first_class_sessions() {
        assert_eq!(
            decide_unscoped_admin_access(&auth(Some("invite_redemption"), None)),
            UnscopedAdminDecision::Allowed
        );
    }

    #[test]
    fn unscoped_admin_access_refuses_every_bound_session_regardless_of_which_community() {
        // No target community exists here to compare against — a bound
        // session is refused unconditionally, not compared to anything.
        assert_eq!(
            decide_unscoped_admin_access(&auth(Some("relink"), Some("com_a"))),
            UnscopedAdminDecision::RefusedBound {
                granting_community_id: "com_a"
            },
            "RFC-081 §2.1a: /communities/new must not let a relink-bound session \
             mint a brand-new community it then admins"
        );
    }
}
