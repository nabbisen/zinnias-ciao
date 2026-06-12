#![allow(dead_code)]
//! Attendance table access (RFC-002 / RFC-006).
//!
//! Status is per (event_day, membership). NULL = No answer — never fabricated.

use worker::{D1Database, Result};
use crate::db::now_utc;

pub struct AttendanceRow {
    pub event_day_id: String,
    pub membership_id: String,
    /// None = No answer (NULL in DB)
    pub status: Option<String>,
    pub status_updated_at: Option<String>,
}

pub struct DayCountRow {
    pub going: u32,
    pub not_going: u32,
    pub attended: u32,
    /// No answer = total active members minus those with an explicit status row
    pub no_answer: u32,
}

/// My attendance for a single day.
pub async fn find_mine(
    db: &D1Database,
    event_day_id: &str,
    membership_id: &str,
) -> Result<Option<AttendanceRow>> {
    let row = db
        .prepare(
            "SELECT event_day_id, membership_id, status, status_updated_at \
             FROM attendances \
             WHERE event_day_id = ?1 AND membership_id = ?2 \
             LIMIT 1",
        )
        .bind(&[event_day_id.into(), membership_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(AttendanceRow {
            event_day_id:      v.get("event_day_id")?.as_str()?.to_owned(),
            membership_id:     v.get("membership_id")?.as_str()?.to_owned(),
            status:            v.get("status").and_then(|x| x.as_str()).map(|s| s.to_owned()),
            status_updated_at: v.get("status_updated_at").and_then(|x| x.as_str()).map(|s| s.to_owned()),
        })
    }))
}

