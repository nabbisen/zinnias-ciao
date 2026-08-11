//! Invite-code table access — RFC-003 / RFC-002.
//!
//! Codes are stored as HMAC-SHA256(pepper, normalize(code)).
//! All state changes (used, revoked) are soft — no hard deletes.

use crate::audit::{self, AuditAction, AuditMetadata};
use crate::db::now_utc;
use worker::{D1Database, Result};
use zinnias_ciao_contracts::SESSION_TTL_SECONDS;

pub struct InviteRow {
    pub id: String,
    pub community_id: String,
    /// Role to grant the joining user — 'admin' or 'member'.
    pub grants_role: String,
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
            id: v.get("id")?.as_str()?.to_owned(),
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            grants_role: v
                .get("grants_role")
                .and_then(|x| x.as_str())
                .unwrap_or("member")
                .to_owned(),
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
            id: v.get("id")?.as_str()?.to_owned(),
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            grants_role: v
                .get("grants_role")
                .and_then(|x| x.as_str())
                .unwrap_or("member")
                .to_owned(),
        })
    }))
}

pub async fn claim_is_still_eligible(
    db: &D1Database,
    invite_id: &str,
    community_id: &str,
    grants_role: &str,
) -> Result<bool> {
    let now = now_utc();
    let row = db
        .prepare(
            "SELECT 1 AS eligible FROM invite_codes i \
             JOIN communities c ON c.id=i.community_id \
             WHERE i.id=?1 AND i.community_id=?2 AND i.grants_role=?3 \
               AND i.used_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>?4 \
               AND c.is_active=1 LIMIT 1",
        )
        .bind(&[
            invite_id.into(),
            community_id.into(),
            grants_role.into(),
            now.as_str().into(),
        ])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.is_some())
}

