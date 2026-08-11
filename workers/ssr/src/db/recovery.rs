//! Account recovery credential table access — RFC-081 §3 (Handoff 057).
//!
//! Modeled directly on `db/relink.rs`: HMAC at rest (AD-3), a claim-then-
//! write shape for the anonymous consumption route, and the same
//! "re-check after a failed claim, distinguish a genuine race from an
//! already-known-invalid code" caller pattern `handlers/relink.rs::post_relink`
//! already establishes. Unlike a relink code, `expires_at` is nullable and
//! this module never sets it — see migration 0017's comment for why a
//! non-expiring recovery credential is the correct choice here.

use worker::{D1Database, Result};
use zinnias_ciao_contracts::SESSION_TTL_SECONDS;

use crate::audit::{self, AuditAction, AuditMetadata};
use crate::db::session::SessionProvenance;
use crate::db::{add_seconds_to_now, now_utc};

pub struct RecoveryTargetRow {
    pub id: String,
    pub user_id: String,
}

/// Resolve a valid code to its owning principal. "Valid" is the single
/// definition every consumption-failure cause collapses into — unknown,
/// consumed, revoked, and expired are indistinguishable here, by
/// construction, not by the caller hiding a more specific answer
/// (Handoff 057 §5.2 / §9: no distinct failure message per cause).
pub async fn find_valid_by_hmac(
    db: &D1Database,
    code_hmac: &str,
) -> Result<Option<RecoveryTargetRow>> {
    let now = now_utc();
    let row = db
        .prepare(
            "SELECT id, user_id FROM account_recovery_credentials \
             WHERE code_hmac = ?1 \
               AND consumed_at IS NULL \
               AND revoked_at IS NULL \
               AND (expires_at IS NULL OR expires_at > ?2) \
             LIMIT 1",
        )
        .bind(&[code_hmac.into(), now.as_str().into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(RecoveryTargetRow {
            id: v.get("id")?.as_str()?.to_owned(),
            user_id: v.get("user_id")?.as_str()?.to_owned(),
        })
    }))
}

