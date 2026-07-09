#![allow(dead_code)]
//! Event creation, edit, and cancellation (RFC-009).

use crate::crypto::random_token;
use crate::db::now_utc;
use worker::{D1Database, Result};

pub struct EventDayInsert<'a> {
    pub seq: u32,
    pub day_date: &'a str,
    pub starts_at_utc: &'a str,
    pub ends_at_utc: &'a str,
    pub series_id: Option<&'a str>,
    pub series_occurrence_date: Option<&'a str>,
}

pub struct EventSeriesInsert<'a> {
    pub id: &'a str,
    pub frequency: &'a str,
    pub start_day_date: &'a str,
    pub starts_at_local: &'a str,
    pub ends_at_local: &'a str,
    pub timezone: &'a str,
    pub end_mode: &'a str,
    pub occurrence_count: Option<u32>,
    pub until_day_date: Option<&'a str>,
    pub materialized_through_day_date: Option<&'a str>,
}

/// Create an event and its day rows in one logical batch.
/// `repeat_rule` and `repeat_count` are stored for reference; the actual
/// day rows in `days` are already the fully-expanded occurrences.
#[allow(clippy::too_many_arguments)]
pub async fn create_event(
    db: &D1Database,
    community_id: &str,
    created_by_membership_id: &str,
    title: &str,
    location: Option<&str>,
    description: Option<&str>,
    days: &[EventDayInsert<'_>],
    repeat_rule: &str,
    repeat_count: Option<u32>,
    series: Option<EventSeriesInsert<'_>>,
) -> Result<String> {
    let event_id = random_token()[..24].to_owned();
    let now = now_utc();
    let rc_js: worker::wasm_bindgen::JsValue = repeat_count
        .map(|n| worker::wasm_bindgen::JsValue::from_f64(n as f64))
        .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
    db.prepare(
        "INSERT INTO events \
         (id, community_id, created_by_membership_id, title, location, description, \
          status, repeat_rule, repeat_count, created_at, updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,'scheduled',?7,?8,?9,?9)",
    )
    .bind(&[
        event_id.as_str().into(),
        community_id.into(),
        created_by_membership_id.into(),
        title.into(),
        location.unwrap_or("").into(),
        description.unwrap_or("").into(),
        repeat_rule.into(),
        rc_js,
        now.as_str().into(),
    ])?
    .run()
    .await?;

    if let Some(series) = series {
        let occurrence_count_js: worker::wasm_bindgen::JsValue = series
            .occurrence_count
            .map(|n| worker::wasm_bindgen::JsValue::from_f64(n as f64))
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
        let until_day_date_js = series
            .until_day_date
            .map(worker::wasm_bindgen::JsValue::from_str)
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
        let materialized_through_js = series
            .materialized_through_day_date
            .map(worker::wasm_bindgen::JsValue::from_str)
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
        db.prepare(
            "INSERT INTO event_series \
             (id, event_id, community_id, frequency, start_day_date, starts_at_local, \
              ends_at_local, timezone, end_mode, occurrence_count, until_day_date, \
              materialized_through_day_date, created_at, updated_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13)",
        )
        .bind(&[
            series.id.into(),
            event_id.as_str().into(),
            community_id.into(),
            series.frequency.into(),
            series.start_day_date.into(),
            series.starts_at_local.into(),
            series.ends_at_local.into(),
            series.timezone.into(),
            series.end_mode.into(),
            occurrence_count_js,
            until_day_date_js,
            materialized_through_js,
            now.as_str().into(),
        ])?
        .run()
        .await?;
    }

    for day in days {
        let day_id = random_token()[..24].to_owned();
        let series_id = day
            .series_id
            .map(worker::wasm_bindgen::JsValue::from_str)
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
        let series_occurrence_date = day
            .series_occurrence_date
            .map(worker::wasm_bindgen::JsValue::from_str)
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL);
        db.prepare(
            "INSERT INTO event_days \
             (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, \
              series_id, series_occurrence_date) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        )
        .bind(&[
            day_id.as_str().into(),
            event_id.as_str().into(),
            community_id.into(),
            day.seq.into(),
            day.day_date.into(),
            day.starts_at_utc.into(),
            day.ends_at_utc.into(),
            now.as_str().into(),
            series_id,
            series_occurrence_date,
        ])?
        .run()
        .await?;
    }
    Ok(event_id)
}

pub async fn cancel_occurrence(
    db: &D1Database,
    event_day_id: &str,
    membership_id: &str,
    series_id: &str,
    community_id: &str,
    exception_day_date: &str,
) -> Result<()> {
    let now = now_utc();
    db.prepare("UPDATE event_days SET occurrence_status='cancelled' WHERE id=?1")
        .bind(&[event_day_id.into()])?
        .run()
        .await?;

    db.prepare(
        "INSERT INTO event_series_exceptions \
         (id, series_id, community_id, exception_day_date, action, event_day_id, \
          created_by_membership_id, created_at) \
         VALUES (?1,?2,?3,?4,'cancel',?5,?6,?7) \
         ON CONFLICT(series_id, exception_day_date) DO UPDATE SET \
           action='cancel', event_day_id=excluded.event_day_id, \
           created_by_membership_id=excluded.created_by_membership_id, \
           created_at=excluded.created_at",
    )
    .bind(&[
        random_token()[..24].to_owned().into(),
        series_id.into(),
        community_id.into(),
        exception_day_date.into(),
        event_day_id.into(),
        membership_id.into(),
        now.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Edit title/location/description on a scheduled event (before first day start).
/// Edit an event's details. Updates title/location/description on the event,
/// and — for single-day events only — the date and time on its single event_day.
/// `day` is `(day_date, starts_at_utc, ends_at_utc)`; pass `None` to edit
/// details only (e.g. multi-day/recurring events, where per-day time editing
/// is out of scope).
pub async fn edit_event(
    db: &D1Database,
    event_id: &str,
    title: &str,
    location: Option<&str>,
    description: Option<&str>,
    day: Option<(&str, &str, &str)>,
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
    .run()
    .await?;

    // Persist the single-day time edit (seq = 1). For multi-day/recurring
    // events `day` is None and only the details above are updated.
    if let Some((day_date, starts_utc, ends_utc)) = day {
        db.prepare(
            "UPDATE event_days SET day_date=?1, starts_at_utc=?2, ends_at_utc=?3 \
             WHERE event_id=?4 AND seq=1",
        )
        .bind(&[
            day_date.into(),
            starts_utc.into(),
            ends_utc.into(),
            event_id.into(),
        ])?
        .run()
        .await?;
    }
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
    .run()
    .await?;
    Ok(())
}