#[allow(clippy::too_many_arguments)]
pub async fn redeem_required(
    db: &D1Database,
    request_id: &str,
    invite_id: &str,
    community_id: &str,
    grants_role: &str,
    user_id: &str,
    membership_id: &str,
    display_name: &str,
    session_id: &str,
    session_hmac: &str,
) -> Result<()> {
    let now = now_utc();
    let session_expires_at = crate::db::add_seconds_to_now(SESSION_TTL_SECONDS);
    let claim = db
        .prepare(
            "UPDATE invite_codes SET used_at=?1 \
             WHERE id=?2 AND community_id=?3 AND grants_role=?4 \
               AND used_at IS NULL AND revoked_at IS NULL AND expires_at>?1 \
               AND EXISTS (SELECT 1 FROM communities c \
                           WHERE c.id=?3 AND c.is_active=1)",
        )
        .bind(&[
            now.as_str().into(),
            invite_id.into(),
            community_id.into(),
            grants_role.into(),
        ])?;
    let user = db
        .prepare("INSERT INTO users (id, created_at) VALUES (?1, ?2)")
        .bind(&[user_id.into(), now.as_str().into()])?;
    let membership = db
        .prepare(
            "INSERT INTO community_memberships \
             (id, community_id, user_id, role, display_name, joined_at) \
             SELECT ?1, i.community_id, ?2, i.grants_role, ?3, ?4 \
             FROM invite_codes i \
             WHERE i.id=?5 AND i.community_id=?6 AND i.grants_role=?7 \
               AND i.used_at=?4 AND i.revoked_at IS NULL \
               AND EXISTS (SELECT 1 FROM users u WHERE u.id=?2)",
        )
        .bind(&[
            membership_id.into(),
            user_id.into(),
            display_name.into(),
            now.as_str().into(),
            invite_id.into(),
            community_id.into(),
            grants_role.into(),
        ])?;
    let link = db
        .prepare(
            "UPDATE invite_codes SET used_by_membership_id=?1 \
             WHERE id=?2 AND community_id=?3 AND used_at=?4 \
               AND used_by_membership_id IS NULL \
               AND EXISTS (SELECT 1 FROM community_memberships m \
                           WHERE m.id=?1 AND m.community_id=?3 \
                             AND m.user_id=?5 AND m.removed_at IS NULL)",
        )
        .bind(&[
            membership_id.into(),
            invite_id.into(),
            community_id.into(),
            now.as_str().into(),
            user_id.into(),
        ])?;
    // Handoff 048 (RFC-081 §2): first-class session — provenance
    // 'invite_redemption', scope_community_id left NULL. Not community-bound.
    // Handoff 054 §5.4: written through SessionProvenance, not a literal.
    let session = db
        .prepare(
            "INSERT INTO sessions \
             (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance) \
             SELECT ?1, ?2, ?3, ?4, ?5, ?4, ?8 \
             WHERE EXISTS (SELECT 1 FROM community_memberships m \
                           WHERE m.id=?6 AND m.community_id=?7 \
                             AND m.user_id=?2 AND m.removed_at IS NULL)",
        )
        .bind(&[
            session_id.into(),
            user_id.into(),
            session_hmac.into(),
            now.as_str().into(),
            session_expires_at.as_str().into(),
            membership_id.into(),
            community_id.into(),
            crate::db::session::SessionProvenance::InviteRedemption
                .as_str()
                .into(),
        ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(membership_id),
        Some(invite_id),
        AuditAction::InviteCodeRedeemed,
        AuditMetadata::None,
    )?;
    audit::execute_asserted_required(
        db,
        claim,
        vec![user, membership, link, session],
        vec![],
        &record,
    )
    .await?;
    Ok(())
}

/// Revoke an unused invite code (admin action — sets revoked_at).
pub async fn revoke_required(
    db: &D1Database,
    request_id: &str,
    invite_id: &str,
    community_id: &str,
    actor_membership_id: &str,
) -> Result<bool> {
    let now = now_utc();
    let mutation = db
        .prepare(
            "UPDATE invite_codes \
         SET revoked_at = ?1 \
         WHERE id = ?2 AND community_id = ?3 \
           AND used_at IS NULL AND revoked_at IS NULL \
           AND EXISTS ( \
             SELECT 1 FROM community_memberships \
             WHERE id = ?4 AND community_id = ?3 \
               AND role = 'admin' AND removed_at IS NULL \
           )",
        )
        .bind(&[
            now.as_str().into(),
            invite_id.into(),
            community_id.into(),
            actor_membership_id.into(),
        ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(actor_membership_id),
        Some(invite_id),
        AuditAction::InviteCodeRevoked,
        AuditMetadata::None,
    )?;
    audit::execute_required(db, mutation, &record).await
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

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(InviteMetaRow {
                id: v.get("id")?.as_str()?.to_owned(),
                expires_at: v.get("expires_at")?.as_str()?.to_owned(),
                grants_role: v
                    .get("grants_role")
                    .and_then(|x| x.as_str())
                    .unwrap_or("member")
                    .to_owned(),
            })
        })
        .collect())
}

/// Insert a new invite code (admin action).
#[allow(clippy::too_many_arguments)]
pub async fn insert_required(
    db: &D1Database,
    request_id: &str,
    id: &str,
    community_id: &str,
    code_hmac: &str,
    created_by_membership_id: &str,
    expires_at: &str,
    grants_role: &str,
) -> Result<bool> {
    let now = now_utc();
    let mutation = db.prepare(
        "INSERT INTO invite_codes \
         (id, community_id, code_hmac, created_by_membership_id, expires_at, grants_role, created_at) \
         SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7 \
         WHERE EXISTS ( \
           SELECT 1 FROM community_memberships \
           WHERE id = ?4 AND community_id = ?2 \
             AND role = 'admin' AND removed_at IS NULL \
         )",
    )
    .bind(&[
        id.into(),
        community_id.into(),
        code_hmac.into(),
        created_by_membership_id.into(),
        expires_at.into(),
        grants_role.into(),
        now.as_str().into(),
    ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(created_by_membership_id),
        Some(id),
        AuditAction::InviteCodeGenerated,
        AuditMetadata::None,
    )?;
    audit::execute_required(db, mutation, &record).await
}
