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
