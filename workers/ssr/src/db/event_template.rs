//! Event template DB helpers (RFC-032).
//!
//! Templates are community-scoped, admin-only, soft-deletable.
//! They store title/location/description/duration as defaults for event creation.

use worker::d1::D1Database;
use worker::Result;
use crate::db::now_utc;

pub struct EventTemplateRow {
    pub id:               String,
    pub community_id:     String,
    pub title:            String,
    pub location:         Option<String>,
    pub description:      Option<String>,
    pub duration_minutes: Option<u32>,
    pub created_at:       String,
}

/// All active templates for a community, ordered by title.
pub async fn list_active(
    db: &D1Database,
    community_id: &str,
) -> Result<Vec<EventTemplateRow>> {
    let rows = db
        .prepare(
            "SELECT id, community_id, title, location, description, duration_minutes, created_at \
             FROM event_templates \
             WHERE community_id = ?1 AND is_active = 1 \
             ORDER BY title ASC",
        )
        .bind(&[community_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows.into_iter().filter_map(|v| {
        Some(EventTemplateRow {
            id:               v.get("id")?.as_str()?.to_owned(),
            community_id:     v.get("community_id")?.as_str()?.to_owned(),
            title:            v.get("title")?.as_str()?.to_owned(),
            location:         v.get("location").and_then(|x| x.as_str()).map(|s| s.to_owned()),
            description:      v.get("description").and_then(|x| x.as_str()).map(|s| s.to_owned()),
            duration_minutes: v.get("duration_minutes").and_then(|x| x.as_u64()).map(|n| n as u32),
            created_at:       v.get("created_at")?.as_str()?.to_owned(),
        })
    }).collect())
}

/// Fetch a single active template by ID, scoped to community.
pub async fn find_active(
    db: &D1Database,
    template_id: &str,
    community_id: &str,
) -> Result<Option<EventTemplateRow>> {
    let row = db
        .prepare(
            "SELECT id, community_id, title, location, description, duration_minutes, created_at \
             FROM event_templates \
             WHERE id = ?1 AND community_id = ?2 AND is_active = 1",
        )
        .bind(&[template_id.into(), community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(EventTemplateRow {
            id:               v.get("id")?.as_str()?.to_owned(),
            community_id:     v.get("community_id")?.as_str()?.to_owned(),
            title:            v.get("title")?.as_str()?.to_owned(),
            location:         v.get("location").and_then(|x| x.as_str()).map(|s| s.to_owned()),
            description:      v.get("description").and_then(|x| x.as_str()).map(|s| s.to_owned()),
            duration_minutes: v.get("duration_minutes").and_then(|x| x.as_u64()).map(|n| n as u32),
            created_at:       v.get("created_at")?.as_str()?.to_owned(),
        })
    }))
}

/// Insert a new template.
pub async fn insert(
    db: &D1Database,
    id: &str,
    community_id: &str,
    membership_id: &str,
    title: &str,
    location: Option<&str>,
    description: Option<&str>,
    duration_minutes: Option<u32>,
) -> Result<()> {
    let now = now_utc();
    // Build nullable bind values using JsValue directly.
    let loc_js: worker::wasm_bindgen::JsValue = location
        .map(|s| worker::wasm_bindgen::JsValue::from_str(s))
        .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
    let desc_js: worker::wasm_bindgen::JsValue = description
        .map(|s| worker::wasm_bindgen::JsValue::from_str(s))
        .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
    let dur_js: worker::wasm_bindgen::JsValue = duration_minutes
        .map(|d| worker::wasm_bindgen::JsValue::from_f64(d as f64))
        .unwrap_or(worker::wasm_bindgen::JsValue::NULL);

    db.prepare(
        "INSERT INTO event_templates \
         (id, community_id, created_by_membership_id, title, location, description, \
          duration_minutes, is_active, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?8)",
    )
    .bind(&[
        id.into(),
        community_id.into(),
        membership_id.into(),
        title.into(),
        loc_js,
        desc_js,
        dur_js,
        now.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Soft-delete (deactivate) a template, scoped to community.
pub async fn soft_delete(
    db: &D1Database,
    template_id: &str,
    community_id: &str,
) -> Result<()> {
    let now = now_utc();
    db.prepare(
        "UPDATE event_templates SET is_active = 0, updated_at = ?1 \
         WHERE id = ?2 AND community_id = ?3",
    )
    .bind(&[now.as_str().into(), template_id.into(), community_id.into()])?
    .run()
    .await?;
    Ok(())
}