/// RFC-081 §3.1 / Handoff 057 §5.1: does `user_id` currently hold a usable
/// (unconsumed, unrevoked, unexpired) recovery credential at all? The
/// account surface's own existence-only disclosure — never the code, the
/// HMAC, or which one.
pub async fn exists_for_user(db: &D1Database, user_id: &str) -> Result<bool> {
    let now = now_utc();
    let row = db
        .prepare(
            "SELECT 1 FROM account_recovery_credentials \
             WHERE user_id = ?1 AND consumed_at IS NULL AND revoked_at IS NULL \
               AND (expires_at IS NULL OR expires_at > ?2) \
             LIMIT 1",
        )
        .bind(&[user_id.into(), now.as_str().into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.is_some())
}

/// RFC-081 §3.1 / Handoff 057 §5.1: issue the member's recovery credential
/// the first time they ever hold a usable authentication method — called
/// by `handlers/identity/mod.rs::link_outcome` immediately after a
/// successful `db::identity::link_required`, never bundled into that same
/// batch: this claim's own guard (no prior credential row for `user_id`,
/// AND at least one active identity now exists) is independently
/// race-safe and self-contained, so a second, closely-following call
/// keeps the logic simple rather than threading a result index out of
/// `execute_asserted_required`'s generic batch — see the review request
/// for why that tradeoff was made deliberately, not by default.
///
/// Returns whether a credential was issued *by this call* — `false` means
/// the member already had one (an ordinary second-or-later link), not a
/// failure. The caller must reveal the plaintext code only when this
/// returns `true`; nothing about this call itself returns the plaintext,
/// since it never receives anything but the already-computed HMAC.
pub async fn issue_at_first_link_required(
    db: &D1Database,
    request_id: &str,
    user_id: &str,
    credential_id: &str,
    code_hmac: &str,
) -> Result<bool> {
    let now = now_utc();
    let claim = db
        .prepare(
            "INSERT INTO account_recovery_credentials \
             (id, user_id, code_hmac, created_at, expires_at) \
             SELECT ?1, ?2, ?3, ?4, NULL \
             WHERE NOT EXISTS (SELECT 1 FROM account_recovery_credentials WHERE user_id = ?2) \
               AND EXISTS (SELECT 1 FROM user_identities \
                           WHERE user_id = ?2 AND status = 'active')",
        )
        .bind(&[
            credential_id.into(),
            user_id.into(),
            code_hmac.into(),
            now.as_str().into(),
        ])?;
    let record = audit::required_record(
        request_id,
        None,
        None,
        None,
        AuditAction::RecoveryCredentialIssued,
        AuditMetadata::None,
    )?;
    audit::execute_required(db, claim, &record).await
}

/// RFC-081 §3.1 / Handoff 057 §5.1: regenerate — revokes whatever
/// credential is currently active for `user_id` (there is at most one, by
/// this same discipline) and issues a new one, batched together so a
/// member can never end up holding two. The revoke has no target id: it
/// unconditionally clears every still-active row for `user_id`, which is
/// simpler than threading the previous credential's id through and
/// produces the same result as long as the "never hold two" invariant
/// already held going in.
pub async fn regenerate_required(
    db: &D1Database,
    request_id: &str,
    user_id: &str,
    new_credential_id: &str,
    new_code_hmac: &str,
) -> Result<bool> {
    let now = now_utc();
    let revoke_previous = db
        .prepare(
            "UPDATE account_recovery_credentials SET revoked_at = ?1 \
             WHERE user_id = ?2 AND consumed_at IS NULL AND revoked_at IS NULL",
        )
        .bind(&[now.as_str().into(), user_id.into()])?;
    let issue_new = db
        .prepare(
            "INSERT INTO account_recovery_credentials \
             (id, user_id, code_hmac, created_at, expires_at) \
             VALUES (?1, ?2, ?3, ?4, NULL)",
        )
        .bind(&[
            new_credential_id.into(),
            user_id.into(),
            new_code_hmac.into(),
            now.as_str().into(),
        ])?;
    let record = audit::required_record(
        request_id,
        None,
        None,
        None,
        AuditAction::RecoveryCredentialRegenerated,
        AuditMetadata::None,
    )?;
    audit::execute_required_tail(db, vec![revoke_previous, issue_new], &record).await
}

/// RFC-081 §3 / Handoff 057 §5.2: consume a recovery credential, minting
/// an account-tier, fresh, unscoped session for its owning principal.
/// Callers must call `find_valid_by_hmac` first; `target` is that row.
/// This function's own claim re-checks the same conditions
/// (`id`/`user_id` plus not-consumed/not-revoked/not-expired) as a
/// defense against a race between the two reads — the exact shape
/// `db/relink.rs::redeem_required` already establishes for its own
/// anonymous consumption route. A claim that fails despite the caller's
/// own earlier check is therefore either a genuine race (treat as a Class
/// A failure, matching `link_required`'s identical reasoning) or the code
/// having been consumed/revoked in the meantime (the caller re-checks
/// `find_valid_by_hmac` after an `Err` and treats `None` there as the
/// ordinary generic failure, never propagating the error in that case).
///
/// No other-session revocation here, deliberately: Handoff 057 §5.2 does
/// not ask for it (unlike unlink's explicit §5.3 requirement), and losing
/// provider access is an ordinary event, not one this package treats as
/// evidence of compromise.
pub async fn consume_required(
    db: &D1Database,
    request_id: &str,
    target: &RecoveryTargetRow,
    session_id: &str,
    session_hmac: &str,
) -> Result<()> {
    let now = now_utc();
    let session_expires_at = add_seconds_to_now(SESSION_TTL_SECONDS);
    let claim = db
        .prepare(
            "UPDATE account_recovery_credentials SET consumed_at = ?1 \
             WHERE id = ?2 AND user_id = ?3 \
               AND consumed_at IS NULL AND revoked_at IS NULL \
               AND (expires_at IS NULL OR expires_at > ?1)",
        )
        .bind(&[
            now.as_str().into(),
            target.id.as_str().into(),
            target.user_id.as_str().into(),
        ])?;
    let session = db
        .prepare(
            "INSERT INTO sessions \
             (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) \
             SELECT ?1, ?2, ?3, ?4, ?5, ?4, ?6, ?4 \
             WHERE EXISTS (SELECT 1 FROM account_recovery_credentials \
                           WHERE id = ?7 AND user_id = ?2 AND consumed_at = ?4)",
        )
        .bind(&[
            session_id.into(),
            target.user_id.as_str().into(),
            session_hmac.into(),
            now.as_str().into(),
            session_expires_at.as_str().into(),
            SessionProvenance::AccountRecovery.as_str().into(),
            target.id.as_str().into(),
        ])?;
    let record = audit::required_record(
        request_id,
        None,
        None,
        None,
        AuditAction::RecoveryCredentialConsumed,
        AuditMetadata::None,
    )?;
    audit::execute_asserted_required(db, claim, vec![session], vec![], &record).await?;
    Ok(())
}

/// RFC-081 §3.3 / Handoff 057 §5.3: "at least one other verified usable
/// method" — defined once, here, and embedded (with statement-local
/// placeholder tokens the caller supplies) into both
/// `db/identity.rs::unlink_required`'s guarded claim and its same-batch
/// revoke-others statement, so within one unlink attempt the two
/// statements can never disagree about the answer — the property Handoff
/// 057 §5.3's concurrency requirement depends on. A second
/// `user_identities` row for the same principal with `status = 'active'`
/// (excluding the identity being unlinked), OR an
/// `account_recovery_credentials` row that is unconsumed, unrevoked, and
/// unexpired.
///
/// `exclude_identity`/`user_id`/`now` are the placeholder tokens (e.g.
/// `"?1"`) the embedding statement wants substituted at each position —
/// only the condition's logic (table/column names, operators) is
/// centralized here; each call site keeps its own parameter numbering.
pub(crate) fn usable_method_exists_sql(exclude_identity: &str, user_id: &str, now: &str) -> String {
    format!(
        "(EXISTS (SELECT 1 FROM user_identities ui \
                  WHERE ui.user_id = {user_id} AND ui.id != {exclude_identity} \
                    AND ui.status = 'active') \
          OR EXISTS (SELECT 1 FROM account_recovery_credentials arc \
                     WHERE arc.user_id = {user_id} AND arc.consumed_at IS NULL \
                       AND arc.revoked_at IS NULL \
                       AND (arc.expires_at IS NULL OR arc.expires_at > {now})))"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usable_method_exists_sql_references_both_halves_of_the_definition() {
        let sql = usable_method_exists_sql("?1", "?2", "?3");
        assert!(
            sql.contains("user_identities") && sql.contains("status = 'active'"),
            "must check for a second active identity"
        );
        assert!(
            sql.contains("account_recovery_credentials")
                && sql.contains("consumed_at IS NULL")
                && sql.contains("revoked_at IS NULL"),
            "must check for an unconsumed, unrevoked recovery credential"
        );
        assert!(
            sql.contains("expires_at IS NULL OR"),
            "NULL expires_at must count as not-expired, not as expired"
        );
    }

    #[test]
    fn usable_method_exists_sql_excludes_the_identity_being_unlinked() {
        let sql = usable_method_exists_sql("?1", "?2", "?3");
        assert!(
            sql.contains("ui.id != ?1"),
            "the identity being unlinked must never count as its own remaining method"
        );
    }

    #[test]
    fn usable_method_exists_sql_substitutes_the_callers_own_placeholder_tokens() {
        let sql = usable_method_exists_sql("?9", "?8", "?7");
        assert!(sql.contains("?9") && sql.contains("?8") && sql.contains("?7"));
        assert!(!sql.contains("?1") && !sql.contains("?2") && !sql.contains("?3"));
    }
}
