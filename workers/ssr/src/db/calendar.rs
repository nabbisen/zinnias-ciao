//! Calendar export token DB helpers (RFC-023).
//!
//! One active token per (membership_id, community_id) pair.
//! Tokens are stored as HMAC-SHA256(pepper, plaintext) — never plaintext.

use worker::Result;
use worker::d1::D1Database;

use crate::audit::{self, AuditAction, AuditMetadata};

/// Metadata returned to callers — never includes the HMAC.
pub struct CalendarTokenRow {
    pub id: String,
}

/// Look up an active (unrevoked) token by its HMAC.
/// Returns the (community_id, membership_id) pair so the feed handler can
/// validate community isolation and fetch events.
pub struct CalendarTokenClaims {
    pub community_id: String,
    pub membership_id: String,
}

pub async fn find_by_hmac(
    db: &D1Database,
    token_hmac: &str,
) -> Result<Option<CalendarTokenClaims>> {
    let rows = db
        .prepare(
            "SELECT community_id, membership_id \
             FROM calendar_tokens \
             WHERE token_hmac = ?1 AND revoked_at IS NULL",
        )
        .bind(&[token_hmac.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows.into_iter().next().and_then(|v| {
        Some(CalendarTokenClaims {
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            membership_id: v.get("membership_id")?.as_str()?.to_owned(),
        })
    }))
}

/// Find the active token for a membership in a community (for display on the Me page).
pub async fn find_active_for_membership(
    db: &D1Database,
    membership_id: &str,
    community_id: &str,
) -> Result<Option<CalendarTokenRow>> {
    let rows = db
        .prepare(
            "SELECT id \
             FROM calendar_tokens \
             WHERE membership_id = ?1 AND community_id = ?2 \
               AND revoked_at IS NULL \
             ORDER BY created_at DESC \
             LIMIT 1",
        )
        .bind(&[membership_id.into(), community_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows.into_iter().next().and_then(|v| {
        Some(CalendarTokenRow {
            id: v.get("id")?.as_str()?.to_owned(),
        })
    }))
}

#[allow(clippy::too_many_arguments)]
pub async fn rotate_required(
    db: &D1Database,
    request_id: &str,
    id: &str,
    community_id: &str,
    membership_id: &str,
    token_hmac: &str,
    now: &str,
) -> Result<bool> {
    let revoke = db
        .prepare(
            "UPDATE calendar_tokens SET revoked_at = ?1 \
             WHERE membership_id = ?2 AND community_id = ?3 \
               AND revoked_at IS NULL \
               AND EXISTS (SELECT 1 FROM community_memberships m \
                   WHERE m.id = ?2 AND m.community_id = ?3 AND m.removed_at IS NULL)",
        )
        .bind(&[now.into(), membership_id.into(), community_id.into()])?;
    let insert = db
        .prepare(
            "INSERT INTO calendar_tokens \
             (id, community_id, membership_id, token_hmac, created_at) \
             SELECT ?1, ?2, ?3, ?4, ?5 \
             WHERE EXISTS (SELECT 1 FROM community_memberships m \
                 WHERE m.id = ?3 AND m.community_id = ?2 AND m.removed_at IS NULL)",
        )
        .bind(&[
            id.into(),
            community_id.into(),
            membership_id.into(),
            token_hmac.into(),
            now.into(),
        ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(membership_id),
        None,
        AuditAction::CalendarFeedTokenGenerated,
        AuditMetadata::None,
    )?;
    audit::execute_required_tail(db, vec![revoke, insert], &record).await
}

pub async fn revoke_required(
    db: &D1Database,
    request_id: &str,
    community_id: &str,
    membership_id: &str,
    now: &str,
) -> Result<usize> {
    const ACTIVE_TOKEN_CAP: u32 = 10_000;
    let mutation = db
        .prepare(
            "UPDATE calendar_tokens SET revoked_at = ?1 \
             WHERE membership_id = ?2 AND community_id = ?3 \
               AND revoked_at IS NULL \
               AND EXISTS (SELECT 1 FROM community_memberships m \
                   WHERE m.id = ?2 AND m.community_id = ?3 AND m.removed_at IS NULL) \
               AND (SELECT COUNT(*) FROM calendar_tokens c \
                    WHERE c.membership_id = ?2 AND c.community_id = ?3 \
                      AND c.revoked_at IS NULL) <= ?4",
        )
        .bind(&[
            now.into(),
            membership_id.into(),
            community_id.into(),
            ACTIVE_TOKEN_CAP.into(),
        ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(membership_id),
        None,
        AuditAction::CalendarFeedTokenRevoked,
        AuditMetadata::None,
    )?;
    audit::execute_required_bounded(db, mutation, &record, ACTIVE_TOKEN_CAP).await
}

/// Events for the ICS feed: title, times, location, status.
/// Only events for the given community_id — enforced in query.
pub struct IcsEventRow {
    pub title: String,
    pub location: Option<String>,
    pub status: String, // "scheduled" | "cancelled"
    pub starts_at_utc: String,
    pub ends_at_utc: String,
    pub day_id: String,
}

/// Fetch all non-deleted event days for a community, ordered by start time.
/// The ICS feed includes past events (needed for calendar sync to work correctly).
pub async fn events_for_feed(db: &D1Database, community_id: &str) -> Result<Vec<IcsEventRow>> {
    let rows = db
        .prepare(
            "SELECT ed.id AS day_id, \
                    e.title, \
                    e.location, \
                    e.status, \
                    ed.starts_at_utc, \
                    ed.ends_at_utc \
             FROM event_days ed \
             JOIN events e ON e.id = ed.event_id \
             WHERE ed.community_id = ?1 \
             ORDER BY ed.starts_at_utc ASC",
        )
        .bind(&[community_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(IcsEventRow {
                day_id: v.get("day_id")?.as_str()?.to_owned(),
                title: v.get("title")?.as_str()?.to_owned(),
                location: v
                    .get("location")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_owned()),
                status: v.get("status")?.as_str()?.to_owned(),
                starts_at_utc: v.get("starts_at_utc")?.as_str()?.to_owned(),
                ends_at_utc: v.get("ends_at_utc")?.as_str()?.to_owned(),
            })
        })
        .collect())
}
