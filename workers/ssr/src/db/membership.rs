//! Membership and user table access — RFC-002 / RFC-004.
//!
//! RFC-082 / Handoff 058: two named predicates, defined once, are the
//! structural answer to the fail-open risk of a suspension column added
//! without touching every site that decides "is this membership active."
//! No query anywhere in this codebase spells either condition inline —
//! `packages/contracts/tests/release_gates.rs` enforces that with a
//! default-fail, comment-stripped scan.

use crate::audit::{self, AuditAction, AuditMetadata};
use worker::{D1Database, Result};
use zinnias_ciao_contracts::Locale;

/// Authorization. The fail-closed default: if a site's intent is unclear,
/// it takes this one. Excludes both terminal removal and reversible
/// suspension.
pub(crate) const MEMBERSHIP_ACTIVE: &str = "removed_at IS NULL AND suspended_at IS NULL";

/// Presence. For listing a member, and for targeting an admin action at a
/// suspended one — an unsuspend cannot find its own target otherwise.
/// Excludes only terminal removal; a suspended row is present.
pub(crate) const MEMBERSHIP_PRESENT: &str = "removed_at IS NULL";

pub struct MembershipRow {
    pub id: String,
    pub community_id: String,
    pub role: String,
}

/// An active membership row that also carries a *resolved* locale
/// (RFC-072). Only [`find_active`] produces this type — the row shape
/// itself, not caller discipline, is what keeps a localized render path
/// from ever taking a locale from `find_active_by_id` or
/// `list_active_for_user`: those return the plain [`MembershipRow`], which
/// has no locale field to reach for. (Slice A review, Observation O2.)
pub struct ActiveMembershipRow {
    pub id: String,
    pub community_id: String,
    pub user_id: String,
    pub role: String,
    pub display_name: String,
    pub locale: Locale,
}

/// An admin membership row that also carries a *resolved* locale (RFC-072,
/// Handoff 030 §7.2). Only [`find_first_admin_for_user`] produces this type
/// — same row-shape discipline as [`ActiveMembershipRow`]. Used by
/// `/communities/new`, which has no `:cid` and therefore no "current
/// membership" to read a locale from; the corrected rule resolves from the
/// admin membership that authorized access to the page, not an arbitrary
/// "earliest-joined membership of any role."
pub struct AdminMembershipRow {
    pub id: String,
    pub community_id: String,
    pub user_id: String,
    pub role: String,
    pub display_name: String,
    pub locale: Locale,
}

/// Resolves the raw stored `ui_language` column (RFC-072/RFC-085 §3.2):
/// three distinguishable inputs, three distinguishable answers, never one
/// value silently doing both jobs.
///
/// - `Some(valid)` — the member's own expressed preference.
/// - `None` (SQL `NULL`) — **no preference expressed**: the *product*
///   default, [`Locale::PRODUCT_DEFAULT`] — the answer that moves when
///   ROADMAP.md's English-default decision is taken.
/// - `Some(other)` outside `0011`'s `CHECK` allow-list — **corrupt**, only
///   reachable via manual repair: the *safety* fallback,
///   [`Locale::FAIL_CLOSED`] — this must never move as a side effect of the
///   product default changing (RFC-085 §5). Never panics: a bad stored
///   value reaching a render path would be an SEC-5 violation.
fn resolve_locale(stored: Option<&str>) -> Locale {
    match stored {
        None => Locale::PRODUCT_DEFAULT,
        Some(value) => Locale::parse(value).unwrap_or(Locale::FAIL_CLOSED),
    }
}

