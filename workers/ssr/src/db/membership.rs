//! Membership and user table access — RFC-002 / RFC-004.

use crate::audit::{self, AuditAction, AuditMetadata};
use crate::db::now_utc;
use worker::{D1Database, Result};
use zinnias_ciao_contracts::Locale;

pub struct MembershipRow {
    pub id: String,
    pub community_id: String,
    pub user_id: String,
    pub role: String,
    pub display_name: String,
    pub is_active: bool,
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
    pub is_active: bool,
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
    pub is_active: bool,
    pub locale: Locale,
}

/// Resolves the raw stored `ui_language` column (RFC-072): the membership's
/// preference if it parses, else Japanese. A stored value outside the
/// `CHECK` allow-list — never expected, but possible via manual repair —
/// falls back the same way as no preference at all. Never panics: a bad
/// stored value reaching a render path would be an SEC-5 violation.
fn resolve_locale(stored: Option<&str>) -> Locale {
    stored.and_then(Locale::parse).unwrap_or_default()
}

/// Find an active membership for the given user + community. This is the
/// query every localized page's membership lookup already performs; RFC-072
/// reads `ui_language` from this same row rather than adding a second query,
/// and resolves it here into [`ActiveMembershipRow::locale`] — the only
/// trustworthy source of a page's locale.
/// Returns `None` if absent or removed (`removed_at IS NOT NULL`).
pub async fn find_active(
    db: &D1Database,
    user_id: &str,
    community_id: &str,
) -> Result<Option<ActiveMembershipRow>> {
    let row = db
        .prepare(
            "SELECT id, community_id, user_id, role, display_name, ui_language \
             FROM community_memberships \
             WHERE user_id = ?1 AND community_id = ?2 AND removed_at IS NULL \
             LIMIT 1",
        )
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
            is_active: true,
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
    fn resolve_locale_falls_back_to_japanese_when_absent() {
        assert_eq!(resolve_locale(None), Locale::Ja);
    }

    #[test]
    fn resolve_locale_falls_back_to_japanese_for_an_out_of_allow_list_value_without_panicking() {
        // A value the CHECK constraint should have rejected on write, but
        // that a defensive read path must still survive (e.g. a
        // hand-repaired row, or a future schema slip).
        for bad in ["fr", "EN", "", "ja-JP", "en-US", "null", "0"] {
            assert_eq!(resolve_locale(Some(bad)), Locale::Ja, "stored={bad:?}");
        }
    }
}

/// Verify a membership_id is still active in a given community.
/// Used by the ICS feed handler to confirm access without a session.
pub async fn find_active_by_id(
    db: &D1Database,
    membership_id: &str,
    community_id: &str,
) -> Result<Option<MembershipRow>> {
    let row = db
        .prepare(
            "SELECT id, community_id, user_id, role, display_name \
             FROM community_memberships \
             WHERE id = ?1 AND community_id = ?2 AND removed_at IS NULL \
             LIMIT 1",
        )
        .bind(&[membership_id.into(), community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(MembershipRow {
            id: v.get("id")?.as_str()?.to_owned(),
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            user_id: v.get("user_id")?.as_str()?.to_owned(),
            role: v.get("role")?.as_str()?.to_owned(),
            display_name: v.get("display_name")?.as_str()?.to_owned(),
            is_active: true,
        })
    }))
}

/// All active memberships for a user (for the communities list / session boot).
pub async fn list_active_for_user(db: &D1Database, user_id: &str) -> Result<Vec<MembershipRow>> {
    let rows = db
        .prepare(
            "SELECT id, community_id, user_id, role, display_name \
             FROM community_memberships \
             WHERE user_id = ?1 AND removed_at IS NULL \
             ORDER BY joined_at ASC",
        )
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(MembershipRow {
                id: v.get("id")?.as_str()?.to_owned(),
                community_id: v.get("community_id")?.as_str()?.to_owned(),
                user_id: v.get("user_id")?.as_str()?.to_owned(),
                role: v.get("role")?.as_str()?.to_owned(),
                display_name: v.get("display_name")?.as_str()?.to_owned(),
                is_active: true,
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
        .prepare(
            "SELECT id, community_id, user_id, role, display_name, ui_language \
             FROM community_memberships \
             WHERE user_id = ?1 AND role = 'admin' AND removed_at IS NULL \
             ORDER BY joined_at ASC LIMIT 1",
        )
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
            is_active: true,
            locale: resolve_locale(ui_language.as_deref()),
        })
    }))
}

/// Create a user row (used during invite redemption for new users).
pub async fn insert_user(db: &D1Database, user_id: &str) -> Result<()> {
    let now = now_utc();
    db.prepare("INSERT OR IGNORE INTO users (id, created_at) VALUES (?1, ?2)")
        .bind(&[user_id.into(), now.as_str().into()])?
        .run()
        .await?;
    Ok(())
}

