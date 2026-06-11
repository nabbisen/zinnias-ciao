//! Session table access — RFC-003 / RFC-002.
//!
//! Sessions are stored as HMAC hashes; the plaintext secret lives only in the
//! cookie.  All writes are parameterized.  Session TTL is set from
//! `zinnias_ciao_contracts::SESSION_TTL_SECONDS` and is NEVER derived from an upstream
//! token exp (regression note, RFC-003 §8).

use worker::{D1Database, Result};
use zinnias_ciao_contracts::SESSION_TTL_SECONDS;

use crate::db::{add_seconds_to_now, now_utc};

pub struct SessionRow {
    pub id: String,
    pub user_id: String,
    pub expires_at: String,
}

/// Look up a session by its HMAC.
/// Returns `None` if missing, expired, or revoked.
pub async fn find_active(db: &D1Database, session_hmac: &str) -> Result<Option<SessionRow>> {
    let now = now_utc();
    let row = db
        .prepare(
            "SELECT id, user_id, expires_at \
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
            expires_at: v.get("expires_at")?.as_str()?.to_owned(),
        })
    }))
}

/// Insert a new session row.
/// `session_hmac` is HMAC-SHA256(pepper, secret); never the raw secret.
pub async fn insert(db: &D1Database, id: &str, user_id: &str, session_hmac: &str) -> Result<()> {
    let now = now_utc();
    // Session lifetime set from constant only — never from a token exp (RFC-003).
    let expires_at = add_seconds_to_now(SESSION_TTL_SECONDS);
    db.prepare(
        "INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?4)",
    )
    .bind(&[
        id.into(),
        user_id.into(),
        session_hmac.into(),
        now.as_str().into(),
        expires_at.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
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
pub async fn touch(db: &D1Database, session_id: &str) -> Result<()> {
    let now = now_utc();
    db.prepare("UPDATE sessions SET last_seen_at = ?1 WHERE id = ?2")
        .bind(&[now.as_str().into(), session_id.into()])?
        .run()
        .await?;
    Ok(())
}
