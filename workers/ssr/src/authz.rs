//! Community-scoped authorization — RFC-004.
//!
//! Every community-scoped route calls `require_membership` before acting.
//! A missing or removed membership returns the same generic not-found response
//! as a nonexistent resource, so private resource existence is never revealed.

use worker::{Env, Result};

use crate::db::membership as membership_db;
use crate::session::AuthContext;

pub struct MembershipContext {
    pub membership_id: String,
    pub community_id: String,
    pub user_id: String,
    pub role: String,
    pub display_name: String,
}

impl MembershipContext {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

/// Verify the authenticated user has an active membership in `community_id`.
/// Returns `Err(not_found)` (generic, RFC-004) if absent or removed.
pub async fn require_membership(
    env: &Env,
    auth: &AuthContext,
    community_id: &str,
) -> Result<MembershipContext> {
    let db = env.d1("DB")?;
    let row = membership_db::find_active(&db, &auth.user_id, community_id)
        .await?
        .ok_or_else(|| worker::Error::RustError("Not found.".to_string()))?; // generic: no resource existence leak

    Ok(MembershipContext {
        membership_id: row.id,
        community_id: row.community_id,
        user_id: row.user_id,
        role: row.role,
        display_name: row.display_name,
    })
}

/// Like `require_membership`, but also checks that the user is an admin.
pub async fn require_admin(
    env: &Env,
    auth: &AuthContext,
    community_id: &str,
) -> Result<MembershipContext> {
    let ctx = require_membership(env, auth, community_id).await?;
    if !ctx.is_admin() {
        return Err(worker::Error::RustError("Not found.".to_string())); // same response as not-found
    }
    Ok(ctx)
}
