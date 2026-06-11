//! Audit log writer — RFC-014.
//!
//! Writes structured metadata; NEVER logs full note bodies or secret values.
//! Every log line carries the request_id for cross-worker correlation.

use crate::crypto::random_token;
use crate::db::now_utc;
use worker::{D1Database, Result, console_log};

pub async fn write(
    db: &D1Database,
    request_id: &str,
    community_id: Option<&str>,
    actor_membership_id: Option<&str>,
    target_kind: &str,
    target_id: Option<&str>,
    action: &str,
    metadata: Option<serde_json::Value>,
) -> Result<()> {
    let id = &random_token()[..16]; // short audit ID
    let now = now_utc();

    // Redact any keys that might carry sensitive content before storing.
    let safe_meta = metadata.map(|mut v| {
        redact_sensitive_keys(&mut v);
        v.to_string()
    });

    db.prepare(
        "INSERT INTO audit_log \
         (id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(&[
        id.into(),
        community_id.unwrap_or("").into(),
        actor_membership_id.unwrap_or("").into(),
        target_kind.into(),
        target_id.unwrap_or("").into(),
        action.into(),
        safe_meta.as_deref().unwrap_or("{}").into(),
        now.as_str().into(),
    ])?
    .run()
    .await?;

    console_log!(
        "[{}] audit: action={} target={}:{} actor={} community={}",
        request_id,
        action,
        target_kind,
        target_id.unwrap_or("-"),
        actor_membership_id.unwrap_or("-"),
        community_id.unwrap_or("-"),
    );

    Ok(())
}

/// Remove keys that must never appear in the audit log (RFC-014).
fn redact_sensitive_keys(v: &mut serde_json::Value) {
    const BLOCKED: &[&str] = &[
        "note",
        "body",
        "secret",
        "token",
        "password",
        "session_hmac",
        "code_hmac",
        "pepper",
    ];
    if let Some(obj) = v.as_object_mut() {
        for key in BLOCKED {
            obj.remove(*key);
        }
    }
}
