//! Session table access — RFC-003 / RFC-002.
//!
//! Sessions are stored as HMAC hashes; the plaintext secret lives only in the
//! cookie.  All writes are parameterized.  Session TTL is set from
//! `zinnias_ciao_contracts::SESSION_TTL_SECONDS` and is NEVER derived from an upstream
//! token exp (regression note, RFC-003 §8).

use worker::{D1Database, Result};

use crate::db::now_utc;

/// RFC-081 §2 / Handoff 054 §5.4: `sessions.provenance` has no `CHECK`
/// constraint — the one-time pre-deployment schema exception (RFC-081
/// §1.2a) is spent, so one cannot be added now. Without this type, a
/// typo'd literal at a minting site is a non-null string, which passes
/// `authz::decide_membership_scope`'s null check and carries no
/// `scope_community_id` — silently treated as an unscoped, first-class
/// session. This type makes the typo a compile error instead of a
/// runtime fact: every minting site writes through `as_str()`, so the
/// database can only ever receive one of these three exact strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionProvenance {
    InviteRedemption,
    Relink,
    ExternalIdentity,
}

impl SessionProvenance {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::InviteRedemption => "invite_redemption",
            Self::Relink => "relink",
            Self::ExternalIdentity => "external_identity",
        }
    }
}

pub struct SessionRow {
    pub id: String,
    pub user_id: String,
    /// RFC-081 §2 / Handoff 048: how this session was minted
    /// (`invite_redemption` or `relink`). Every session created after
    /// migration 0012 has one; authorization refuses NULL (§7.3 — a
    /// pre-cutover session, all of which migration 0012 revoked outright).
    pub provenance: Option<String>,
    /// RFC-081 §2.1a: the community that granted this session, for
    /// relink-derived sessions only. NULL for first-class
    /// (invite-redemption) sessions, which are not community-bound.
    pub scope_community_id: Option<String>,
    /// RFC-080 §6 / Handoff 055: when this session last actually
    /// authenticated — distinct from `created_at`, which a future
    /// rotation (Slice 5b) will leave behind. NULL for every session
    /// minted before migration 0015; the step-up predicate
    /// (`authz::is_fresh_for_account_operations`) treats NULL as not
    /// fresh, the same fail-closed treatment NULL `provenance` gets.
    pub authenticated_at: Option<String>,
}

/// Look up a session by its HMAC.
/// Returns `None` if missing, expired, or revoked.
pub async fn find_active(db: &D1Database, session_hmac: &str) -> Result<Option<SessionRow>> {
    let now = now_utc();
    let row = db
        .prepare(
            "SELECT id, user_id, provenance, scope_community_id, authenticated_at \
             FROM sessions \
             WHERE session_hmac = ?1 \
               AND revoked_at IS NULL \
               AND expires_at > ?2 \
             LIMIT 1",
        )
        .bind(&[session_hmac.into(), now.as_str().into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(SessionRow {
            id: v.get("id")?.as_str()?.to_owned(),
            user_id: v.get("user_id")?.as_str()?.to_owned(),
            provenance: v
                .get("provenance")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned()),
            scope_community_id: v
                .get("scope_community_id")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned()),
            authenticated_at: v
                .get("authenticated_at")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned()),
        })
    }))
}

/// Revoke a session (logout / admin incident).
pub async fn revoke(db: &D1Database, session_id: &str) -> Result<()> {
    let now = now_utc();
    db.prepare("UPDATE sessions SET revoked_at = ?1 WHERE id = ?2")
        .bind(&[now.as_str().into(), session_id.into()])?
        .run()
        .await?;
    Ok(())
}

/// Touch `last_seen_at` (periodic, not on every request — guards privacy).
/// RFC-038: `last_seen_at` remains available but is deliberately not
/// written per-request under the fixed-window decision — available, not
/// wired to a periodic caller. Handoff 044 audit verdict: `deliberate`.
#[allow(dead_code)]
pub async fn touch(db: &D1Database, session_id: &str) -> Result<()> {
    let now = now_utc();
    db.prepare("UPDATE sessions SET last_seen_at = ?1 WHERE id = ?2")
        .bind(&[now.as_str().into(), session_id.into()])?
        .run()
        .await?;
    Ok(())
}