/// Create a community membership row.
pub async fn insert_membership(
    db: &D1Database,
    id: &str,
    community_id: &str,
    user_id: &str,
    role: &str,
    display_name: &str,
) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "INSERT INTO community_memberships \
         (id, community_id, user_id, role, display_name, joined_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&[
        id.into(),
        community_id.into(),
        user_id.into(),
        role.into(),
        display_name.into(),
        now.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Count active memberships in a community (for no_answer calculation).
pub async fn count_active(db: &D1Database, community_id: &str) -> Result<u32> {
    let row = db
        .prepare(
            "SELECT COUNT(*) AS cnt FROM community_memberships \
             WHERE community_id = ?1 AND removed_at IS NULL",
        )
        .bind(&[community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.and_then(|v| v.get("cnt")?.as_u64()).unwrap_or(0) as u32)
}

/// All active memberships for a community (for participant list).
pub struct MemberSummary {
    pub id: String,
    pub display_name: String,
    pub role: String,
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

pub async fn list_all_active(db: &D1Database, community_id: &str) -> Result<Vec<MemberSummary>> {
    let rows = db
        .prepare(
            "SELECT id, display_name, role FROM community_memberships \
             WHERE community_id = ?1 AND removed_at IS NULL \
             ORDER BY display_name ASC, id ASC",
        )
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

pub async fn find_active_summary(
    db: &D1Database,
    membership_id: &str,
    community_id: &str,
) -> Result<Option<MemberSummary>> {
    let row = db
        .prepare(
            "SELECT id, display_name, role FROM community_memberships \
             WHERE id = ?1 AND community_id = ?2 AND removed_at IS NULL \
             LIMIT 1",
        )
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

/// Count active admins in a community (for last-admin guard).
pub async fn count_admins(db: &D1Database, community_id: &str) -> Result<u32> {
    let row = db
        .prepare(
            "SELECT COUNT(*) AS cnt FROM community_memberships \
             WHERE community_id = ?1 AND role = 'admin' AND removed_at IS NULL",
        )
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
        .prepare(
            "SELECT role FROM community_memberships \
             WHERE id = ?1 AND community_id = ?2 AND removed_at IS NULL LIMIT 1",
        )
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
        .prepare(
            "UPDATE community_memberships \
             SET role = 'admin' \
             WHERE id = ?1 \
               AND community_id = ?2 \
               AND removed_at IS NULL \
               AND role = 'member' \
               AND id != ?3 \
               AND EXISTS ( \
                 SELECT 1 FROM community_memberships \
                 WHERE id = ?3 AND community_id = ?2 \
                   AND role = 'admin' AND removed_at IS NULL \
               )",
        )
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
        .prepare(
            "UPDATE community_memberships \
             SET role = 'member' \
             WHERE id = ?1 \
               AND community_id = ?2 \
               AND removed_at IS NULL \
               AND role = 'admin' \
               AND id != ?3 \
               AND EXISTS ( \
                 SELECT 1 FROM community_memberships \
                 WHERE id = ?3 AND community_id = ?2 \
                   AND role = 'admin' AND removed_at IS NULL \
               ) \
               AND (SELECT COUNT(*) FROM community_memberships \
                    WHERE community_id = ?2 AND role = 'admin' AND removed_at IS NULL) > 1",
        )
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
pub async fn soft_remove_guarded_required(
    db: &D1Database,
    request_id: &str,
    membership_id: &str,
    community_id: &str,
    actor_membership_id: &str,
) -> Result<RemoveMemberResult> {
    let now = crate::db::now_utc();
    let mutation = db
        .prepare(
            "UPDATE community_memberships \
             SET removed_at = ?1 \
             WHERE id = ?2 \
               AND community_id = ?3 \
               AND removed_at IS NULL \
               AND id != ?4 \
               AND EXISTS ( \
                 SELECT 1 FROM community_memberships \
                 WHERE id = ?4 AND community_id = ?3 \
                   AND role = 'admin' AND removed_at IS NULL \
               ) \
               AND (role != 'admin' OR \
                    (SELECT COUNT(*) FROM community_memberships \
                     WHERE community_id = ?3 AND role = 'admin' AND removed_at IS NULL) > 1)",
        )
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

    match get_role(db, membership_id, community_id).await?.as_deref() {
        Some("admin") if count_admins(db, community_id).await? <= 1 => {
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

/// All communities a user is an active member of, with display metadata,
/// ordered by joined_at. Used for navigation and multi-community summaries.
pub async fn list_communities_for_user(
    db: &D1Database,
    user_id: &str,
) -> Result<Vec<CommunitySummary>> {
    let rows = db
        .prepare(
            "SELECT m.community_id, c.name AS community_name, c.timezone, m.role \
             FROM community_memberships m \
             JOIN communities c ON c.id = m.community_id \
             WHERE m.user_id = ?1 AND m.removed_at IS NULL \
             ORDER BY m.joined_at ASC",
        )
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

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
