//! `user_identities` table access — RFC-080 §3.3.
//!
//! Handoff 050 §5.3: no callers yet. Slice 4 (the authentication
//! transaction / fake-issuer callback, RFC-080 §5/§10) is the first
//! caller, where a returning subject is looked up to decide link vs.
//! collision. Each item below trips the dead-code lint until then —
//! expected, not a defect; see the item-level `#[allow(dead_code)]` on
//! each, naming Slice 4 as the arriving caller. This module intentionally
//! contains nothing beyond what that one lookup needs.

use worker::{D1Database, Result};
use zinnias_ciao_contracts::SESSION_TTL_SECONDS;

use crate::audit::{self, AuditAction, AuditMetadata};
use crate::db::session::SessionProvenance;
use crate::db::{add_seconds_to_now, now_utc, session};

/// One row of `user_identities` (RFC-080 §3.3). `status` is returned
/// as-is, not filtered to `'active'` in the query below — whether a
/// `'revoked'` row should be treated as unlinked or surfaced distinctly
/// is a collision-handling decision that belongs to Slice 4, not to this
/// accessor.
///
/// **Decided in Handoff 053 (Slice 4a):** a revoked identity authenticates
/// nobody and is indistinguishable to the caller from one that was never
/// linked — see `identity::identity_lookup_is_authenticatable`, which is
/// where that decision is actually enforced. This accessor still returns
/// the row unfiltered, on purpose: the decision belongs one layer up, not
/// baked into the query, so a future caller with a genuinely different
/// need (e.g. showing an admin that an identity exists but is revoked)
/// is not blocked by this function's own choice.
#[allow(dead_code)] // Slice 4: read by the authentication callback (RFC-080 §5).
pub struct UserIdentityRow {
    pub id: String,
    pub user_id: String,
    pub identity_namespace_id: String,
    pub linked_at: String,
    pub last_authenticated_at: Option<String>,
    pub status: String,
}

/// Look up a linked identity by the exact key the table's
/// `UNIQUE(identity_namespace_id, subject_lookup)` constraint enforces.
/// `subject_lookup` must already be the keyed digest
/// (`crypto::subject_lookup`) — this function never sees a raw subject.
#[allow(dead_code)] // Slice 4: the authentication callback's first read of user_identities.
pub async fn find_by_subject_lookup(
    db: &D1Database,
    identity_namespace_id: &str,
    subject_lookup: &str,
) -> Result<Option<UserIdentityRow>> {
    let row = db
        .prepare(
            "SELECT id, user_id, identity_namespace_id, linked_at, \
                    last_authenticated_at, status \
             FROM user_identities \
             WHERE identity_namespace_id = ?1 AND subject_lookup = ?2 \
             LIMIT 1",
        )
        .bind(&[identity_namespace_id.into(), subject_lookup.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(UserIdentityRow {
            id: v.get("id")?.as_str()?.to_owned(),
            user_id: v.get("user_id")?.as_str()?.to_owned(),
            identity_namespace_id: v.get("identity_namespace_id")?.as_str()?.to_owned(),
            linked_at: v.get("linked_at")?.as_str()?.to_owned(),
            last_authenticated_at: v
                .get("last_authenticated_at")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned()),
            status: v.get("status")?.as_str()?.to_owned(),
        })
    }))
}

/// The account surface's own row shape (RFC-080 §6 / RFC-081 §6, Handoff
/// 055 §5.4) — deliberately narrower than [`UserIdentityRow`], not a
/// filtered view of it: this query never `SELECT`s `subject_lookup`, so
/// there is no digest for a rendering bug to ever leak, structurally
/// rather than by caller discipline. Same reasoning as
/// `db/invite.rs::InviteMetaRow` ("Code HMACs are never returned").
pub struct LinkedIdentitySummary {
    pub identity_namespace_id: String,
    pub linked_at: String,
}

