#![allow(dead_code)]
//! Event and EventDay table access (RFC-002 / RFC-005).

use worker::{D1Database, Result};

pub struct EventRow {
    pub id: String,
    pub community_id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub status: String, // "scheduled" | "cancelled"
    pub repeat_rule: String,
    pub repeat_count: Option<u32>,
}

pub struct EventDayRow {
    pub id: String,
    pub event_id: String,
    pub seq: u32,
    pub day_date: String,
    pub starts_at_utc: String,
    pub ends_at_utc: String,
    pub occurrence_status: String,
    pub series_id: Option<String>,
    pub series_occurrence_date: Option<String>,
}

/// Fetch a single active event by id, scoped to community (RFC-004).
pub async fn find_for_community(
    db: &D1Database,
    event_id: &str,
    community_id: &str,
) -> Result<Option<EventRow>> {
    let row = db
        .prepare(
            "SELECT id, community_id, title, description, location, status, \
                    repeat_rule, repeat_count \
             FROM events \
             WHERE id = ?1 AND community_id = ?2 \
             LIMIT 1",
        )
        .bind(&[event_id.into(), community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(EventRow {
            id: v.get("id")?.as_str()?.to_owned(),
            community_id: v.get("community_id")?.as_str()?.to_owned(),
            title: v.get("title")?.as_str()?.to_owned(),
            description: v
                .get("description")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned()),
            location: v
                .get("location")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned()),
            status: v.get("status")?.as_str()?.to_owned(),
            repeat_rule: v
                .get("repeat_rule")
                .and_then(|x| x.as_str())
                .unwrap_or("none")
                .to_owned(),
            repeat_count: v
                .get("repeat_count")
                .and_then(|x| x.as_u64())
                .map(|n| n as u32),
        })
    }))
}

/// All days for one event, ordered by seq.
pub async fn days_for_event(db: &D1Database, event_id: &str) -> Result<Vec<EventDayRow>> {
    let rows = db
        .prepare(
            "SELECT id, event_id, seq, day_date, starts_at_utc, ends_at_utc \
                    , COALESCE(occurrence_status, 'scheduled') AS occurrence_status, \
                    series_id, series_occurrence_date \
             FROM event_days \
             WHERE event_id = ?1 \
             ORDER BY seq ASC",
        )
        .bind(&[event_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(EventDayRow {
                id: v.get("id")?.as_str()?.to_owned(),
                event_id: v.get("event_id")?.as_str()?.to_owned(),
                seq: v.get("seq")?.as_u64()? as u32,
                day_date: v.get("day_date")?.as_str()?.to_owned(),
                starts_at_utc: v.get("starts_at_utc")?.as_str()?.to_owned(),
                ends_at_utc: v.get("ends_at_utc")?.as_str()?.to_owned(),
                occurrence_status: v
                    .get("occurrence_status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("scheduled")
                    .to_owned(),
                series_id: v
                    .get("series_id")
                    .and_then(|x| x.as_str())
                    .map(str::to_owned),
                series_occurrence_date: v
                    .get("series_occurrence_date")
                    .and_then(|x| x.as_str())
                    .map(str::to_owned),
            })
        })
        .collect())
}

/// Home list query: upcoming event_days for one community within a date window.
/// Returns rows joined with event title/location/status; ordered by starts_at_utc.
pub struct HomeEventRow {
    pub community_id: String,
    pub event_id: String,
    pub event_title: String,
    pub event_location: Option<String>,
    pub event_status: String,
    pub day_id: String,
    pub day_date: String,
    pub starts_at_utc: String,
    pub ends_at_utc: String,
    pub occurrence_status: String,
    pub series_id: Option<String>,
    pub seq: u32,
    pub total_days: u32,
}

pub async fn home_upcoming(
    db: &D1Database,
    community_id: &str,
    from_utc: &str,
    to_utc: &str,
) -> Result<Vec<HomeEventRow>> {
    let rows = db
        .prepare(
            "SELECT \
               e.id AS event_id, e.title AS event_title, \
               e.location AS event_location, e.status AS event_status, \
               d.id AS day_id, d.day_date, d.starts_at_utc, d.ends_at_utc, \
               COALESCE(d.occurrence_status, 'scheduled') AS occurrence_status, \
               d.series_id, d.seq, \
               (SELECT COUNT(*) FROM event_days d2 WHERE d2.event_id = e.id) AS total_days \
             FROM event_days d \
             JOIN events e ON e.id = d.event_id \
             WHERE d.community_id = ?1 \
               AND d.starts_at_utc >= ?2 \
               AND d.starts_at_utc <  ?3 \
             ORDER BY d.starts_at_utc ASC \
             LIMIT 100",
        )
        .bind(&[community_id.into(), from_utc.into(), to_utc.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(HomeEventRow {
                community_id: community_id.to_owned(),
                event_id: v.get("event_id")?.as_str()?.to_owned(),
                event_title: v.get("event_title")?.as_str()?.to_owned(),
                event_location: v
                    .get("event_location")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_owned()),
                event_status: v.get("event_status")?.as_str()?.to_owned(),
                day_id: v.get("day_id")?.as_str()?.to_owned(),
                day_date: v.get("day_date")?.as_str()?.to_owned(),
                starts_at_utc: v.get("starts_at_utc")?.as_str()?.to_owned(),
                ends_at_utc: v.get("ends_at_utc")?.as_str()?.to_owned(),
                occurrence_status: v
                    .get("occurrence_status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("scheduled")
                    .to_owned(),
                series_id: v
                    .get("series_id")
                    .and_then(|x| x.as_str())
                    .map(str::to_owned),
                seq: v.get("seq")?.as_u64()? as u32,
                total_days: v.get("total_days")?.as_u64()? as u32,
            })
        })
        .collect())
}

