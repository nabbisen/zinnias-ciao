//! Auth compatibility helpers.
//!
//! The current published `codlet` crate is runtime-neutral. This Worker keeps
//! the D1/KV integration in the service while preserving a small helper surface
//! for handlers that issue/consume form tokens and manage invite metadata.

use worker::{Env, Result};

pub use crate::form_token::ConsumeResult;

/// Issue a single-use CSRF form token.
pub async fn issue_token(
    env: &Env,
    user_id: &str,
    purpose: &str,
    bound_resource: Option<&str>,
) -> Result<String> {
    let pepper = crate::crypto::pepper(env)?;
    let db = env.d1("DB")?;
    crate::form_token::issue(&db, pepper.as_str(), user_id, purpose, bound_resource).await
}

/// Validate and consume a single-use CSRF form token.
///
/// Callers must match on the returned `ConsumeResult` explicitly
/// (`matches!(result, ConsumeResult::Replay(_))`) — never call `.is_some()`
/// on it, and never wrap it back into an `Option`. See
/// `form_token::consume_detailed`'s doc comment for why.
pub async fn consume_token(
    env: &Env,
    user_id: &str,
    purpose: &str,
    raw_token: &str,
    bound_resource: Option<&str>,
) -> Result<ConsumeResult> {
    let pepper = crate::crypto::pepper(env)?;
    let db = env.d1("DB")?;
    crate::form_token::consume_detailed(
        &db,
        pepper.as_str(),
        user_id,
        purpose,
        raw_token,
        bound_resource,
    )
    .await
}

/// Metadata for one active invite code.
pub struct InviteCodeMeta {
    pub id: String,
    /// ISO-8601 prefix for display.
    pub expires_at: String,
    /// "admin" or "member".
    pub grants_role: String,
}

/// List active invite codes for a community.
pub async fn list_active_invites(env: &Env, community_id: &str) -> Vec<InviteCodeMeta> {
    let mut result = Vec::new();
    if let Ok(db) = env.d1("DB")
        && let Ok(rows) = crate::db::invite::list_active_for_community(&db, community_id).await
    {
        for inv in rows {
            result.push(InviteCodeMeta {
                id: inv.id,
                expires_at: inv.expires_at,
                grants_role: inv.grants_role,
            });
        }
    }
    result
}
