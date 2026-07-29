// `community_id` and `user_id` are populated for completeness (RFC-004 context object)
// but most handlers address the community via the validated URL parameter directly.
// `membership_id`, `role`, and `display_name` are the fields actually read.
#![allow(dead_code)]

//!
//! Every community-scoped route calls `require_membership` before acting.
//! A missing or removed membership returns the same generic not-found response
//! as a nonexistent resource, so private resource existence is never revealed.

use worker::{Env, Result};

use crate::db::membership as membership_db;
use crate::session::AuthContext;
use zinnias_ciao_contracts::Locale;

pub struct MembershipContext {
    pub membership_id: String,
    pub community_id: String,
    pub user_id: String,
    pub role: String,
    pub display_name: String,
    /// Resolved once here (RFC-072), never re-derived downstream: the
    /// membership's stored preference if it parses, else Japanese. A
    /// stored value outside the allow-list — never expected, but possible
    /// via manual repair — falls back the same way as no preference at
    /// all. Never panics on this path (SEC-5).
    pub locale: Locale,
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

    let locale = resolve_locale(row.ui_language.as_deref());

    Ok(MembershipContext {
        membership_id: row.id,
        community_id: row.community_id,
        user_id: row.user_id,
        role: row.role,
        display_name: row.display_name,
        locale,
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

/// Require that the user is an active admin in at least one community.
/// This supports guarded bootstrap flows that are not scoped to an existing
/// community URL, without granting access to anonymous or member-only users.
pub async fn require_active_admin_somewhere(
    env: &Env,
    auth: &AuthContext,
) -> Result<MembershipContext> {
    let db = env.d1("DB")?;
    let row = membership_db::find_first_admin_for_user(&db, &auth.user_id)
        .await?
        .ok_or_else(|| worker::Error::RustError("Not found.".to_string()))?;

    Ok(MembershipContext {
        membership_id: row.id,
        community_id: row.community_id,
        user_id: row.user_id,
        role: row.role,
        display_name: row.display_name,
        locale: Locale::default(), // not a localized page; not resolved from this query
    })
}

/// RFC-072 locale resolution (Slice A): active membership preference, else
/// Japanese. `stored` is the raw `ui_language` column value — `None` (no
/// preference set) and `Some(value)` outside the allow-list (never
/// expected from the `CHECK` constraint, but possible via manual repair)
/// both resolve to the same safe fallback. Never panics: a bad stored
/// value reaching a render path would be an SEC-5 violation.
fn resolve_locale(stored: Option<&str>) -> Locale {
    stored.and_then(Locale::parse).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::resolve_locale;
    use zinnias_ciao_contracts::Locale;

    #[test]
    fn resolve_locale_uses_the_stored_preference_when_valid() {
        assert_eq!(resolve_locale(Some("ja")), Locale::Ja);
        assert_eq!(resolve_locale(Some("en")), Locale::En);
    }

    #[test]
    fn resolve_locale_falls_back_to_japanese_when_absent() {
        assert_eq!(resolve_locale(None), Locale::Ja);
    }

    #[test]
    fn resolve_locale_falls_back_to_japanese_for_an_out_of_allow_list_value_without_panicking() {
        // A value the CHECK constraint should have rejected on write, but
        // that a defensive read path must still survive (e.g. a
        // hand-repaired row, or a future schema slip).
        for bad in ["fr", "EN", "", "ja-JP", "en-US", "null", "0"] {
            assert_eq!(resolve_locale(Some(bad)), Locale::Ja, "stored={bad:?}");
        }
    }
}
