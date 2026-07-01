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
/// Atomically mark an invite used, but only if it is still valid (unused,
/// unrevoked, unexpired). Returns `true` if THIS call performed the transition
/// (changed exactly one row), `false` if the invite was already used/revoked/
/// expired. Callers must check the boolean to enforce one-time use under races.
pub async fn mark_used(db: &D1Database, invite_id: &str, membership_id: &str) -> Result<bool> {
    let now = now_utc();
    let res = db.prepare(
        "UPDATE invite_codes \
         SET used_at = ?1, used_by_membership_id = ?2 \
         WHERE id = ?3 \
           AND used_at IS NULL \
           AND revoked_at IS NULL \
           AND expires_at > ?1",
    )
    .bind(&[now.as_str().into(), membership_id.into(), invite_id.into()])?
    .run()
    .await?;
    let changed = res.meta().ok().flatten().and_then(|m| m.changes).unwrap_or(0);
    Ok(changed == 1)
}

/// Revoke an unused invite code (admin action — sets revoked_at).
pub async fn revoke(db: &D1Database, invite_id: &str, community_id: &str) -> Result<()> {
    let now = now_utc();
    // Scoped to community_id to prevent cross-community revocation.
    db.prepare(
        "UPDATE invite_codes \
         SET revoked_at = ?1 \
         WHERE id = ?2 AND community_id = ?3 \
           AND used_at IS NULL AND revoked_at IS NULL",
    )
    .bind(&[now.as_str().into(), invite_id.into(), community_id.into()])?
    .run()
    .await?;
    Ok(())
}

/// Active (unused, unrevoked, unexpired) invite codes for a community.
/// Returns (id, expires_at, grants_role) ordered newest first.
/// Code HMACs are never returned — admins see only metadata.
pub struct InviteMetaRow {
    pub id: String,
    pub expires_at: String,
    pub grants_role: String,
}

pub async fn list_active_for_community(
    db: &D1Database,
    community_id: &str,
) -> Result<Vec<InviteMetaRow>> {
    let now = now_utc();
    let rows = db
        .prepare(
            "SELECT id, expires_at, grants_role \
             FROM invite_codes \
             WHERE community_id = ?1 \
               AND used_at IS NULL \
               AND revoked_at IS NULL \
               AND expires_at > ?2 \
             ORDER BY expires_at DESC \
             LIMIT 20",
        )
        .bind(&[community_id.into(), now.as_str().into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows.into_iter().filter_map(|v| {
        Some(InviteMetaRow {
            id:          v.get("id")?.as_str()?.to_owned(),
            expires_at:  v.get("expires_at")?.as_str()?.to_owned(),
            grants_role: v.get("grants_role")
                          .and_then(|x| x.as_str())
                          .unwrap_or("member")
                          .to_owned(),
        })
    }).collect())
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
