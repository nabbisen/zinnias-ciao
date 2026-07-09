#![allow(dead_code)]
//! Recurrence-series access and materialization (RFC-065).

use crate::crypto::random_token;
use crate::db::event_write::EventDayInsert;
use crate::db::now_utc;
use worker::{D1Database, Result};
use zinnias_ciao_contracts::tz;
use zinnias_ciao_domain::{
    DayInput, RECURRENCE_MATERIALIZATION_INSERT_CAP, RecurrenceEnd, RecurrenceFreq,
    generate_recurrence_occurrences_after,
};

pub struct EventSeriesRow {
    pub id: String,
    pub event_id: String,
    pub community_id: String,
    pub frequency: String,
    pub start_day_date: String,
    pub starts_at_local: Option<String>,
    pub ends_at_local: Option<String>,
    pub timezone: String,
    pub end_mode: String,
    pub occurrence_count: Option<u32>,
    pub until_day_date: Option<String>,
    pub materialized_through_day_date: Option<String>,
}

pub struct MaterializationReport {
    pub inserted: usize,
    pub cap_reached: bool,
}

pub async fn find_for_event(
    db: &D1Database,
    event_id: &str,
    community_id: &str,
) -> Result<Option<EventSeriesRow>> {
    let row = db
        .prepare(
            "SELECT id, event_id, community_id, frequency, start_day_date, \
                    starts_at_local, ends_at_local, timezone, end_mode, \
                    occurrence_count, until_day_date, materialized_through_day_date \
             FROM event_series \
             WHERE event_id = ?1 AND community_id = ?2 \
             LIMIT 1",
        )
        .bind(&[event_id.into(), community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(parse_series_row))
}

pub async fn materialize_for_community_through(
    db: &D1Database,
    community_id: &str,
    through_day_date: &str,
) -> Result<MaterializationReport> {
    let series = list_active_for_community(db, community_id).await?;
    let mut inserted = 0usize;
    let mut cap_reached = false;
    let mut remaining = RECURRENCE_MATERIALIZATION_INSERT_CAP;
    for row in series {
        if let Some(current) = row.materialized_through_day_date.as_deref()
            && current >= through_day_date
        {
            continue;
        }
        if remaining == 0 {
            cap_reached = true;
            break;
        }
        let report = materialize_series(db, &row, through_day_date, remaining).await?;
        inserted += report.inserted;
        remaining = remaining.saturating_sub(report.inserted);
        cap_reached |= report.cap_reached;
    }
    Ok(MaterializationReport {
        inserted,
        cap_reached,
    })
}

pub async fn materialize_series(
    db: &D1Database,
    series: &EventSeriesRow,
    through_day_date: &str,
    max_inserts: usize,
) -> Result<MaterializationReport> {
    if max_inserts == 0 {
        return Ok(MaterializationReport {
            inserted: 0,
            cap_reached: true,
        });
    }
    let freq = RecurrenceFreq::parse_form_value(&series.frequency);
    if !freq.is_recurring() {
        return Ok(MaterializationReport {
            inserted: 0,
            cap_reached: false,
        });
    }
    let (Some(starts_at_local), Some(ends_at_local)) = (
        series.starts_at_local.as_deref(),
        series.ends_at_local.as_deref(),
    ) else {
        return Ok(MaterializationReport {
            inserted: 0,
            cap_reached: false,
        });
    };
    let end = match series.end_mode.as_str() {
        "open_ended" => RecurrenceEnd::OpenEnded,
        "until_date" => RecurrenceEnd::UntilDate(series.until_day_date.clone().unwrap_or_default()),
        _ => RecurrenceEnd::AfterCount(series.occurrence_count.unwrap_or(1)),
    };
    let skips = list_skip_dates(db, &series.id).await?;
    let base = DayInput {
        day_date: series.start_day_date.clone(),
        starts_at: starts_at_local.to_owned(),
        ends_at: ends_at_local.to_owned(),
    };
    let previous_materialized = series.materialized_through_day_date.as_deref();
    let occurrences = match generate_recurrence_occurrences_after(
        &base,
        freq,
        &end,
        previous_materialized,
        through_day_date,
        &skips,
        max_inserts,
    ) {
        Ok(v) => v,
        Err(_) => {
            return Ok(MaterializationReport {
                inserted: 0,
                cap_reached: true,
            });
        }
    };
    let offset = tz::offset_minutes_or_utc(&series.timezone);
    let now = now_utc();
    let mut inserted = 0usize;
    let mut last_date = series
        .materialized_through_day_date
        .clone()
        .unwrap_or_else(|| series.start_day_date.clone());

    for occurrence in &occurrences {
        if occurrence.day.day_date <= last_date {
            continue;
        }
        let starts_at_utc =
            tz::local_to_utc(&occurrence.day.day_date, &occurrence.day.starts_at, offset);
        let ends_at_utc =
            tz::local_to_utc(&occurrence.day.day_date, &occurrence.day.ends_at, offset);
        let day = EventDayInsert {
            seq: occurrence.ordinal,
            day_date: &occurrence.day.day_date,
            starts_at_utc: &starts_at_utc,
            ends_at_utc: &ends_at_utc,
            series_id: Some(&series.id),
            series_occurrence_date: Some(&occurrence.day.day_date),
        };
        if insert_occurrence_if_missing(db, &series.event_id, &series.community_id, &day, &now)
            .await?
        {
            inserted += 1;
            if inserted >= max_inserts {
                last_date = occurrence.day.day_date.clone();
                break;
            }
        }
        last_date = occurrence.day.day_date.clone();
    }

    if last_date
        > series
            .materialized_through_day_date
            .clone()
            .unwrap_or_default()
    {
        db.prepare(
            "UPDATE event_series SET materialized_through_day_date=?1, updated_at=?2 WHERE id=?3",
        )
        .bind(&[
            last_date.as_str().into(),
            now.as_str().into(),
            series.id.as_str().into(),
        ])?
        .run()
        .await?;
    }

    Ok(MaterializationReport {
        inserted,
        cap_reached: occurrences.len() >= max_inserts || inserted >= max_inserts,
    })
}

async fn insert_occurrence_if_missing(
    db: &D1Database,
    event_id: &str,
    community_id: &str,
    day: &EventDayInsert<'_>,
    now: &str,
) -> Result<bool> {
    if occurrence_exists(
        db,
        day.series_id.unwrap_or(""),
        day.series_occurrence_date.unwrap_or(""),
    )
    .await?
    {
        return Ok(false);
    }
    let day_id = random_token()[..24].to_owned();
    db.prepare(
        "INSERT OR IGNORE INTO event_days \
         (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, \
          series_id, series_occurrence_date) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
    )
    .bind(&[
        day_id.as_str().into(),
        event_id.into(),
        community_id.into(),
        day.seq.into(),
        day.day_date.into(),
        day.starts_at_utc.into(),
        day.ends_at_utc.into(),
        now.into(),
        day.series_id.unwrap_or("").into(),
        day.series_occurrence_date.unwrap_or("").into(),
    ])?
    .run()
    .await?;
    Ok(true)
}

async fn occurrence_exists(
    db: &D1Database,
    series_id: &str,
    series_occurrence_date: &str,
) -> Result<bool> {
    if series_id.is_empty() || series_occurrence_date.is_empty() {
        return Ok(false);
    }
    let row = db
        .prepare(
            "SELECT id FROM event_days \
             WHERE series_id = ?1 AND series_occurrence_date = ?2 \
             LIMIT 1",
        )
        .bind(&[series_id.into(), series_occurrence_date.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.is_some())
}

async fn list_active_for_community(
    db: &D1Database,
    community_id: &str,
) -> Result<Vec<EventSeriesRow>> {
    let rows = db
        .prepare(
            "SELECT s.id, s.event_id, s.community_id, s.frequency, s.start_day_date, \
                    s.starts_at_local, s.ends_at_local, s.timezone, s.end_mode, \
                    s.occurrence_count, s.until_day_date, s.materialized_through_day_date \
             FROM event_series s \
             JOIN events e ON e.id = s.event_id \
             WHERE s.community_id = ?1 AND e.status = 'scheduled' \
             ORDER BY s.start_day_date ASC \
             LIMIT 100",
        )
        .bind(&[community_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    Ok(rows.into_iter().filter_map(parse_series_row).collect())
}

async fn list_skip_dates(db: &D1Database, series_id: &str) -> Result<Vec<String>> {
    let rows = db
        .prepare(
            "SELECT exception_day_date FROM event_series_exceptions \
             WHERE series_id = ?1 AND action = 'skip'",
        )
        .bind(&[series_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;
    Ok(rows
        .into_iter()
        .filter_map(|v| {
            v.get("exception_day_date")
                .and_then(|x| x.as_str())
                .map(str::to_owned)
        })
        .collect())
}

fn parse_series_row(v: serde_json::Value) -> Option<EventSeriesRow> {
    Some(EventSeriesRow {
        id: v.get("id")?.as_str()?.to_owned(),
        event_id: v.get("event_id")?.as_str()?.to_owned(),
        community_id: v.get("community_id")?.as_str()?.to_owned(),
        frequency: v.get("frequency")?.as_str()?.to_owned(),
        start_day_date: v.get("start_day_date")?.as_str()?.to_owned(),
        starts_at_local: v
            .get("starts_at_local")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        ends_at_local: v
            .get("ends_at_local")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        timezone: v.get("timezone")?.as_str()?.to_owned(),
        end_mode: v.get("end_mode")?.as_str()?.to_owned(),
        occurrence_count: v
            .get("occurrence_count")
            .and_then(|x| x.as_u64())
            .map(|n| n as u32),
        until_day_date: v
            .get("until_day_date")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        materialized_through_day_date: v
            .get("materialized_through_day_date")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
    })
}
