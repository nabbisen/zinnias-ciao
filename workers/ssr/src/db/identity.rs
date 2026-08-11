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
