#![allow(dead_code)]
//! Community table access.

use worker::{D1Database, Result};

use crate::crypto::random_token;
use crate::db::now_utc;

pub struct CommunityRow {
    pub id: String,
    pub name: String,
    pub timezone: String,
}

pub async fn find_active(db: &D1Database, community_id: &str) -> Result<Option<CommunityRow>> {
    let row = db
        .prepare(
            "SELECT id, name, timezone FROM communities \
             WHERE id = ?1 AND is_active = 1 LIMIT 1",
        )
        .bind(&[community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(CommunityRow {
            id: v.get("id")?.as_str()?.to_owned(),
            name: v.get("name")?.as_str()?.to_owned(),
            timezone: v.get("timezone")?.as_str()?.to_owned(),
        })
    }))
}

pub async fn create_with_first_admin(
    db: &D1Database,
    community_id: &str,
    name: &str,
    timezone: &str,
    first_admin_membership_id: &str,
    user_id: &str,
    display_name: &str,
) -> Result<()> {
    let now = now_utc();
    let community_audit_id = format!("aud_{}", &random_token()[..24]);
    let membership_audit_id = format!("aud_{}", &random_token()[..24]);

    let community_stmt = db
        .prepare(
            "INSERT INTO communities (id, name, timezone, is_active, created_at) \
             VALUES (?1, ?2, ?3, 1, ?4)",
        )
        .bind(&[
            community_id.into(),
            name.into(),
            timezone.into(),
            now.as_str().into(),
        ])?;

    let membership_stmt = db
        .prepare(
            "INSERT INTO community_memberships \
             (id, community_id, user_id, role, display_name, joined_at) \
             VALUES (?1, ?2, ?3, 'admin', ?4, ?5)",
        )
        .bind(&[
            first_admin_membership_id.into(),
            community_id.into(),
            user_id.into(),
            display_name.into(),
            now.as_str().into(),
        ])?;

    let community_audit_stmt = db
        .prepare(
            "INSERT INTO audit_log \
             (id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at) \
             VALUES (?1, ?2, ?3, 'community', ?4, 'community.created', '{}', ?5)",
        )
        .bind(&[
            community_audit_id.as_str().into(),
            community_id.into(),
            first_admin_membership_id.into(),
            community_id.into(),
            now.as_str().into(),
        ])?;

    let membership_audit_stmt = db
        .prepare(
            "INSERT INTO audit_log \
             (id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at) \
             VALUES (?1, ?2, ?3, 'membership', ?4, 'membership.created_first_admin', '{}', ?5)",
        )
        .bind(&[
            membership_audit_id.as_str().into(),
            community_id.into(),
            first_admin_membership_id.into(),
            first_admin_membership_id.into(),
            now.as_str().into(),
        ])?;

    db.batch(vec![
        community_stmt,
        membership_stmt,
        community_audit_stmt,
        membership_audit_stmt,
    ])
    .await?;
    Ok(())
}