/// All attendances for a single day (for the participant list).
pub async fn list_for_day(
    db: &D1Database,
    event_day_id: &str,
) -> Result<Vec<AttendanceRow>> {
    let rows = db
        .prepare(
            "SELECT event_day_id, membership_id, status, status_updated_at \
             FROM attendances \
             WHERE event_day_id = ?1",
        )
        .bind(&[event_day_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows.into_iter().filter_map(|v| {
        Some(AttendanceRow {
            event_day_id:      v.get("event_day_id")?.as_str()?.to_owned(),
            membership_id:     v.get("membership_id")?.as_str()?.to_owned(),
            status:            v.get("status").and_then(|x| x.as_str()).map(|s| s.to_owned()),
            status_updated_at: v.get("status_updated_at").and_then(|x| x.as_str()).map(|s| s.to_owned()),
        })
    }).collect())
}

/// Status counts for one day. `active_member_count` is used to derive `no_answer`.
pub async fn counts_for_day(
    db: &D1Database,
    event_day_id: &str,
    active_member_count: u32,
) -> Result<DayCountRow> {
    let row = db
        .prepare(
            "SELECT \
               SUM(CASE WHEN status = 'going'     THEN 1 ELSE 0 END) AS going, \
               SUM(CASE WHEN status = 'not_going' THEN 1 ELSE 0 END) AS not_going, \
               SUM(CASE WHEN status = 'attended'  THEN 1 ELSE 0 END) AS attended, \
               COUNT(*) AS total_rows \
             FROM attendances \
             WHERE event_day_id = ?1",
        )
        .bind(&[event_day_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let (going, not_going, attended, total_rows) = row
        .map(|v| {
            let g  = v.get("going")     .and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            let ng = v.get("not_going") .and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            let a  = v.get("attended")  .and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            let t  = v.get("total_rows").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            (g, ng, a, t)
        })
        .unwrap_or((0, 0, 0, 0));

    let no_answer = active_member_count.saturating_sub(total_rows);
    Ok(DayCountRow { going, not_going, attended, no_answer })
}

/// Upsert a status for (event_day, membership). `status` = None clears to No answer.
pub async fn upsert(
    db: &D1Database,
    event_day_id: &str,
    membership_id: &str,
    status: Option<&str>,
) -> Result<()> {
    let now = now_utc();
    match status {
        Some(s) => {
            db.prepare(
                "INSERT INTO attendances (id, event_day_id, membership_id, status, status_updated_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5) \
                 ON CONFLICT(event_day_id, membership_id) DO UPDATE \
                 SET status = excluded.status, status_updated_at = excluded.status_updated_at, \
                     updated_at = excluded.updated_at",
            )
            .bind(&[
                crate::crypto::random_token()[..16].to_owned().into(),
                event_day_id.into(),
                membership_id.into(),
                s.into(),
                now.as_str().into(),
            ])?
            .run()
            .await?;
        }
        None => {
            // Clear to No answer: set status = NULL
            db.prepare(
                "INSERT INTO attendances (id, event_day_id, membership_id, status, status_updated_at, updated_at) \
                 VALUES (?1, ?2, ?3, NULL, ?4, ?4) \
                 ON CONFLICT(event_day_id, membership_id) DO UPDATE \
                 SET status = NULL, status_updated_at = excluded.status_updated_at, \
                     updated_at = excluded.updated_at",
            )
            .bind(&[
                crate::crypto::random_token()[..16].to_owned().into(),
                event_day_id.into(),
                membership_id.into(),
                now.as_str().into(),
            ])?
            .run()
            .await?;
        }
    }
    Ok(())
}

/// My attendances keyed by day_id, for a list of day IDs (RFC-029: no N+1).
/// Builds a single `IN (?,?,...)`  query at runtime — D1 supports positional
/// placeholders when spelled out individually.
pub async fn list_mine_for_days(
    db: &D1Database,
    membership_id: &str,
    day_ids: &[&str],
) -> Result<std::collections::HashMap<String, String>> {
    if day_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    // Build "?1, ?2, ..., ?N" and bind values [day_id_0, ..., day_id_{N-1}, membership_id]
    let placeholders = zinnias_ciao_contracts::build_in_placeholders(day_ids.len(), 0);
    let membership_ph = format!("?{}", day_ids.len() + 1);

    let sql = format!(
        "SELECT event_day_id, status FROM attendances \
         WHERE event_day_id IN ({placeholders}) AND membership_id = {membership_ph}"
    );

    // Build bind array: [day_id_0, ..., day_id_{N-1}, membership_id]
    // Use owned Strings so .into() can convert to JsValue.
    let mut bind_values: Vec<_> = day_ids
        .iter()
        .map(|id| worker::wasm_bindgen::JsValue::from_str(id))
        .collect();
    bind_values.push(worker::wasm_bindgen::JsValue::from_str(membership_id));

    let rows = db
        .prepare(&sql)
        .bind(&bind_values)?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    let mut map = std::collections::HashMap::new();
    for v in rows {
        if let (Some(day_id), Some(status)) = (
            v.get("event_day_id").and_then(|x| x.as_str()),
            v.get("status").and_then(|x| x.as_str()),
        ) {
            map.insert(day_id.to_owned(), status.to_owned());
        }
    }
    Ok(map)
}

/// Status counts for multiple days in a single query (RFC-029: no N+1).
/// Returns a HashMap<day_id, DayCountRow>.
/// `active_member_count` is used to derive `no_answer` for each day.
pub async fn counts_for_days(
    db: &D1Database,
    day_ids: &[&str],
    active_member_count: u32,
) -> Result<std::collections::HashMap<String, DayCountRow>> {
    if day_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let placeholders = zinnias_ciao_contracts::build_in_placeholders(day_ids.len(), 0);

    let sql = format!(
        "SELECT \
           event_day_id, \
           SUM(CASE WHEN status = 'going'     THEN 1 ELSE 0 END) AS going, \
           SUM(CASE WHEN status = 'not_going' THEN 1 ELSE 0 END) AS not_going, \
           SUM(CASE WHEN status = 'attended'  THEN 1 ELSE 0 END) AS attended, \
           COUNT(*) AS total_rows \
         FROM attendances \
         WHERE event_day_id IN ({placeholders}) \
         GROUP BY event_day_id"
    );

    let bind_values: Vec<_> = day_ids
        .iter()
        .map(|id| worker::wasm_bindgen::JsValue::from_str(id))
        .collect();

    let rows = db
        .prepare(&sql)
        .bind(&bind_values)?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    let mut map = std::collections::HashMap::new();
    for v in rows {
        let day_id = match v.get("event_day_id").and_then(|x| x.as_str()) {
            Some(id) => id.to_owned(),
            None => continue,
        };
        let g  = v.get("going")     .and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let ng = v.get("not_going") .and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let a  = v.get("attended")  .and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let t  = v.get("total_rows").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let no_answer = active_member_count.saturating_sub(t);
        map.insert(day_id, DayCountRow { going: g, not_going: ng, attended: a, no_answer });
    }

    // Days with zero attendances have no row in the result — fill them in.
    for day_id in day_ids {
        map.entry(day_id.to_string()).or_insert(DayCountRow {
            going: 0, not_going: 0, attended: 0,
            no_answer: active_member_count,
        });
    }

    Ok(map)
}

