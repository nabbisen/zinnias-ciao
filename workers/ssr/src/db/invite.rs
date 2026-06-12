#![allow(dead_code)]
//! Invite-code table access — RFC-003 / RFC-002.
//!
//! Codes are stored as HMAC-SHA256(pepper, normalize(code)).
//! All state changes (used, revoked) are soft — no hard deletes.

use crate::db::now_utc;
use worker::{D1Database, Result};

pub struct InviteRow {
    pub id: String,
    pub community_id: String,
    /// Role to grant the joining user — 'admin' or 'member'.
    pub grants_role: String,
    /// Whether the code has already been used or revoked or is expired
    pub is_valid: bool,
}

/// Look up an invite code by HMAC.
/// Returns the row only if the code exists, is not used, not revoked, and not expired.
pub async fn find_valid(db: &D1Database, code_hmac: &str) -> Result<Option<InviteRow>> {
    let now = now_utc();
    let row = db
        .prepare(
            "SELECT id, community_id, grants_role \
             FROM invite_codes \
             WHERE code_hmac = ?1 \
               AND used_at IS NULL \
               AND revoked_at IS NULL \
               AND expires_at > ?2 \
             LIMIT 1",
        )
        .bind(&[code_hmac.into(), now.as_str().into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(InviteRow {
            id:           v.get("id")?.as_str()?.to_owned(),
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            grants_role:  v.get("grants_role")
                           .and_then(|x| x.as_str())
                           .unwrap_or("member")
                           .to_owned(),
            is_valid: true,
        })
    }))
}

/// Look up an invite code by its ID to retrieve grants_role at redemption time.
/// Used by post_profile after the ticket is validated — the HMAC check already
/// happened in post_join; here we just need the role the code confers.
pub async fn find_by_id(db: &D1Database, invite_id: &str) -> Result<Option<InviteRow>> {
    let row = db
        .prepare(
            "SELECT id, community_id, grants_role \
             FROM invite_codes \
             WHERE id = ?1 \
               AND used_at IS NULL \
               AND revoked_at IS NULL \
             LIMIT 1",
        )
        .bind(&[invite_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(InviteRow {
            id:           v.get("id")?.as_str()?.to_owned(),
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            grants_role:  v.get("grants_role")
                           .and_then(|x| x.as_str())
                           .unwrap_or("member")
                           .to_owned(),
            is_valid: true,
        })
    }))
}

/// Mark an invite code as used (atomic with session/membership creation in the handler).
pub async fn mark_used(db: &D1Database, invite_id: &str, membership_id: &str) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "UPDATE invite_codes \
         SET used_at = ?1, used_by_membership_id = ?2 \
         WHERE id = ?3",
    )
    .bind(&[now.as_str().into(), membership_id.into(), invite_id.into()])?
    .run()
    .await?;
    Ok(())
}

/// Insert a new invite code (admin action).
pub async fn insert(
    db: &D1Database,
    id: &str,
    community_id: &str,
    code_hmac: &str,
    created_by_membership_id: &str,
    expires_at: &str,
    grants_role: &str,
) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "INSERT INTO invite_codes \
         (id, community_id, code_hmac, created_by_membership_id, expires_at, grants_role, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&[
        id.into(),
        community_id.into(),
        code_hmac.into(),
        created_by_membership_id.into(),
        expires_at.into(),
        grants_role.into(),
        now.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}