/// Find an active membership for the given user + community. This is the
/// query every localized page's membership lookup already performs; RFC-072
/// reads `ui_language` from this same row rather than adding a second query,
/// and resolves it here into [`ActiveMembershipRow::locale`] — the only
/// trustworthy source of a page's locale.
/// Returns `None` if absent, removed, or suspended (`MEMBERSHIP_ACTIVE`) —
/// this is the front door `authz::require_membership` calls on every
/// community-scoped request.
pub async fn find_active(
    db: &D1Database,
    user_id: &str,
    community_id: &str,
) -> Result<Option<ActiveMembershipRow>> {
    let row = db
        .prepare(format!(
            "SELECT id, community_id, user_id, role, display_name, ui_language \
             FROM community_memberships \
             WHERE user_id = ?1 AND community_id = ?2 AND {MEMBERSHIP_ACTIVE} \
             LIMIT 1"
        ))
        .bind(&[user_id.into(), community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        let ui_language = v
            .get("ui_language")
            .and_then(|value| value.as_str())
            .map(str::to_owned);
        Some(ActiveMembershipRow {
            id: v.get("id")?.as_str()?.to_owned(),
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            user_id: v.get("user_id")?.as_str()?.to_owned(),
            role: v.get("role")?.as_str()?.to_owned(),
            display_name: v.get("display_name")?.as_str()?.to_owned(),
            locale: resolve_locale(ui_language.as_deref()),
        })
    }))
}

#[cfg(test)]
mod tests {
    use super::resolve_locale;
    use zinnias_ciao_contracts::Locale;

    #[test]
    fn resolve_locale_uses_the_stored_preference_when_valid() {
        assert_eq!(resolve_locale(Some("ja")), Locale::Ja);
        assert_eq!(resolve_locale(Some("en")), Locale::En);
    }

    #[test]
    fn resolve_locale_falls_back_to_the_product_default_when_absent() {
        // RFC-085 §3.2: NULL means no preference expressed — this is the
        // *product* question, distinct from the fail-closed one below.
        assert_eq!(resolve_locale(None), Locale::PRODUCT_DEFAULT);
    }

    #[test]
    fn resolve_locale_falls_back_to_fail_closed_for_an_out_of_allow_list_value_without_panicking() {
        // A value the CHECK constraint should have rejected on write, but
        // that a defensive read path must still survive (e.g. a
        // hand-repaired row, or a future schema slip). RFC-085 §3.2: this
        // is the *safety* question — asserted against Locale::FAIL_CLOSED
        // by name, documenting intent, not guarding against a re-merge.
        // Handoff 079's flip means PRODUCT_DEFAULT (En) and FAIL_CLOSED
        // (Ja) now genuinely differ, so this value comparison happens to
        // catch a re-merge today — but that is a property of today's
        // values, not a guarantee: if a future change ever pointed both
        // constants at the same locale again, this comparison would go
        // blind exactly as it did before the flip. The actual, permanent
        // re-merge guard is the source-level gate
        // `resolve_locale_corrupt_value_arm_references_fail_closed_not_product_default`
        // (release_gates.rs), which reads which named constant the arm
        // references regardless of what either constant currently equals.
        for bad in ["fr", "EN", "", "ja-JP", "en-US", "null", "0"] {
            assert_eq!(
                resolve_locale(Some(bad)),
                Locale::FAIL_CLOSED,
                "stored={bad:?}"
            );
        }
    }

    /// Handoff 079 §5: the two ambient answers now visibly diverge, in one
    /// place — no expressed preference resolves to the *product* default
    /// (English, the decision this handoff took), while a corrupt stored
    /// value still resolves to the *safety* answer (Japanese, unmoved).
    /// This is RFC-085's separation made observable: before this flip
    /// these two assertions would have looked identical by coincidence.
    #[test]
    fn no_preference_and_a_corrupt_value_now_resolve_to_different_locales() {
        assert_eq!(resolve_locale(None), Locale::En);
        assert_eq!(resolve_locale(Some("fr")), Locale::Ja);
        assert_ne!(resolve_locale(None), resolve_locale(Some("fr")));
    }
}