/// Every `active` identity linked to `user_id`, oldest first. `revoked`
/// rows are excluded — matching `identity::identity_lookup_is_authenticatable`'s
/// existing decision that a revoked identity is, to every caller, as good
/// as never linked.
pub async fn list_active_for_user(
    db: &D1Database,
    user_id: &str,
) -> Result<Vec<LinkedIdentitySummary>> {
    let rows = db
        .prepare(
            "SELECT identity_namespace_id, linked_at \
             FROM user_identities \
             WHERE user_id = ?1 AND status = 'active' \
             ORDER BY linked_at ASC",
        )
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(LinkedIdentitySummary {
                identity_namespace_id: v.get("identity_namespace_id")?.as_str()?.to_owned(),
                linked_at: v.get("linked_at")?.as_str()?.to_owned(),
            })
        })
        .collect())
}

/// RFC-081 §4 / Handoff 056 §5.1: link a verified external identity to an
/// already-known `user_id`, atomically with session rotation. Callers must
/// check `find_by_subject_lookup` first and treat `Some(_)` as a
/// collision (Handoff 056 §5.4) — this function's own `claim` (the
/// identity insert) is the second, authoritative check: its
/// `WHERE NOT EXISTS` guard means a race loses zero rows here, never a
/// `UNIQUE` constraint error, matching every other claim-then-create shape
/// in this codebase (`db/invite.rs`, `db/relink.rs`). A claim that fails
/// despite the caller's own earlier check is therefore a genuine race, not
/// an expected outcome — `execute_asserted_required` correctly treats it
/// as a Class A failure rather than a graceful collision, which is why
/// the ordinary-collision path must never reach this function at all.
///
/// Additive by construction: the only statement here that is not an
/// `INSERT` is the revoke-others `UPDATE`, which touches `sessions`, never
/// `user_identities` — nothing in this function can ever remove or
/// deactivate an existing identity link.
#[allow(clippy::too_many_arguments)]
pub async fn link_required(
    db: &D1Database,
    request_id: &str,
    user_id: &str,
    identity_id: &str,
    identity_namespace_id: &str,
    subject_lookup: &str,
    session_id: &str,
    session_hmac: &str,
) -> Result<()> {
    let now = now_utc();
    let session_expires_at = add_seconds_to_now(SESSION_TTL_SECONDS);
    let claim = db
        .prepare(
            "INSERT INTO user_identities \
             (id, user_id, identity_namespace_id, subject_lookup, linked_at, status) \
             SELECT ?1, ?2, ?3, ?4, ?5, 'active' \
             WHERE NOT EXISTS (SELECT 1 FROM user_identities \
                               WHERE identity_namespace_id=?3 AND subject_lookup=?4) \
               AND EXISTS (SELECT 1 FROM users u WHERE u.id=?2)",
        )
        .bind(&[
            identity_id.into(),
            user_id.into(),
            identity_namespace_id.into(),
            subject_lookup.into(),
            now.as_str().into(),
        ])?;
    let session_insert = db
        .prepare(
            "INSERT INTO sessions \
             (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) \
             SELECT ?1, ?2, ?3, ?4, ?5, ?4, ?6, ?4 \
             WHERE EXISTS (SELECT 1 FROM user_identities WHERE id=?7 AND user_id=?2)",
        )
        .bind(&[
            session_id.into(),
            user_id.into(),
            session_hmac.into(),
            now.as_str().into(),
            session_expires_at.as_str().into(),
            SessionProvenance::ExternalIdentity.as_str().into(),
            identity_id.into(),
        ])?;
    let revoke_others = session::revoke_others_statement(db, user_id, session_id, &now)?;
    let record = audit::required_record(
        request_id,
        None,
        None,
        None,
        AuditAction::ExternalIdentityLinked,
        AuditMetadata::None,
    )?;
    audit::execute_asserted_required(
        db,
        claim,
        vec![session_insert],
        vec![revoke_others],
        &record,
    )
    .await?;
    Ok(())
}
