//! Attendance table access (RFC-002 / RFC-006).
//!
//! Status is per (event_day, membership). NULL = No answer — never fabricated.

use crate::audit::{self, AuditAction, AuditMetadata};
use crate::db::now_utc;
use worker::{D1Database, Result};

pub const ADMIN_OVERRIDE_CELL_CAP: usize = 10_000;

pub struct AttendanceRow {
    pub event_day_id: String,
    pub membership_id: String,
    /// None = No answer (NULL in DB)
    pub status: Option<String>,
}

pub struct DayCountRow {
    pub going: u32,
    pub not_going: u32,
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
            "SELECT event_day_id, membership_id, status \
             FROM attendances \
             WHERE event_day_id = ?1 AND membership_id = ?2 \
             LIMIT 1",
        )
        .bind(&[event_day_id.into(), membership_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|v| {
        Some(AttendanceRow {
            event_day_id: v.get("event_day_id")?.as_str()?.to_owned(),
            membership_id: v.get("membership_id")?.as_str()?.to_owned(),
            status: v
                .get("status")
                .and_then(|x| x.as_str())
                .map(|s| s.to_owned()),
        })
    }))
}

/// All attendances for a single day (for the participant list).
pub async fn list_for_day(db: &D1Database, event_day_id: &str) -> Result<Vec<AttendanceRow>> {
    let rows = db
        .prepare(
            "SELECT event_day_id, membership_id, status \
             FROM attendances \
             WHERE event_day_id = ?1",
        )
        .bind(&[event_day_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows
        .into_iter()
        .filter_map(|v| {
            Some(AttendanceRow {
                event_day_id: v.get("event_day_id")?.as_str()?.to_owned(),
                membership_id: v.get("membership_id")?.as_str()?.to_owned(),
                status: v
                    .get("status")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_owned()),
            })
        })
        .collect())
}

/// Fetch all attendance rows for multiple event days in a single `IN` query
/// (RFC-029 / RFC-044: no per-day N+1). Returns a map from `event_day_id` to
/// the list of attendance rows for that day.
pub async fn list_for_event_days(
    db: &D1Database,
    day_ids: &[&str],
) -> Result<std::collections::HashMap<String, Vec<AttendanceRow>>> {
    if day_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let placeholders = zinnias_ciao_contracts::build_in_placeholders(day_ids.len(), 0);
    let sql = format!(
        "SELECT event_day_id, membership_id, status \
         FROM attendances WHERE event_day_id IN ({placeholders})"
    );
    let binds: Vec<worker::wasm_bindgen::JsValue> = day_ids.iter().map(|id| (*id).into()).collect();
    let rows = db
        .prepare(&sql)
        .bind(&binds)?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    let mut out: std::collections::HashMap<String, Vec<AttendanceRow>> =
        std::collections::HashMap::new();
    for v in rows {
        if let Some(row) = (|| -> Option<AttendanceRow> {
            Some(AttendanceRow {
                event_day_id: v.get("event_day_id")?.as_str()?.to_owned(),
                membership_id: v.get("membership_id")?.as_str()?.to_owned(),
                status: v
                    .get("status")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_owned()),
            })
        })() {
            out.entry(row.event_day_id.clone()).or_default().push(row);
        }
    }
    Ok(out)
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

