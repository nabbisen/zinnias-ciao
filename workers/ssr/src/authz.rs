//! Every community-scoped route calls `require_membership` before acting.
//! A missing or removed membership returns the same generic not-found response
//! as a nonexistent resource, so private resource existence is never revealed.

use worker::{Env, Result};

use crate::db::membership as membership_db;
use crate::db::session::SessionProvenance;
use crate::session::AuthContext;
use zinnias_ciao_contracts::Locale;

/// RFC-080 §6 / Handoff 055 §5.2: account-level operations are rare and
/// deliberate — a member performing one has just decided to. Fifteen
/// minutes is long enough for a form to be filled and short enough that a
/// stolen cookie found later is not sufficient on its own.
pub(crate) const ACCOUNT_OPERATION_FRESHNESS_SECONDS: u64 = 900;

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

/// RFC-082 §4 / Handoff 058: the sentinel `require_membership` returns for
/// a present-but-suspended membership, parallel to `not_found()`. Caught by
/// `lib.rs::main`'s top-level dispatch and rendered as `render::suspended()`
/// — this keeps `require_membership`'s signature and every one of its call
/// sites (`let membership = require_membership(...).await?;`) unchanged.
pub(crate) fn suspended() -> worker::Error {
    worker::Error::RustError("Suspended.".to_string())
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

    let row = match membership_db::find_active(&db, &auth.user_id, community_id).await? {
        Some(row) => row,
        None => {
            // RFC-082 §4: a present-but-suspended membership gets the
            // explicit paused page, not the generic not-found a genuinely
            // absent or removed membership gets. `exists_present` is only
            // ever `true` here for a suspended row — `find_active` already
            // ruled out active.
            if membership_db::exists_present(&db, &auth.user_id, community_id).await? {
                return Err(suspended());
            }
            return Err(not_found()); // generic: no resource existence leak
        }
    };

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

/// RFC-080 §6 / Handoff 055 §5.4: is this session an **account-tier**
/// session at all, independent of freshness? Shared by
/// [`decide_account_surface_access`] (viewing `/account` needs this alone)
/// and [`is_fresh_for_account_operations`] (acting on it needs this *and*
/// freshness) — factored out so the two refusal conditions below are
/// asserted once, not duplicated into two call sites that could drift.
///
/// Both of the following must hold:
/// - provenance is present and is not `Relink` (RFC-081 §2 already refuses
///   a community-admin-derived session at the community tier; this is
///   where that refusal extends to the account tier);
/// - `scope_community_id` is `None` — a community-bound session is not an
///   account session. In this codebase every `Relink` session is also
///   scoped, so this alone already covers it in practice; the explicit
///   provenance check above is defence in depth against a hypothetical
///   future unscoped `Relink` session, not dead code.
fn is_account_tier_session(auth: &AuthContext) -> bool {
    match auth.provenance.as_deref() {
        None => return false,
        Some(provenance) if provenance == SessionProvenance::Relink.as_str() => return false,
        Some(_) => {}
    }
    auth.scope_community_id.is_none()
}

/// RFC-080 §6 / Handoff 055 §5.2: the account-tier step-up predicate — this
/// is where "may this session act on the account itself" gets decided,
/// strictly more powerful than "may this session reach this community."
/// Pure, in `decide_membership_scope`'s own shape: no D1, no wall-clock
/// read, natively unit-testable. `freshness_window_start` is `db::now_utc`
/// minus [`ACCOUNT_OPERATION_FRESHNESS_SECONDS`], computed by the caller
/// (e.g. `db::subtract_seconds_from_now(ACCOUNT_OPERATION_FRESHNESS_SECONDS)`)
/// — not "now" itself, so this function does no date arithmetic of its
/// own: `authenticated_at` and `freshness_window_start` are both
/// `db::now_utc`'s fixed `YYYY-MM-DDTHH:MM:SS.mmmZ` shape, which sorts
/// lexicographically in the same order it sorts chronologically, so
/// freshness is a plain string comparison.
///
/// All of the following must hold: [`is_account_tier_session`], and
/// `authenticated_at` is present and no earlier than
/// `freshness_window_start` — `None` is refused the same fail-closed way
/// `None` provenance already is, not treated as fresh.
pub(crate) fn is_fresh_for_account_operations(
    auth: &AuthContext,
    freshness_window_start: &str,
) -> bool {
    if !is_account_tier_session(auth) {
        return false;
    }
    match auth.authenticated_at.as_deref() {
        Some(authenticated_at) => authenticated_at >= freshness_window_start,
        None => false,
    }
}

/// RFC-081 §6 / Handoff 055 §5.4: the pure fail-closed decision behind
/// `require_account_surface` — same extraction reasoning as
/// `decide_membership_scope`. `/account` is reachable by any account-tier
/// session regardless of freshness or membership count (RFC-081 §6: a
/// principal with no active membership still reaches the account
/// surface); freshness only changes what the page *offers*, decided by
/// [`is_fresh_for_account_operations`] separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AccountSurfaceDecision<'a> {
    Allowed,
    RefusedNullProvenance,
    /// Covers both the `Relink`-provenance refusal and the
    /// out-of-tier-scope refusal — in this codebase's current shape they
    /// are the same live case (every `Relink` session is scoped), so one
    /// audited reason serves both. `granting_community_id` is `None` only
    /// for the hypothetical unscoped-`Relink` case, which has no
    /// community to name — not audited, matching
    /// `RefusedNullProvenance`'s own "no producer to catch in the act"
    /// reasoning.
    RefusedIneligible {
        granting_community_id: Option<&'a str>,
    },
}

