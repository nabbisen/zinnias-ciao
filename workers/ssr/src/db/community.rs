//! Community table access.

use worker::{D1Database, Result};

use crate::audit::{self, AuditAction, AuditMetadata};
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

#[allow(clippy::too_many_arguments)]
pub async fn create_with_first_admin(
    db: &D1Database,
    request_id: &str,
    community_id: &str,
    name: &str,
    timezone: &str,
    first_admin_membership_id: &str,
    user_id: &str,
    display_name: &str,
) -> Result<()> {
    let now = now_utc();

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

    let primary_audit = audit::required_record(
        request_id,
        Some(community_id),
        Some(first_admin_membership_id),
        Some(community_id),
        AuditAction::CommunityCreated,
        AuditMetadata::None,
    )?;
    let membership_audit = audit::required_record(
        request_id,
        Some(community_id),
        Some(first_admin_membership_id),
        Some(first_admin_membership_id),
        AuditAction::MembershipCreatedFirstAdmin,
        AuditMetadata::None,
    )?;
    audit::execute_required_batch(
        db,
        vec![community_stmt, membership_stmt],
        &primary_audit,
        &[membership_audit],
    )
    .await?;
    Ok(())
}
