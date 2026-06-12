#![allow(dead_code)]
//! Event creation, edit, and cancellation (RFC-009).

use worker::{D1Database, Result};
use crate::db::now_utc;
use crate::crypto::random_token;

/// Create an event and its day rows in one logical batch.
pub async fn create_event(
    db: &D1Database,
    community_id: &str,
    created_by_membership_id: &str,
    title: &str,
    location: Option<&str>,
    description: Option<&str>,
    days: &[(String, String, String)], // (day_date, starts_at_utc, ends_at_utc)
) -> Result<String> {
    let event_id = random_token()[..24].to_owned();
    let now = now_utc();
    db.prepare(
        "INSERT INTO events \
         (id, community_id, created_by_membership_id, title, location, description, \
          status, created_at, updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,'scheduled',?7,?7)",
    )
    .bind(&[
        event_id.as_str().into(),
        community_id.into(),
        created_by_membership_id.into(),
        title.into(),
        location.unwrap_or("").into(),
        description.unwrap_or("").into(),
        now.as_str().into(),
    ])?
    .run().await?;

    for (seq, (day_date, starts_utc, ends_utc)) in days.iter().enumerate() {
        let day_id = random_token()[..24].to_owned();
        db.prepare(
            "INSERT INTO event_days \
             (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        )
        .bind(&[
            day_id.as_str().into(),
            event_id.as_str().into(),
            community_id.into(),
            ((seq + 1) as u32).into(),
            day_date.as_str().into(),
            starts_utc.as_str().into(),
            ends_utc.as_str().into(),
            now.as_str().into(),
        ])?
        .run().await?;
    }
    Ok(event_id)
}

/// Edit title/location/description on a scheduled event (before first day start).
pub async fn edit_event(
    db: &D1Database,
    event_id: &str,
    title: &str,
    location: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "UPDATE events SET title=?1, location=?2, description=?3, updated_at=?4 WHERE id=?5",
    )
    .bind(&[
        title.into(),
        location.unwrap_or("").into(),
        description.unwrap_or("").into(),
        now.as_str().into(),
        event_id.into(),
    ])?
    .run().await?;
    Ok(())
}

/// Soft-cancel an event.
pub async fn cancel_event(
    db: &D1Database,
    event_id: &str,
    cancelled_by_membership_id: &str,
) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "UPDATE events SET status='cancelled', cancelled_at=?1, \
         cancelled_by_membership_id=?2, updated_at=?1 WHERE id=?3",
    )
    .bind(&[
        now.as_str().into(),
        cancelled_by_membership_id.into(),
        event_id.into(),
    ])?
    .run().await?;
    Ok(())
}