/// Does a present (not-removed) membership exist for this user in this
/// community? RFC-082 / Handoff 058: `authz::require_membership` calls this
/// only after `find_active` has already failed, to distinguish "suspended"
/// (present, not active — an explicit page) from "genuinely absent"
/// (generic not-found). By the RFC-082 state model, present-but-not-active
/// necessarily means suspended, so no further check is needed here.
pub async fn exists_present(db: &D1Database, user_id: &str, community_id: &str) -> Result<bool> {
    let row = db
        .prepare(format!(
            "SELECT 1 FROM community_memberships \
             WHERE user_id = ?1 AND community_id = ?2 AND {MEMBERSHIP_PRESENT} \
             LIMIT 1"
        ))
        .bind(&[user_id.into(), community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.is_some())
}

/// Verify a membership_id is still active in a given community.
/// Used by the ICS feed handler to confirm access without a session.
pub async fn find_active_by_id(
    db: &D1Database,
    membership_id: &str,
    community_id: &str,
) -> Result<Option<MembershipRow>> {
    let row = db
        .prepare(format!(
            "SELECT id, community_id, role \
             FROM community_memberships \
             WHERE id = ?1 AND community_id = ?2 AND {MEMBERSHIP_ACTIVE} \
             LIMIT 1"
        ))
        .bind(&[membership_id.into(), community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(MembershipRow {
            id: v.get("id")?.as_str()?.to_owned(),
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            role: v.get("role")?.as_str()?.to_owned(),
        })
    }))
}

/// All active memberships for a user (for the communities list / session
/// boot).
///
/// RFC-081 §2 (Handoff 050 §5.4, carried from the Slice 1 review):
/// `scope_community_id` is required, not optional with a default, for the
/// same reason `list_communities_for_user` gained one in Handoff 049 — a
/// second enumeration of the same fact must not become a side door a
/// future caller can reach without thinking about session scope. `Some(id)`
/// restricts the result to that one community; `None` (a first-class,
/// unscoped session) returns every active membership, as before.
pub async fn list_active_for_user(
    db: &D1Database,
    user_id: &str,
    scope_community_id: Option<&str>,
) -> Result<Vec<MembershipRow>> {
    let rows = if let Some(scope) = scope_community_id {
        db.prepare(format!(
            "SELECT id, community_id, role \
             FROM community_memberships \
             WHERE user_id = ?1 AND {MEMBERSHIP_ACTIVE} AND community_id = ?2 \
             ORDER BY joined_at ASC"
        ))
        .bind(&[user_id.into(), scope.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?
    } else {
        db.prepare(format!(
            "SELECT id, community_id, role \
             FROM community_memberships \
             WHERE user_id = ?1 AND {MEMBERSHIP_ACTIVE} \
             ORDER BY joined_at ASC"
        ))
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?
    };

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(MembershipRow {
                id: v.get("id")?.as_str()?.to_owned(),
                community_id: v.get("community_id")?.as_str()?.to_owned(),
                role: v.get("role")?.as_str()?.to_owned(),
            })
        })
        .collect())
}

/// First active admin membership for a user, if any, with a *resolved*
/// locale (RFC-072, Handoff 030 §7.2). Used by non-community-scoped flows
/// that still require an existing admin — both as a presence check
/// (`me.rs`) and, for `/communities/new`, as the locale source for a page
/// with no `:cid` to read a "current membership" from.
pub async fn find_first_admin_for_user(
    db: &D1Database,
    user_id: &str,
) -> Result<Option<AdminMembershipRow>> {
    let row = db
        .prepare(format!(
            "SELECT id, community_id, user_id, role, display_name, ui_language \
             FROM community_memberships \
             WHERE user_id = ?1 AND role = 'admin' AND {MEMBERSHIP_ACTIVE} \
             ORDER BY joined_at ASC LIMIT 1"
        ))
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        let ui_language = v
            .get("ui_language")
            .and_then(|value| value.as_str())
            .map(str::to_owned);
        Some(AdminMembershipRow {
            id: v.get("id")?.as_str()?.to_owned(),
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            user_id: v.get("user_id")?.as_str()?.to_owned(),
            role: v.get("role")?.as_str()?.to_owned(),
            display_name: v.get("display_name")?.as_str()?.to_owned(),
            locale: resolve_locale(ui_language.as_deref()),
        })
    }))
}

