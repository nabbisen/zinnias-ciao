#![allow(dead_code)]
//! Membership and user table access — RFC-002 / RFC-004.

use crate::db::now_utc;
use worker::{D1Database, Result};

pub struct MembershipRow {
    pub id: String,
    pub community_id: String,
    pub user_id: String,
    pub role: String,
    pub display_name: String,
    pub is_active: bool,
}

/// Find an active membership for the given user + community.
/// Returns `None` if absent or removed (`removed_at IS NOT NULL`).
pub async fn find_active(
    db: &D1Database,
    user_id: &str,
    community_id: &str,
) -> Result<Option<MembershipRow>> {
    let row = db
        .prepare(
            "SELECT id, community_id, user_id, role, display_name \
             FROM community_memberships \
             WHERE user_id = ?1 AND community_id = ?2 AND removed_at IS NULL \
             LIMIT 1",
        )
        .bind(&[user_id.into(), community_id.into()])?
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

/// First active admin membership for a user, if any.
/// Used by non-community-scoped flows that still require an existing admin.
pub async fn find_first_admin_for_user(
    db: &D1Database,
    user_id: &str,
) -> Result<Option<MembershipRow>> {
    let row = db
        .prepare(
            "SELECT id, community_id, user_id, role, display_name \
             FROM community_memberships \
             WHERE user_id = ?1 AND role = 'admin' AND removed_at IS NULL \
             ORDER BY joined_at ASC LIMIT 1",
        )
        .bind(&[user_id.into()])?
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

pub async fn list_all_active(db: &D1Database, community_id: &str) -> Result<Vec<MemberSummary>> {
    let rows = db
        .prepare(
            "SELECT id, display_name, role FROM community_memberships \
             WHERE community_id = ?1 AND removed_at IS NULL \
             ORDER BY display_name ASC",
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

/// Soft-remove a member (sets removed_at, preserves history — RFC-010 §5).
/// Scoped to community_id to prevent cross-community removal.
pub async fn soft_remove(db: &D1Database, membership_id: &str, community_id: &str) -> Result<()> {
    let now = crate::db::now_utc();
    db.prepare(
        "UPDATE community_memberships SET removed_at = ?1 \
         WHERE id = ?2 AND community_id = ?3 AND removed_at IS NULL",
    )
    .bind(&[
        now.as_str().into(),
        membership_id.into(),
        community_id.into(),
    ])?
    .run()
    .await?;
    Ok(())
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
