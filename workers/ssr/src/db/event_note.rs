#![allow(dead_code)]
//! Event note table access (RFC-002 / RFC-007).
//!
//! One note per (event, membership). Soft-deleted by note_deleted_at.

use worker::{D1Database, Result};
use crate::db::now_utc;

pub struct NoteRow {
    pub id: String,
    pub event_id: String,
    pub membership_id: String,
    pub note: String,
    pub note_updated_at: String,
}

/// Fetch the current note for a member on an event. Returns None if absent or deleted.
pub async fn find_mine(
    db: &D1Database,
    event_id: &str,
    membership_id: &str,
) -> Result<Option<NoteRow>> {
    let row = db
        .prepare(
            "SELECT id, event_id, membership_id, note, note_updated_at \
             FROM event_notes \
             WHERE event_id = ?1 AND membership_id = ?2 \
               AND note_deleted_at IS NULL AND hidden_by_admin_at IS NULL \
             LIMIT 1",
        )
        .bind(&[event_id.into(), membership_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(NoteRow {
            id:              v.get("id")?.as_str()?.to_owned(),
            event_id:        v.get("event_id")?.as_str()?.to_owned(),
            membership_id:   v.get("membership_id")?.as_str()?.to_owned(),
            note:            v.get("note")?.as_str()?.to_owned(),
            note_updated_at: v.get("note_updated_at")?.as_str()?.to_owned(),
        })
    }))
}

/// All visible notes for an event, for the notes list on Event Detail.
pub async fn list_for_event(
    db: &D1Database,
    event_id: &str,
) -> Result<Vec<NoteRow>> {
    let rows = db
        .prepare(
            "SELECT id, event_id, membership_id, note, note_updated_at \
             FROM event_notes \
             WHERE event_id = ?1 \
               AND note_deleted_at IS NULL AND hidden_by_admin_at IS NULL \
             ORDER BY note_updated_at ASC",
        )
        .bind(&[event_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows.into_iter().filter_map(|v| {
        Some(NoteRow {
            id:              v.get("id")?.as_str()?.to_owned(),
            event_id:        v.get("event_id")?.as_str()?.to_owned(),
            membership_id:   v.get("membership_id")?.as_str()?.to_owned(),
            note:            v.get("note")?.as_str()?.to_owned(),
            note_updated_at: v.get("note_updated_at")?.as_str()?.to_owned(),
        })
    }).collect())
}

/// Upsert a note. Called after server-side validation (RFC-007 §5).
pub async fn upsert(
    db: &D1Database,
    event_id: &str,
    membership_id: &str,
    note: &str,
) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "INSERT INTO event_notes (id, event_id, membership_id, note, note_updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(event_id, membership_id) DO UPDATE \
         SET note = excluded.note, note_updated_at = excluded.note_updated_at, \
             note_deleted_at = NULL",
    )
    .bind(&[
        crate::crypto::random_token()[..16].to_owned().into(),
        event_id.into(),
        membership_id.into(),
        note.into(),
        now.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Soft-delete a member's own note.
pub async fn soft_delete(
    db: &D1Database,
    event_id: &str,
    membership_id: &str,
) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "UPDATE event_notes SET note_deleted_at = ?1 \
         WHERE event_id = ?2 AND membership_id = ?3",
    )
    .bind(&[now.as_str().into(), event_id.into(), membership_id.into()])?
    .run()
    .await?;
    Ok(())
}

/// Admin moderation hide (does not copy note body to audit — RFC-014).
pub async fn admin_hide(
    db: &D1Database,
    event_id: &str,
    membership_id: &str,
) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "UPDATE event_notes SET hidden_by_admin_at = ?1 \
         WHERE event_id = ?2 AND membership_id = ?3",
    )
    .bind(&[now.as_str().into(), event_id.into(), membership_id.into()])?
    .run()
    .await?;
    Ok(())
}