/// Count active memberships in a community (for no_answer calculation).
pub async fn count_active(db: &D1Database, community_id: &str) -> Result<u32> {
    let row = db
        .prepare(format!(
            "SELECT COUNT(*) AS cnt FROM community_memberships \
             WHERE community_id = ?1 AND {MEMBERSHIP_ACTIVE}"
        ))
        .bind(&[community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.and_then(|v| v.get("cnt")?.as_u64()).unwrap_or(0) as u32)
}

/// One member row for participant-facing contexts (event/attendance/matrix
/// views). Never includes a suspended member — see [`PresentMemberSummary`]
/// for the admin member-management row, which does.
pub struct MemberSummary {
    pub id: String,
    pub display_name: String,
    pub role: String,
}

/// One member row for admin member-management contexts (RFC-082 §5): the
/// member list must show a suspended member, marked suspended, with an
/// unsuspend action — the reason `MEMBERSHIP_PRESENT` exists.
pub struct PresentMemberSummary {
    pub id: String,
    pub display_name: String,
    pub role: String,
    pub suspended_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleUpdateResult {
    Changed,
    AlreadyApplied,
    LastAdminBlocked,
    InvalidTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveMemberResult {
    Removed,
    LastAdminBlocked,
    InvalidTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspendResult {
    Suspended,
    AlreadySuspended,
    LastAdminBlocked,
    InvalidTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsuspendResult {
    Unsuspended,
    AlreadyActive,
    InvalidTarget,
}

/// Event/attendance/participant-tracking contexts (event.rs, communities.rs
/// matrix view, admin attendance override, admin note moderation): a
/// suspended member cannot use the app, so excluding them keeps these
/// surfaces consistent with `count_active` and with `db/attendance.rs`'s own
/// `MEMBERSHIP_ACTIVE` target check — see the coupling note in the review
/// request. For the admin member list, use [`list_present_for_admin`].
pub async fn list_all_active(db: &D1Database, community_id: &str) -> Result<Vec<MemberSummary>> {
    let rows = db
        .prepare(format!(
            "SELECT id, display_name, role FROM community_memberships \
             WHERE community_id = ?1 AND {MEMBERSHIP_ACTIVE} \
             ORDER BY display_name ASC, id ASC"
        ))
        .bind(&[community_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(MemberSummary {
                id: v.get("id")?.as_str()?.to_owned(),
                display_name: v.get("display_name")?.as_str()?.to_owned(),
                role: v.get("role")?.as_str()?.to_owned(),
            })
        })
        .collect())
}

/// RFC-082 §5: the admin member list, listing present (not-removed) members
/// including suspended ones, so an admin can see and unsuspend them.
pub async fn list_present_for_admin(
    db: &D1Database,
    community_id: &str,
) -> Result<Vec<PresentMemberSummary>> {
    let rows = db
        .prepare(format!(
            "SELECT id, display_name, role, suspended_at FROM community_memberships \
             WHERE community_id = ?1 AND {MEMBERSHIP_PRESENT} \
             ORDER BY display_name ASC, id ASC"
        ))
        .bind(&[community_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(PresentMemberSummary {
                id: v.get("id")?.as_str()?.to_owned(),
                display_name: v.get("display_name")?.as_str()?.to_owned(),
                role: v.get("role")?.as_str()?.to_owned(),
                suspended_at: v
                    .get("suspended_at")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned),
            })
        })
        .collect())
}

/// Role-transfer and help-signin confirmation targets: kept ACTIVE, since
/// neither promoting/demoting nor handing a signin code to an already-
/// suspended member has a clear need. For the removal-confirmation page
/// (which must remain reachable for a suspended target, since
/// suspended→removed is a valid RFC-082 §1 transition) and the
/// suspend/unsuspend confirmation pages, use [`find_present_summary`].
pub async fn find_active_summary(
    db: &D1Database,
    membership_id: &str,
    community_id: &str,
) -> Result<Option<MemberSummary>> {
    let row = db
        .prepare(format!(
            "SELECT id, display_name, role FROM community_memberships \
             WHERE id = ?1 AND community_id = ?2 AND {MEMBERSHIP_ACTIVE} \
             LIMIT 1"
        ))
        .bind(&[membership_id.into(), community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(MemberSummary {
            id: v.get("id")?.as_str()?.to_owned(),
            display_name: v.get("display_name")?.as_str()?.to_owned(),
            role: v.get("role")?.as_str()?.to_owned(),
        })
    }))
}

/// Present (not-removed) target lookup — reaches an already-suspended
/// member, unlike [`find_active_summary`]. Used by the removal-confirmation
/// page (RFC-082 §1: suspended→removed is a valid transition) and the
/// suspend/unsuspend confirmation pages and mutations.
pub async fn find_present_summary(
    db: &D1Database,
    membership_id: &str,
    community_id: &str,
) -> Result<Option<PresentMemberSummary>> {
    let row = db
        .prepare(format!(
            "SELECT id, display_name, role, suspended_at FROM community_memberships \
             WHERE id = ?1 AND community_id = ?2 AND {MEMBERSHIP_PRESENT} \
             LIMIT 1"
        ))
        .bind(&[membership_id.into(), community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(PresentMemberSummary {
            id: v.get("id")?.as_str()?.to_owned(),
            display_name: v.get("display_name")?.as_str()?.to_owned(),
            role: v.get("role")?.as_str()?.to_owned(),
            suspended_at: v
                .get("suspended_at")
                .and_then(|value| value.as_str())
                .map(str::to_owned),
        })
    }))
}