pub(crate) fn decide_account_surface_access(auth: &AuthContext) -> AccountSurfaceDecision<'_> {
    match auth.provenance.as_deref() {
        None => return AccountSurfaceDecision::RefusedNullProvenance,
        Some(provenance) if provenance == SessionProvenance::Relink.as_str() => {
            return AccountSurfaceDecision::RefusedIneligible {
                granting_community_id: auth.scope_community_id.as_deref(),
            };
        }
        Some(_) => {}
    }
    if let Some(scope) = auth.scope_community_id.as_deref() {
        return AccountSurfaceDecision::RefusedIneligible {
            granting_community_id: Some(scope),
        };
    }
    AccountSurfaceDecision::Allowed
}

/// Verify the authenticated session may reach the account surface at all
/// (RFC-081 §6, Handoff 055 §5.4). Returns `Err(not_found)` (generic,
/// RFC-004 shape) if refused — indistinguishable from any other not-found
/// response, same discipline as `require_membership`.
pub async fn require_account_surface(
    env: &Env,
    auth: &AuthContext,
    request_id: &str,
) -> Result<()> {
    match decide_account_surface_access(auth) {
        AccountSurfaceDecision::Allowed => Ok(()),
        AccountSurfaceDecision::RefusedNullProvenance => Err(not_found()),
        AccountSurfaceDecision::RefusedIneligible {
            granting_community_id: None,
        } => Err(not_found()),
        AccountSurfaceDecision::RefusedIneligible {
            granting_community_id: Some(scope),
        } => {
            let db = env.d1("DB")?;
            crate::audit::write_session_scope_refused(&db, request_id, scope, None).await;
            Err(not_found())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth(provenance: Option<&str>, scope_community_id: Option<&str>) -> AuthContext {
        auth_with_freshness(provenance, scope_community_id, None)
    }

    fn auth_with_freshness(
        provenance: Option<&str>,
        scope_community_id: Option<&str>,
        authenticated_at: Option<&str>,
    ) -> AuthContext {
        AuthContext {
            session_id: "sess_test".to_owned(),
            user_id: "usr_test".to_owned(),
            provenance: provenance.map(str::to_owned),
            scope_community_id: scope_community_id.map(str::to_owned),
            authenticated_at: authenticated_at.map(str::to_owned),
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

    // ── decide_account_surface_access (§5.4) ─────────────────────────────

    #[test]
    fn account_surface_null_provenance_is_refused() {
        assert_eq!(
            decide_account_surface_access(&auth(None, None)),
            AccountSurfaceDecision::RefusedNullProvenance
        );
    }

    #[test]
    fn account_surface_allows_unscoped_first_class_and_external_identity_sessions() {
        for provenance in ["invite_redemption", "external_identity"] {
            assert_eq!(
                decide_account_surface_access(&auth(Some(provenance), None)),
                AccountSurfaceDecision::Allowed,
                "provenance={provenance}"
            );
        }
    }

    #[test]
    fn account_surface_allows_a_principal_with_zero_memberships() {
        // RFC-081 §6: the account surface's whole point is to remain
        // reachable with no active membership — this decision has no
        // knowledge of membership count at all, which is exactly what
        // makes that true; membership count is irrelevant input here.
        assert_eq!(
            decide_account_surface_access(&auth(Some("invite_redemption"), None)),
            AccountSurfaceDecision::Allowed
        );
    }

    #[test]
    fn account_surface_refuses_relink_provenance_with_the_granting_community_audited() {
        assert_eq!(
            decide_account_surface_access(&auth(Some("relink"), Some("com_a"))),
            AccountSurfaceDecision::RefusedIneligible {
                granting_community_id: Some("com_a")
            },
            "the account surface must be refused to Relink sessions entirely, \
             carrying the granting community for the audit write"
        );
    }

    #[test]
    fn account_surface_refuses_any_scoped_session_even_with_eligible_provenance() {
        // Defensive: today only Relink produces a scoped session, but the
        // refusal is keyed on scope, not provenance name, so a
        // hypothetical future scoped non-Relink provenance is refused too.
        assert_eq!(
            decide_account_surface_access(&auth(Some("invite_redemption"), Some("com_a"))),
            AccountSurfaceDecision::RefusedIneligible {
                granting_community_id: Some("com_a")
            }
        );
    }

    // ── is_fresh_for_account_operations (§5.2) — exhaustive ─────────────

    const WINDOW_START: &str = "2026-08-11T00:00:00.000Z";
    const WITHIN_WINDOW: &str = "2026-08-11T00:10:00.000Z"; // 10 min after start
    const BEFORE_WINDOW: &str = "2026-08-10T23:50:00.000Z"; // 10 min before start

    #[test]
    fn null_provenance_is_refused_even_when_otherwise_fresh() {
        assert!(!is_fresh_for_account_operations(
            &auth_with_freshness(None, None, Some(WITHIN_WINDOW)),
            WINDOW_START,
        ));
    }

    #[test]
    fn relink_provenance_is_refused_even_when_unscoped_and_fresh() {
        // Relink sessions in this codebase are always scoped in practice,
        // but the predicate must refuse Relink on its own, independent of
        // scope — RFC-081 §2's community-admin-authority boundary applies
        // to the account tier even in a hypothetical unscoped shape.
        assert!(!is_fresh_for_account_operations(
            &auth_with_freshness(Some("relink"), None, Some(WITHIN_WINDOW)),
            WINDOW_START,
        ));
        assert!(!is_fresh_for_account_operations(
            &auth_with_freshness(Some("relink"), Some("com_a"), Some(WITHIN_WINDOW)),
            WINDOW_START,
        ));
    }

    #[test]
    fn invite_redemption_and_external_identity_are_both_eligible_provenances() {
        for provenance in ["invite_redemption", "external_identity"] {
            assert!(
                is_fresh_for_account_operations(
                    &auth_with_freshness(Some(provenance), None, Some(WITHIN_WINDOW)),
                    WINDOW_START,
                ),
                "provenance={provenance} must be eligible when unscoped and fresh"
            );
        }
    }

    #[test]
    fn account_recovery_provenance_is_account_tier_and_eligible_when_fresh() {
        // Handoff 057 §5.2: pinned rather than assumed — the session a
        // recovery-credential consumption mints must be account-tier and
        // fresh by construction (`is_account_tier_session` excludes only
        // `Relink`), not merely by omission of a check somewhere else.
        assert!(is_fresh_for_account_operations(
            &auth_with_freshness(Some("account_recovery"), None, Some(WITHIN_WINDOW)),
            WINDOW_START,
        ));
        assert_eq!(
            decide_account_surface_access(&auth(Some("account_recovery"), None)),
            AccountSurfaceDecision::Allowed
        );
    }

    #[test]
    fn scoped_session_is_refused_even_with_eligible_provenance_and_fresh_authentication() {
        assert!(!is_fresh_for_account_operations(
            &auth_with_freshness(
                Some("invite_redemption"),
                Some("com_a"),
                Some(WITHIN_WINDOW)
            ),
            WINDOW_START,
        ));
        assert!(!is_fresh_for_account_operations(
            &auth_with_freshness(
                Some("external_identity"),
                Some("com_a"),
                Some(WITHIN_WINDOW)
            ),
            WINDOW_START,
        ));
    }

    #[test]
    fn null_authenticated_at_is_refused_not_treated_as_fresh() {
        assert!(!is_fresh_for_account_operations(
            &auth_with_freshness(Some("invite_redemption"), None, None),
            WINDOW_START,
        ));
    }

    #[test]
    fn authenticated_at_within_the_window_is_fresh() {
        assert!(is_fresh_for_account_operations(
            &auth_with_freshness(Some("invite_redemption"), None, Some(WITHIN_WINDOW)),
            WINDOW_START,
        ));
    }

    #[test]
    fn authenticated_at_before_the_window_is_stale() {
        assert!(!is_fresh_for_account_operations(
            &auth_with_freshness(Some("invite_redemption"), None, Some(BEFORE_WINDOW)),
            WINDOW_START,
        ));
    }

    #[test]
    fn authenticated_at_exactly_at_the_window_boundary_is_fresh() {
        // Inclusive boundary: a session authenticated exactly
        // ACCOUNT_OPERATION_FRESHNESS_SECONDS ago is still fresh, not yet
        // stale — `>=`, not `>`.
        assert!(is_fresh_for_account_operations(
            &auth_with_freshness(Some("invite_redemption"), None, Some(WINDOW_START)),
            WINDOW_START,
        ));
    }

    #[test]
    fn authenticated_at_one_millisecond_before_the_boundary_is_stale() {
        assert!(!is_fresh_for_account_operations(
            &auth_with_freshness(
                Some("invite_redemption"),
                None,
                Some("2026-08-10T23:59:59.999Z")
            ),
            WINDOW_START,
        ));
    }

    #[test]
    fn every_provenance_and_scope_and_freshness_combination_is_covered() {
        // A brute-force cross-product, independent of the targeted tests
        // above: only (eligible provenance, unscoped, authenticated_at
        // within the window) may pass.
        let provenances: [Option<&str>; 5] = [
            None,
            Some("invite_redemption"),
            Some("relink"),
            Some("external_identity"),
            Some("account_recovery"),
        ];
        let scopes: [Option<&str>; 2] = [None, Some("com_a")];
        let freshness: [Option<&str>; 3] = [None, Some(BEFORE_WINDOW), Some(WITHIN_WINDOW)];

        for provenance in provenances {
            for scope in scopes {
                for authenticated_at in freshness {
                    let expected = provenance.is_some_and(|p| p != "relink")
                        && scope.is_none()
                        && authenticated_at == Some(WITHIN_WINDOW);
                    assert_eq!(
                        is_fresh_for_account_operations(
                            &auth_with_freshness(provenance, scope, authenticated_at),
                            WINDOW_START,
                        ),
                        expected,
                        "provenance={provenance:?} scope={scope:?} authenticated_at={authenticated_at:?}"
                    );
                }
            }
        }
    }
}