/// Apply the complete admin attendance matrix in one set-based statement and
/// append exactly one audit row when at least one semantic status changes.
#[allow(clippy::too_many_arguments)]
pub async fn admin_override_matrix(
    db: &D1Database,
    request_id: &str,
    community_id: &str,
    event_id: &str,
    admin_membership_id: &str,
    submitted_json: &str,
    submitted_cells: usize,
) -> Result<usize> {
    if submitted_cells == 0 {
        return Ok(0);
    }
    if submitted_cells > ADMIN_OVERRIDE_CELL_CAP {
        return Err(worker::Error::RustError(
            "attendance override cell bound exceeded".to_owned(),
        ));
    }
    let now = now_utc();
    let mutation = db
        .prepare(
            "WITH submitted AS ( \
               SELECT json_extract(value, '$.day_id') AS day_id, \
                      json_extract(value, '$.membership_id') AS membership_id, \
                      json_extract(value, '$.status') AS status \
               FROM json_each(?1) \
             ), eligible AS ( \
               SELECT s.day_id, s.membership_id, s.status \
               FROM submitted s \
               JOIN event_days d ON d.id = s.day_id \
               JOIN events e ON e.id = d.event_id \
               JOIN community_memberships target ON target.id = s.membership_id \
               WHERE d.event_id = ?3 \
                 AND d.community_id = ?4 AND e.community_id = ?4 \
                 AND d.occurrence_status = 'scheduled' \
                 AND e.status = 'scheduled' \
                 AND target.community_id = ?4 AND target.removed_at IS NULL \
                 AND (s.status IS NULL OR s.status IN ('going','not_going','attended')) \
             ) \
             INSERT INTO attendances \
               (id, event_day_id, membership_id, status, status_updated_at, updated_at) \
             SELECT lower(hex(randomblob(8))), s.day_id, s.membership_id, s.status, ?2, ?2 \
             FROM eligible s \
             WHERE (SELECT COUNT(*) FROM eligible) = (SELECT COUNT(*) FROM submitted) \
               AND (s.status IS NOT NULL OR EXISTS ( \
                    SELECT 1 FROM attendances current \
                    WHERE current.event_day_id = s.day_id \
                      AND current.membership_id = s.membership_id \
                      AND current.status IS NOT NULL \
               )) \
               AND EXISTS ( \
                    SELECT 1 FROM community_memberships actor \
                    WHERE actor.id = ?5 AND actor.community_id = ?4 \
                      AND actor.role = 'admin' AND actor.removed_at IS NULL \
               ) \
             ON CONFLICT(event_day_id, membership_id) DO UPDATE SET \
               status = excluded.status, \
               status_updated_at = excluded.status_updated_at, \
               updated_at = excluded.updated_at \
             WHERE attendances.status IS NOT excluded.status",
        )
        .bind(&[
            submitted_json.into(),
            now.as_str().into(),
            event_id.into(),
            community_id.into(),
            admin_membership_id.into(),
        ])?;
    // The fixed placeholder is validated by the closed model; the persisted
    // count is replaced with SQLite changes() by the specialized builder.
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(admin_membership_id),
        Some(event_id),
        AuditAction::AttendanceAdminOverride,
        AuditMetadata::AttendanceOverride { changed_count: 1 },
    )?;
    audit::execute_required_attendance_override(db, mutation, &record, submitted_cells as u32).await
}

/// Set an admin's own past-day attendance to `attended` with its required
/// audit row. Current authorization, event/day state, and no-op state are
/// repeated inside the mutation statement.
pub async fn set_admin_attended_required(
    db: &D1Database,
    request_id: &str,
    community_id: &str,
    event_id: &str,
    event_day_id: &str,
    membership_id: &str,
) -> Result<bool> {
    let now = now_utc();
    let attendance_id = crate::crypto::random_token()[..16].to_owned();
    let mutation = db
        .prepare(
            "INSERT INTO attendances \
             (id, event_day_id, membership_id, status, status_updated_at, updated_at) \
             SELECT ?1, ?2, ?3, 'attended', ?4, ?4 \
             WHERE EXISTS ( \
               SELECT 1 \
               FROM event_days d \
               JOIN events e ON e.id = d.event_id \
               JOIN community_memberships m ON m.id = ?3 \
               WHERE d.id = ?2 AND d.event_id = ?5 \
                 AND d.community_id = ?6 AND e.community_id = ?6 \
                 AND d.ends_at_utc <= ?4 \
                 AND d.occurrence_status = 'scheduled' \
                 AND e.status = 'scheduled' \
                 AND m.community_id = ?6 AND m.role = 'admin' \
                 AND m.removed_at IS NULL \
             ) \
             ON CONFLICT(event_day_id, membership_id) DO UPDATE SET \
               status = excluded.status, \
               status_updated_at = excluded.status_updated_at, \
               updated_at = excluded.updated_at \
             WHERE attendances.status IS NOT 'attended'",
        )
        .bind(&[
            attendance_id.as_str().into(),
            event_day_id.into(),
            membership_id.into(),
            now.as_str().into(),
            event_id.into(),
            community_id.into(),
        ])?;
    let record = audit::required_record(
        request_id,
        Some(community_id),
        Some(membership_id),
        Some(event_day_id),
        AuditAction::AttendanceAdminSetAttended,
        AuditMetadata::None,
    )?;
    audit::execute_required(db, mutation, &record).await
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
        let g = v.get("going").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let ng = v.get("not_going").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let t = v.get("total_rows").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let no_answer = active_member_count.saturating_sub(t);
        map.insert(
            day_id,
            DayCountRow {
                going: g,
                not_going: ng,
                no_answer,
            },
        );
    }

    // Days with zero attendances have no row in the result — fill them in.
    for day_id in day_ids {
        map.entry(day_id.to_string()).or_insert(DayCountRow {
            going: 0,
            not_going: 0,
            no_answer: active_member_count,
        });
    }

    Ok(map)
}