/// Count active admins in a community (for last-admin guard).
pub async fn count_admins(db: &D1Database, community_id: &str) -> Result<u32> {
    let row = db
        .prepare(format!(
            "SELECT COUNT(*) AS cnt FROM community_memberships \
             WHERE community_id = ?1 AND role = 'admin' AND {MEMBERSHIP_ACTIVE}"
        ))
        .bind(&[community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.and_then(|v| v.get("cnt")?.as_u64()).unwrap_or(0) as u32)
}

/// Get role string for a membership_id, scoped to community_id.
pub async fn get_role(
    db: &D1Database,
    membership_id: &str,
    community_id: &str,
) -> Result<Option<String>> {
    let row = db
        .prepare(format!(
            "SELECT role FROM community_memberships \
             WHERE id = ?1 AND community_id = ?2 AND {MEMBERSHIP_ACTIVE} LIMIT 1"
        ))
        .bind(&[membership_id.into(), community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.and_then(|v| v.get("role")?.as_str().map(|s| s.to_owned())))
}

pub async fn promote_to_admin_required(
    db: &D1Database,
    request_id: &str,
    membership_id: &str,
    community_id: &str,
    actor_membership_id: &str,
) -> Result<RoleUpdateResult> {
    let mutation = db
        .prepare(format!(
            "UPDATE community_memberships \
             SET role = 'admin' \
             WHERE id = ?1 \
               AND community_id = ?2 \
               AND {MEMBERSHIP_ACTIVE} \
               AND role = 'member' \
               AND id != ?3 \
               AND EXISTS ( \
                 SELECT 1 FROM community_memberships \
                 WHERE id = ?3 AND community_id = ?2 \
                   AND role = 'admin' AND {MEMBERSHIP_ACTIVE} \
               )"
        ))
        .bind(&[
            membership_id.into(),
            community_id.into(),
            actor_membership_id.into(),
        ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(actor_membership_id),
        Some(membership_id),
        AuditAction::MembershipPromotedToAdmin,
        AuditMetadata::None,
    )?;
    if audit::execute_required(db, mutation, &record).await? {
        return Ok(RoleUpdateResult::Changed);
    }

    match get_role(db, membership_id, community_id).await?.as_deref() {
        Some("admin") => Ok(RoleUpdateResult::AlreadyApplied),
        _ => Ok(RoleUpdateResult::InvalidTarget),
    }
}

pub async fn demote_to_member_required(
    db: &D1Database,
    request_id: &str,
    membership_id: &str,
    community_id: &str,
    actor_membership_id: &str,
) -> Result<RoleUpdateResult> {
    let mutation = db
        .prepare(format!(
            "UPDATE community_memberships \
             SET role = 'member' \
             WHERE id = ?1 \
               AND community_id = ?2 \
               AND {MEMBERSHIP_ACTIVE} \
               AND role = 'admin' \
               AND id != ?3 \
               AND EXISTS ( \
                 SELECT 1 FROM community_memberships \
                 WHERE id = ?3 AND community_id = ?2 \
                   AND role = 'admin' AND {MEMBERSHIP_ACTIVE} \
               ) \
               AND (SELECT COUNT(*) FROM community_memberships \
                    WHERE community_id = ?2 AND role = 'admin' AND {MEMBERSHIP_ACTIVE}) > 1"
        ))
        .bind(&[
            membership_id.into(),
            community_id.into(),
            actor_membership_id.into(),
        ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(actor_membership_id),
        Some(membership_id),
        AuditAction::MembershipDemotedToMember,
        AuditMetadata::None,
    )?;
    if audit::execute_required(db, mutation, &record).await? {
        return Ok(RoleUpdateResult::Changed);
    }

    match get_role(db, membership_id, community_id).await?.as_deref() {
        Some("member") => Ok(RoleUpdateResult::AlreadyApplied),
        Some("admin") if count_admins(db, community_id).await? <= 1 => {
            Ok(RoleUpdateResult::LastAdminBlocked)
        }
        _ => Ok(RoleUpdateResult::InvalidTarget),
    }
}

/// Soft-remove a member while preserving the at-least-one-admin invariant.
///
/// RFC-082 §1: `suspended → removed` is a valid transition, so the target
/// check here is `MEMBERSHIP_PRESENT`, not `MEMBERSHIP_ACTIVE` — the one
/// deliberate exception in this file to the fail-closed default. Removing an
/// already-suspended member must succeed; the actor check stays ACTIVE.
pub async fn soft_remove_guarded_required(
    db: &D1Database,
    request_id: &str,
    membership_id: &str,
    community_id: &str,
    actor_membership_id: &str,
) -> Result<RemoveMemberResult> {
    let now = crate::db::now_utc();
    let mutation = db
        .prepare(format!(
            "UPDATE community_memberships \
             SET removed_at = ?1 \
             WHERE id = ?2 \
               AND community_id = ?3 \
               AND {MEMBERSHIP_PRESENT} \
               AND id != ?4 \
               AND EXISTS ( \
                 SELECT 1 FROM community_memberships \
                 WHERE id = ?4 AND community_id = ?3 \
                   AND role = 'admin' AND {MEMBERSHIP_ACTIVE} \
               ) \
               AND (role != 'admin' OR \
                    (SELECT COUNT(*) FROM community_memberships \
                     WHERE community_id = ?3 AND role = 'admin' AND {MEMBERSHIP_ACTIVE}) > 1)"
        ))
        .bind(&[
            now.as_str().into(),
            membership_id.into(),
            community_id.into(),
            actor_membership_id.into(),
        ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(actor_membership_id),
        Some(membership_id),
        AuditAction::MembershipRemoved,
        AuditMetadata::None,
    )?;
    if audit::execute_required(db, mutation, &record).await? {
        return Ok(RemoveMemberResult::Removed);
    }

    // `find_present_summary`, not `get_role` (which is ACTIVE-scoped): the
    // target may be suspended, and that must still resolve to
    // `LastAdminBlocked` when it's the community's last admin, not fall
    // through to `InvalidTarget`.
    match find_present_summary(db, membership_id, community_id).await? {
        Some(row) if row.role == "admin" && count_admins(db, community_id).await? <= 1 => {
            Ok(RemoveMemberResult::LastAdminBlocked)
        }
        _ => Ok(RemoveMemberResult::InvalidTarget),
    }
}

/// One community entry for user-scoped navigation and summaries.
pub struct CommunitySummary {
    pub community_id: String,
    pub community_name: String,
    pub timezone: String,
    pub role: String,
}

/// All communities a user is present in (RFC-082: including suspended ones),
/// with display metadata, ordered by joined_at. Used for navigation and
/// multi-community summaries.
///
/// RFC-081 §2 (Handoff 049): `scope_community_id` is required, not optional
/// with a default, so a caller cannot forget it the way `get_communities`
/// and `get_switch` did in Handoff 048 — pass `auth.scope_community_id`.
/// `Some(id)` restricts the result to that one community (a community-bound
/// session must never see or reach any other); `None` (a first-class,
/// unscoped session) returns every community, as before.
///
/// RFC-082 / Handoff 058: `MEMBERSHIP_PRESENT`, not `MEMBERSHIP_ACTIVE` — a
/// suspended community must stay in the switcher and the `/account` list.
/// Suspension is meant to be discoverable (RFC-082 §4's explicit paused
/// page), not indistinguishable from never having joined; hiding a
/// suspended community here would look identical to removal and defeat
/// that transparency goal. `community_memberships` is the only table with a
/// `removed_at`/`suspended_at` column, so the bare predicate resolves
/// unambiguously against `m` despite the join.
pub async fn list_communities_for_user(
    db: &D1Database,
    user_id: &str,
    scope_community_id: Option<&str>,
) -> Result<Vec<CommunitySummary>> {
    let rows = if let Some(scope) = scope_community_id {
        db.prepare(format!(
            "SELECT m.community_id, c.name AS community_name, c.timezone, m.role \
             FROM community_memberships m \
             JOIN communities c ON c.id = m.community_id \
             WHERE m.user_id = ?1 AND {MEMBERSHIP_PRESENT} AND m.community_id = ?2 \
             ORDER BY m.joined_at ASC"
        ))
        .bind(&[user_id.into(), scope.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?
    } else {
        db.prepare(format!(
            "SELECT m.community_id, c.name AS community_name, c.timezone, m.role \
             FROM community_memberships m \
             JOIN communities c ON c.id = m.community_id \
             WHERE m.user_id = ?1 AND {MEMBERSHIP_PRESENT} \
             ORDER BY m.joined_at ASC"
        ))
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?
    };

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(CommunitySummary {
                community_id: v.get("community_id")?.as_str()?.to_owned(),
                community_name: v.get("community_name")?.as_str()?.to_owned(),
                timezone: v
                    .get("timezone")
                    .and_then(|value| value.as_str())
                    .unwrap_or("UTC")
                    .to_owned(),
                role: v.get("role")?.as_str()?.to_owned(),
            })
        })
        .collect())
}

/// One community entry alongside that membership's raw stored
/// `ui_language` — RFC-084 §4 (Handoff 084). Wraps [`CommunitySummary`]
/// rather than duplicating its fields, so the account tier's locale ladder
/// (which needs every present membership's preference) and its community
/// listing (which needs [`CommunitySummary`]) are served by the one query
/// [`list_communities_with_locale_for_user`] runs — required so that
/// `CommunitySummary` itself, read by 23 other call sites through
/// [`list_communities_for_user`], never widens to carry a language value
/// none of them want (RFC-084 §7).
pub struct CommunityLocaleRow {
    pub summary: CommunitySummary,
    pub ui_language: Option<String>,
}

/// The account tier's sibling to [`list_communities_for_user`] (RFC-084 §4,
/// Handoff 084): the same unscoped SELECT — the account tier is never
/// community-scoped, so there is no `scope_community_id` parameter to take
/// — plus `m.ui_language`. `account/mod.rs` calls this **instead of**
/// [`list_communities_for_user`] (one query, swapped, not added);
/// `account/link.rs` and `account/unlink.rs`, which make no membership
/// query today, each pay one new query to reach the same resolution
/// (RFC-084 §10 decision 2).
pub async fn list_communities_with_locale_for_user(
    db: &D1Database,
    user_id: &str,
) -> Result<Vec<CommunityLocaleRow>> {
    let rows = db
        .prepare(format!(
            "SELECT m.community_id, c.name AS community_name, c.timezone, m.role, m.ui_language \
             FROM community_memberships m \
             JOIN communities c ON c.id = m.community_id \
             WHERE m.user_id = ?1 AND {MEMBERSHIP_PRESENT} \
             ORDER BY m.joined_at ASC"
        ))
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            let summary = CommunitySummary {
                community_id: v.get("community_id")?.as_str()?.to_owned(),
                community_name: v.get("community_name")?.as_str()?.to_owned(),
                timezone: v
                    .get("timezone")
                    .and_then(|value| value.as_str())
                    .unwrap_or("UTC")
                    .to_owned(),
                role: v.get("role")?.as_str()?.to_owned(),
            };
            let ui_language = v
                .get("ui_language")
                .and_then(|value| value.as_str())
                .map(str::to_owned);
            Some(CommunityLocaleRow {
                summary,
                ui_language,
            })
        })
        .collect())
}

/// Suspend a member while preserving the at-least-one-admin invariant —
/// RFC-082 §1: active → suspended, community admin only. No role change
/// (RFC-082 §8.10).
///
/// The last-admin guard here is not itself required by RFC-082's transition
/// table, but follows the same shape `soft_remove_guarded_required` already
/// enforces, for a reason specific to suspension: unsuspending requires an
/// *active* admin actor (`MEMBERSHIP_ACTIVE`), and no other path in this
/// package restores one. Suspending a community's last active admin would
/// leave nobody who could ever unsuspend anyone — a permanent lockout, and
/// a worse failure than the visible "last admin blocked" refusal this
/// guard produces instead.
pub async fn suspend_required(
    db: &D1Database,
    request_id: &str,
    membership_id: &str,
    community_id: &str,
    actor_membership_id: &str,
) -> Result<SuspendResult> {
    let now = crate::db::now_utc();
    let mutation = db
        .prepare(format!(
            "UPDATE community_memberships \
             SET suspended_at = ?1, suspended_by_membership_id = ?4 \
             WHERE id = ?2 \
               AND community_id = ?3 \
               AND {MEMBERSHIP_ACTIVE} \
               AND id != ?4 \
               AND EXISTS ( \
                 SELECT 1 FROM community_memberships \
                 WHERE id = ?4 AND community_id = ?3 \
                   AND role = 'admin' AND {MEMBERSHIP_ACTIVE} \
               ) \
               AND (role != 'admin' OR \
                    (SELECT COUNT(*) FROM community_memberships \
                     WHERE community_id = ?3 AND role = 'admin' AND {MEMBERSHIP_ACTIVE}) > 1)"
        ))
        .bind(&[
            now.as_str().into(),
            membership_id.into(),
            community_id.into(),
            actor_membership_id.into(),
        ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(actor_membership_id),
        Some(membership_id),
        AuditAction::MembershipSuspended,
        AuditMetadata::None,
    )?;
    if audit::execute_required(db, mutation, &record).await? {
        return Ok(SuspendResult::Suspended);
    }

    match find_present_summary(db, membership_id, community_id).await? {
        Some(row) if row.suspended_at.is_some() => Ok(SuspendResult::AlreadySuspended),
        Some(row) if row.role == "admin" && count_admins(db, community_id).await? <= 1 => {
            Ok(SuspendResult::LastAdminBlocked)
        }
        _ => Ok(SuspendResult::InvalidTarget),
    }
}

/// Reverse a suspension, restoring the member's prior role unchanged —
/// RFC-082 §1: suspended → active, community admin only. No session
/// revocation (RFC-082 §7): a session carries a principal, not an
/// authorization, and the member's next request to any community simply
/// passes `MEMBERSHIP_ACTIVE` again.
pub async fn unsuspend_required(
    db: &D1Database,
    request_id: &str,
    membership_id: &str,
    community_id: &str,
    actor_membership_id: &str,
) -> Result<UnsuspendResult> {
    let mutation = db
        .prepare(format!(
            "UPDATE community_memberships \
             SET suspended_at = NULL, suspended_by_membership_id = NULL \
             WHERE id = ?1 \
               AND community_id = ?2 \
               AND {MEMBERSHIP_PRESENT} \
               AND suspended_at IS NOT NULL \
               AND id != ?3 \
               AND EXISTS ( \
                 SELECT 1 FROM community_memberships \
                 WHERE id = ?3 AND community_id = ?2 \
                   AND role = 'admin' AND {MEMBERSHIP_ACTIVE} \
               )"
        ))
        .bind(&[
            membership_id.into(),
            community_id.into(),
            actor_membership_id.into(),
        ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(actor_membership_id),
        Some(membership_id),
        AuditAction::MembershipUnsuspended,
        AuditMetadata::None,
    )?;
    if audit::execute_required(db, mutation, &record).await? {
        return Ok(UnsuspendResult::Unsuspended);
    }

    match find_present_summary(db, membership_id, community_id).await? {
        Some(row) if row.suspended_at.is_none() => Ok(UnsuspendResult::AlreadyActive),
        _ => Ok(UnsuspendResult::InvalidTarget),
    }
}
