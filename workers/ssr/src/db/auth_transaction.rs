//! `auth_transactions` table access — RFC-080 §5.
//!
//! Handoff 053 §4: no callers yet. Slice 4b (the callback route) is the
//! first caller, where a transaction is created before the redirect out
//! and consumed on the callback in. Each item below trips the dead-code
//! lint until then — expected, not a defect; see the item-level
//! `#[allow(dead_code)]` on each, naming Slice 4b as the arriving caller.

use worker::{D1Database, Result};

#[allow(dead_code)] // Slice 4b: the callback route reads these fields to complete the exchange.
pub struct AuthTransactionRow {
    pub id: String,
    pub action: String,
    pub identity_namespace_id: String,
    pub nonce_hmac: String,
    pub pkce_verifier: String,
    pub initiating_session_provenance: Option<String>,
    pub invite_reference: Option<String>,
    pub callback_uri: String,
    pub return_to: Option<String>,
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)] // Slice 4b: created before the redirect out to the provider.
pub async fn insert_required(
    db: &D1Database,
    id: &str,
    lookup_key_hmac: &str,
    action: &str,
    identity_namespace_id: &str,
    nonce_hmac: &str,
    pkce_verifier: &str,
    initiating_session_provenance: Option<&str>,
    invite_reference: Option<&str>,
    callback_uri: &str,
    return_to: Option<&str>,
    created_at: &str,
    expires_at: &str,
) -> Result<()> {
    db.prepare(
        "INSERT INTO auth_transactions \
         (id, lookup_key_hmac, action, identity_namespace_id, nonce_hmac, \
          pkce_verifier, initiating_session_provenance, invite_reference, \
          callback_uri, return_to, created_at, expires_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind(&[
        id.into(),
        lookup_key_hmac.into(),
        action.into(),
        identity_namespace_id.into(),
        nonce_hmac.into(),
        pkce_verifier.into(),
        initiating_session_provenance.into(),
        invite_reference.into(),
        callback_uri.into(),
        return_to.into(),
        created_at.into(),
        expires_at.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Resolve an unconsumed, unexpired transaction by the HMAC of the `state`
/// value the callback presented. Finding a row here *is* the state check
/// (migration 0014's own design note); there is no separate comparison.
#[allow(dead_code)] // Slice 4b: the callback route's first lookup.
pub async fn find_active_by_lookup_key_hmac(
    db: &D1Database,
    lookup_key_hmac: &str,
    now: &str,
) -> Result<Option<AuthTransactionRow>> {
    let row = db
        .prepare(
            "SELECT id, action, identity_namespace_id, nonce_hmac, pkce_verifier, \
                    initiating_session_provenance, invite_reference, callback_uri, return_to \
             FROM auth_transactions \
             WHERE lookup_key_hmac = ?1 AND consumed_at IS NULL AND expires_at > ?2 \
             LIMIT 1",
        )
        .bind(&[lookup_key_hmac.into(), now.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(AuthTransactionRow {
            id: v.get("id")?.as_str()?.to_owned(),
            action: v.get("action")?.as_str()?.to_owned(),
            identity_namespace_id: v.get("identity_namespace_id")?.as_str()?.to_owned(),
            nonce_hmac: v.get("nonce_hmac")?.as_str()?.to_owned(),
            pkce_verifier: v.get("pkce_verifier")?.as_str()?.to_owned(),
            initiating_session_provenance: v
                .get("initiating_session_provenance")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned()),
            invite_reference: v
                .get("invite_reference")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned()),
            callback_uri: v.get("callback_uri")?.as_str()?.to_owned(),
            return_to: v
                .get("return_to")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned()),
        })
    }))
}

/// Atomically mark a transaction consumed. Single-use is the `WHERE
/// consumed_at IS NULL` guard, matching invite_codes/membership_relink_codes:
/// a second attempt against an already-consumed (or expired, or
/// nonexistent) row affects zero rows rather than racing with the first.
/// Returns `true` only if this call was the one that consumed it —
/// `false` means replay, expiry, or an unknown id, and the caller must
/// treat all three identically (a stale or reused transaction is refused,
/// not diagnosed to the caller).
#[allow(dead_code)] // Slice 4b: called once the callback's token has verified.
pub async fn consume_required(db: &D1Database, id: &str, now: &str) -> Result<bool> {
    let result = db
        .prepare(
            "UPDATE auth_transactions SET consumed_at = ?1 \
             WHERE id = ?2 AND consumed_at IS NULL AND expires_at > ?1",
        )
        .bind(&[now.into(), id.into()])?
        .run()
        .await?;

    let changed = result
        .meta()
        .ok()
        .flatten()
        .and_then(|m| m.changes)
        .unwrap_or(0);
    Ok(changed == 1)
}
