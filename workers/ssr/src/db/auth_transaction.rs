//! `auth_transactions` table access — RFC-080 §5.
//!
//! Handoff 053 §4: no callers yet. Slice 4b (the callback route) is the
//! first caller, where a transaction is created before the redirect out
//! and consumed on the callback in. Each item below trips the dead-code
//! lint until then — expected, not a defect; see the item-level
//! `#[allow(dead_code)]` on each, naming Slice 4b as the arriving caller.

use worker::{D1Database, Result};
use zinnias_ciao_contracts::SESSION_TTL_SECONDS;

use crate::audit::{self, AuditAction, AuditMetadata};
use crate::db::session::SessionProvenance;
use crate::db::{add_seconds_to_now, now_utc};

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
    // `Option<&str>::into()` produces a JS `undefined` for `None`, which D1
    // rejects outright ("Type 'undefined' not supported") rather than
    // treating it as a bindable NULL — the same pitfall
    // `db/event_template.rs`/`db/event_write.rs` already work around.
    let initiating_session_provenance_js = initiating_session_provenance
        .map(worker::wasm_bindgen::JsValue::from_str)
        .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
    let invite_reference_js = invite_reference
        .map(worker::wasm_bindgen::JsValue::from_str)
        .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
    let return_to_js = return_to
        .map(worker::wasm_bindgen::JsValue::from_str)
        .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
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
        initiating_session_provenance_js,
        invite_reference_js,
        callback_uri.into(),
        return_to_js,
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

/// RFC-080 §5.1 step 8, `sign_in` outcome: the identity already resolved
/// to `user_id` (Handoff 054 §5.6's decision already applied by the
/// caller — only an `active` identity reaches this function). Atomically:
/// touch `last_authenticated_at` (the guard — a concurrently revoked
/// identity changes zero rows here, and the session insert's own `WHERE
/// EXISTS` re-checks the same condition rather than trusting this blindly,
/// matching `db/invite.rs`/`db/relink.rs`'s established shape), issue the
/// session with `SessionProvenance::ExternalIdentity`, and audit.
pub async fn issue_sign_in_required(
    db: &D1Database,
    request_id: &str,
    identity_id: &str,
    user_id: &str,
    session_id: &str,
    session_hmac: &str,
) -> Result<()> {
    let now = now_utc();
    let session_expires_at = add_seconds_to_now(SESSION_TTL_SECONDS);
    let touch = db
        .prepare(
            "UPDATE user_identities SET last_authenticated_at = ?1 \
             WHERE id = ?2 AND user_id = ?3 AND status = 'active'",
        )
        .bind(&[now.as_str().into(), identity_id.into(), user_id.into()])?;
    let session = db
        .prepare(
            "INSERT INTO sessions \
             (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) \
             SELECT ?1, ?2, ?3, ?4, ?5, ?4, ?6, ?4 \
             WHERE EXISTS (SELECT 1 FROM user_identities \
                           WHERE id = ?7 AND user_id = ?2 AND status = 'active')",
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
    let record = audit::required_record(
        request_id,
        None,
        None,
        None,
        AuditAction::ExternalSessionIssued,
        AuditMetadata::None,
    )?;
    audit::execute_asserted_required(db, touch, vec![session], vec![], &record).await?;
    Ok(())
}

/// RFC-080 §5.1 step 8, `join` outcome: an unrecognized identity claiming
/// an invite. Mirrors `db/invite.rs::redeem_required` exactly (the same
/// atomic claim-then-create shape, the same `AuditAction::InviteCodeRedeemed`
/// — structurally this *is* an invite redemption, and `sessions.provenance`
/// is what records that it arrived via external identity rather than a
/// typed-in code) with one addition: the `user_identities` link, inserted
/// only once the new user exists. RFC-080 §7 "no orphan users": if the
/// invite claim fails (already used, expired, revoked), every other
/// statement's own `WHERE EXISTS`/`FROM invite_codes i WHERE i.used_at=?`
/// guard means nothing is created — no user, no identity link, no
/// membership, no session.
#[allow(clippy::too_many_arguments)]
pub async fn issue_join_required(
    db: &D1Database,
    request_id: &str,
    invite_id: &str,
    community_id: &str,
    grants_role: &str,
    user_id: &str,
    membership_id: &str,
    display_name: &str,
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
    let identity = db
        .prepare(
            "INSERT INTO user_identities \
             (id, user_id, identity_namespace_id, subject_lookup, linked_at, status) \
             SELECT ?1, ?2, ?3, ?4, ?5, 'active' \
             WHERE EXISTS (SELECT 1 FROM users u WHERE u.id=?2)",
        )
        .bind(&[
            identity_id.into(),
            user_id.into(),
            identity_namespace_id.into(),
            subject_lookup.into(),
            now.as_str().into(),
        ])?;
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
    let session = db
        .prepare(
            "INSERT INTO sessions \
             (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) \
             SELECT ?1, ?2, ?3, ?4, ?5, ?4, ?6, ?4 \
             WHERE EXISTS (SELECT 1 FROM community_memberships m \
                           WHERE m.id=?7 AND m.community_id=?8 \
                             AND m.user_id=?2 AND m.removed_at IS NULL)",
        )
        .bind(&[
            session_id.into(),
            user_id.into(),
            session_hmac.into(),
            now.as_str().into(),
            session_expires_at.as_str().into(),
            SessionProvenance::ExternalIdentity.as_str().into(),
            membership_id.into(),
            community_id.into(),
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
        vec![user, identity, membership, link, session],
        vec![],
        &record,
    )
    .await?;
    Ok(())
}