pub async fn calendar_month_for_community(
    db: &D1Database,
    community_id: &str,
    from_day_date: &str,
    to_day_date: &str,
) -> Result<Vec<HomeEventRow>> {
    let rows = db
        .prepare(
            "SELECT \
               e.community_id AS community_id, \
               e.id AS event_id, e.title AS event_title, \
               e.location AS event_location, e.status AS event_status, \
               d.id AS day_id, d.day_date, d.starts_at_utc, d.ends_at_utc, \
               COALESCE(d.occurrence_status, 'scheduled') AS occurrence_status, \
               d.series_id, d.seq, \
               (SELECT COUNT(*) FROM event_days d2 WHERE d2.event_id = e.id) AS total_days \
             FROM event_days d \
             JOIN events e ON e.id = d.event_id \
             WHERE d.community_id = ?1 \
               AND d.day_date >= ?2 \
               AND d.day_date <  ?3 \
             ORDER BY d.day_date ASC, d.starts_at_utc ASC \
             LIMIT 300",
        )
        .bind(&[
            community_id.into(),
            from_day_date.into(),
            to_day_date.into(),
        ])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(HomeEventRow {
                community_id: v.get("community_id")?.as_str()?.to_owned(),
                event_id: v.get("event_id")?.as_str()?.to_owned(),
                event_title: v.get("event_title")?.as_str()?.to_owned(),
                event_location: v
                    .get("event_location")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_owned()),
                event_status: v.get("event_status")?.as_str()?.to_owned(),
                day_id: v.get("day_id")?.as_str()?.to_owned(),
                day_date: v.get("day_date")?.as_str()?.to_owned(),
                starts_at_utc: v.get("starts_at_utc")?.as_str()?.to_owned(),
                ends_at_utc: v.get("ends_at_utc")?.as_str()?.to_owned(),
                occurrence_status: v
                    .get("occurrence_status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("scheduled")
                    .to_owned(),
                series_id: v
                    .get("series_id")
                    .and_then(|x| x.as_str())
                    .map(str::to_owned),
                seq: v.get("seq")?.as_u64()? as u32,
                total_days: v.get("total_days")?.as_u64()? as u32,
            })
        })
        .collect())
}

pub async fn home_upcoming_for_communities(
    db: &D1Database,
    community_ids: &[&str],
    from_utc: &str,
    to_utc: &str,
) -> Result<Vec<HomeEventRow>> {
    if community_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = zinnias_ciao_contracts::build_in_placeholders(community_ids.len(), 0);
    let from_ph = format!("?{}", community_ids.len() + 1);
    let to_ph = format!("?{}", community_ids.len() + 2);
    let sql = format!(
        "SELECT \
           e.community_id AS community_id, \
           e.id AS event_id, e.title AS event_title, \
           e.location AS event_location, e.status AS event_status, \
           d.id AS day_id, d.day_date, d.starts_at_utc, d.ends_at_utc, \
           COALESCE(d.occurrence_status, 'scheduled') AS occurrence_status, \
           d.series_id, d.seq, \
           (SELECT COUNT(*) FROM event_days d2 WHERE d2.event_id = e.id) AS total_days \
         FROM event_days d \
         JOIN events e ON e.id = d.event_id \
         WHERE d.community_id IN ({placeholders}) \
           AND d.starts_at_utc >= {from_ph} \
           AND d.starts_at_utc <  {to_ph} \
         ORDER BY d.community_id ASC, d.starts_at_utc ASC \
         LIMIT 300"
    );
    let mut binds: Vec<worker::wasm_bindgen::JsValue> = community_ids
        .iter()
        .map(|id| worker::wasm_bindgen::JsValue::from_str(id))
        .collect();
    binds.push(worker::wasm_bindgen::JsValue::from_str(from_utc));
    binds.push(worker::wasm_bindgen::JsValue::from_str(to_utc));

    let rows = db
        .prepare(&sql)
        .bind(&binds)?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(HomeEventRow {
                community_id: v.get("community_id")?.as_str()?.to_owned(),
                event_id: v.get("event_id")?.as_str()?.to_owned(),
                event_title: v.get("event_title")?.as_str()?.to_owned(),
                event_location: v
                    .get("event_location")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_owned()),
                event_status: v.get("event_status")?.as_str()?.to_owned(),
                day_id: v.get("day_id")?.as_str()?.to_owned(),
                day_date: v.get("day_date")?.as_str()?.to_owned(),
                starts_at_utc: v.get("starts_at_utc")?.as_str()?.to_owned(),
                ends_at_utc: v.get("ends_at_utc")?.as_str()?.to_owned(),
                occurrence_status: v
                    .get("occurrence_status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("scheduled")
                    .to_owned(),
                series_id: v
                    .get("series_id")
                    .and_then(|x| x.as_str())
                    .map(str::to_owned),
                seq: v.get("seq")?.as_u64()? as u32,
                total_days: v.get("total_days")?.as_u64()? as u32,
            })
        })
        .collect())
}
