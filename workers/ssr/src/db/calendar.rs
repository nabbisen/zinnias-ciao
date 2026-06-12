//! Calendar export token DB helpers (RFC-023).
//!
//! One active token per (membership_id, community_id) pair.
//! Tokens are stored as HMAC-SHA256(pepper, plaintext) — never plaintext.

use worker::d1::D1Database;
use worker::Result;

/// Metadata returned to callers — never includes the HMAC.
pub struct CalendarTokenRow {
    pub id: String,
    pub created_at: String,
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
            community_id:  v.get("community_id")?.as_str()?.to_owned(),
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
            "SELECT id, created_at \
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
            id:         v.get("id")?.as_str()?.to_owned(),
            created_at: v.get("created_at")?.as_str()?.to_owned(),
        })
    }))
}

/// Insert a new calendar token. Caller has already revoked any previous token.
pub async fn insert(
    db: &D1Database,
    id: &str,
    community_id: &str,
    membership_id: &str,
    token_hmac: &str,
    created_at: &str,
) -> Result<()> {
    db.prepare(
        "INSERT INTO calendar_tokens \
         (id, community_id, membership_id, token_hmac, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&[
        id.into(),
        community_id.into(),
        membership_id.into(),
        token_hmac.into(),
        created_at.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Revoke all active tokens for a membership in a community.
/// Called before inserting a replacement (rotate) or when the member
/// explicitly disables the feed.
pub async fn revoke_for_membership(
    db: &D1Database,
    membership_id: &str,
    community_id: &str,
    revoked_at: &str,
) -> Result<()> {
    db.prepare(
        "UPDATE calendar_tokens \
         SET revoked_at = ?1 \
         WHERE membership_id = ?2 AND community_id = ?3 \
           AND revoked_at IS NULL",
    )
    .bind(&[revoked_at.into(), membership_id.into(), community_id.into()])?
    .run()
    .await?;
    Ok(())
}

/// Events for the ICS feed: title, times, location, status.
/// Only events for the given community_id — enforced in query.
pub struct IcsEventRow {
    pub event_id:     String,
    pub title:        String,
    pub location:     Option<String>,
    pub status:       String, // "scheduled" | "cancelled"
    pub starts_at_utc: String,
    pub ends_at_utc:   String,
    pub day_id:       String,
}

/// Fetch all non-deleted event days for a community, ordered by start time.
/// The ICS feed includes past events (needed for calendar sync to work correctly).
pub async fn events_for_feed(
    db: &D1Database,
    community_id: &str,
) -> Result<Vec<IcsEventRow>> {
    let rows = db
        .prepare(
            "SELECT ed.id AS day_id, \
                    e.id AS event_id, \
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

    Ok(rows.into_iter().filter_map(|v| {
        Some(IcsEventRow {
            day_id:        v.get("day_id")?.as_str()?.to_owned(),
            event_id:      v.get("event_id")?.as_str()?.to_owned(),
            title:         v.get("title")?.as_str()?.to_owned(),
            location:      v.get("location").and_then(|x| x.as_str()).map(|s| s.to_owned()),
            status:        v.get("status")?.as_str()?.to_owned(),
            starts_at_utc: v.get("starts_at_utc")?.as_str()?.to_owned(),
            ends_at_utc:   v.get("ends_at_utc")?.as_str()?.to_owned(),
        })
    }).collect())
}
