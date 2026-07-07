#![allow(dead_code)]
//! Active-member help-signin code table access — RFC-024.

use worker::{D1Database, Result};
use zinnias_ciao_contracts::RELINK_CODE_TTL_SECONDS;

use crate::db::{add_seconds_to_now, now_utc};

pub struct RelinkTargetRow {
    pub id: String,
    pub community_id: String,
    pub membership_id: String,
    pub created_by_membership_id: String,
    pub user_id: String,
}

pub fn expires_at() -> String {
    add_seconds_to_now(RELINK_CODE_TTL_SECONDS)
}

/// Revoke previous unused codes for a membership before issuing a new one.
pub async fn revoke_unused_for_membership(db: &D1Database, membership_id: &str) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "UPDATE membership_relink_codes \
         SET revoked_at = ?1 \
         WHERE membership_id = ?2 \
           AND used_at IS NULL \
           AND revoked_at IS NULL \
           AND expires_at > ?1",
    )
    .bind(&[now.as_str().into(), membership_id.into()])?
    .run()
    .await?;
    Ok(())
}

pub async fn insert(
    db: &D1Database,
    id: &str,
    code_hmac: &str,
    community_id: &str,
    membership_id: &str,
    created_by_membership_id: &str,
    expires_at: &str,
) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "INSERT INTO membership_relink_codes \
         (id, code_hmac, community_id, membership_id, created_by_membership_id, created_at, expires_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&[
        id.into(),
        code_hmac.into(),
        community_id.into(),
        membership_id.into(),
        created_by_membership_id.into(),
        now.as_str().into(),
        expires_at.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Resolve a valid code to an active membership and its current user_id.
///
/// The membership/community join is the defensive RFC-024 check: a code targets
/// membership_id, but session minting must only proceed when that membership is
/// still active and still belongs to the code's community.
pub async fn find_valid_by_hmac(
    db: &D1Database,
    code_hmac: &str,
) -> Result<Option<RelinkTargetRow>> {
    let now = now_utc();
    let row = db
        .prepare(
            "SELECT r.id, r.community_id, r.membership_id, r.created_by_membership_id, m.user_id \
             FROM membership_relink_codes r \
             JOIN community_memberships m ON m.id = r.membership_id \
             WHERE r.code_hmac = ?1 \
               AND r.used_at IS NULL \
               AND r.revoked_at IS NULL \
               AND r.expires_at > ?2 \
               AND m.removed_at IS NULL \
               AND m.community_id = r.community_id \
             LIMIT 1",
        )
        .bind(&[code_hmac.into(), now.as_str().into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(RelinkTargetRow {
            id: v.get("id")?.as_str()?.to_owned(),
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            membership_id: v.get("membership_id")?.as_str()?.to_owned(),
            created_by_membership_id: v.get("created_by_membership_id")?.as_str()?.to_owned(),
            user_id: v.get("user_id")?.as_str()?.to_owned(),
        })
    }))
}

/// Mark a code used with the same validity checks used during lookup.
/// Returns true only for the caller that wins the single-use transition.
pub async fn mark_used(db: &D1Database, id: &str) -> Result<bool> {
    let now = now_utc();
    let res = db
        .prepare(
            "UPDATE membership_relink_codes \
             SET used_at = ?1 \
             WHERE id = ?2 \
               AND used_at IS NULL \
               AND revoked_at IS NULL \
               AND expires_at > ?1",
        )
        .bind(&[now.as_str().into(), id.into()])?
        .run()
        .await?;
    let changed = res
        .meta()
        .ok()
        .flatten()
        .and_then(|m| m.changes)
        .unwrap_or(0);
    Ok(changed == 1)
}
