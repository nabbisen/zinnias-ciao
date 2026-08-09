// Handoff 046 §7.4: dead only on the native target — the `#[cfg(target_arch
// = "wasm32")]`-gated Durable Object glue below is this module's only
// caller of the pure policy/transition logic, and that glue does not
// compile natively. A blanket `#![allow(dead_code)]` would say "this module
// has dead code," which is false for the artifact that ships; this form
// keeps wasm32 honest, so a genuinely dead item here is still caught on the
// target that matters.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
//! `AbuseLimiter` Durable Object — RFC-078.
//!
//! One SQLite-backed Durable Object per HMAC-derived subject/scope,
//! serialized by the platform's per-object input gate. Exposes a private,
//! versioned, same-Worker-only protocol (`POST /v1/reserve`, `POST
//! /v1/reset`) to `abuse_control`, which is the only caller. Never exposed
//! as a public application route.
//!
//! The atomic transition (read → decide → write) runs with no `.await`
//! between the read and the write: `SqlStorage::exec` is synchronous, so
//! Durable Object input-gate serialization plus that synchronicity together
//! give the required atomicity (H-N3). The cleanup alarm is scheduled only
//! after the write has been persisted, and `alarm()` is implemented
//! explicitly — the `DurableObject::alarm()` trait default is
//! `unimplemented!()` and would panic when a retention alarm fires.
//!
//! The `#[durable_object]`-annotated class and its `worker`/`wasm_bindgen`
//! glue only compile for `wasm32` (wasm-bindgen's macro-generated runtime
//! support is itself target-gated upstream), so every item that touches it
//! below is individually gated with `#[cfg(target_arch = "wasm32")]`. The
//! pure policy/transition logic has no such dependency and is natively unit
//! tested.

#[cfg(target_arch = "wasm32")]
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use worker::{
    Date, DurableObject, Env, Method, Request, Response, Result as WorkerResult, SqlStorage,
    SqlStorageValue, State, durable_object,
};

#[cfg(target_arch = "wasm32")]
use crate::abuse_control::PROTOCOL_MAX_BODY_BYTES;

// ── Pure policy and transition logic (natively testable) ───────────────────

#[derive(Clone, Debug, Eq, PartialEq)]
struct Row {
    policy: String,
    window_started_ms: i64,
    count: i64,
}

/// Extract the media type from a `Content-Type` header value, ignoring any
/// parameters (e.g. `; charset=utf-8`) and comparing case-insensitively, so a
/// cosmetic runtime normalization does not turn every reservation into a
/// fail-closed `503` (I-N4).
fn is_json_media_type(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case("application/json")
}

/// `(limit, window_ms)` for a closed policy identifier. `None` for anything
/// else — an unknown policy fails closed.
fn policy_limits(policy: &str) -> Option<(i64, i64)> {
    match policy {
        "invite" | "relink" => Some((10, 300_000)),
        "community" => Some((3, 86_400_000)),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransitionOutcome {
    Allowed,
    Blocked { retry_after_seconds: u32 },
}

/// The exact transition order required by RFC-078:
/// 1. absent or logically expired → start a new window at count 1, Allowed;
/// 2. only after expiry handling, an unexpired policy mismatch is an error;
/// 3. unexpired and below limit → increment, Allowed;
/// 4. otherwise → persist a saturating value no greater than `limit + 1`,
///    Blocked with retry-after clamped to `1..=window_seconds`.
fn transition(
    existing: Option<Row>,
    requested_policy: &str,
    now_ms: i64,
) -> Result<(Row, TransitionOutcome), ()> {
    let (limit, window_ms) = policy_limits(requested_policy).ok_or(())?;

    let Some(row) = existing else {
        return Ok((
            Row {
                policy: requested_policy.to_owned(),
                window_started_ms: now_ms,
                count: 1,
            },
            TransitionOutcome::Allowed,
        ));
    };

    let elapsed_ms = now_ms - row.window_started_ms;
    if elapsed_ms >= window_ms {
        return Ok((
            Row {
                policy: requested_policy.to_owned(),
                window_started_ms: now_ms,
                count: 1,
            },
            TransitionOutcome::Allowed,
        ));
    }

    if row.policy != requested_policy {
        return Err(());
    }

    if row.count < limit {
        return Ok((
            Row {
                count: row.count + 1,
                ..row
            },
            TransitionOutcome::Allowed,
        ));
    }

    let saturated_count = (row.count + 1).min(limit + 1);
    let window_seconds = (window_ms / 1000) as u32;
    let remaining_ms = window_ms - elapsed_ms;
    // `i64::div_ceil` is not stable; compute ceiling division manually.
    let retry_after_seconds =
        (((remaining_ms + 999) / 1000) as u32).clamp(1, window_seconds.max(1));

    Ok((
        Row {
            count: saturated_count,
            ..row
        },
        TransitionOutcome::Blocked {
            retry_after_seconds,
        },
    ))
}

// ── Durable Object glue (wasm32 only) ───────────────────────────────────────

#[cfg(target_arch = "wasm32")]
const CREATE_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS limiter_state (\
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1), \
    policy TEXT NOT NULL, \
    window_started_ms INTEGER NOT NULL, \
    count INTEGER NOT NULL CHECK (count >= 0)\
)";

/// Retention margin added beyond the window before scheduling cleanup.
/// Alarms are a storage-retention control, not part of correctness — the
/// authoritative expiry check runs again on every `reserve`.
#[cfg(target_arch = "wasm32")]
const ALARM_RETENTION_MARGIN_MS: i64 = 60_000;

#[cfg(target_arch = "wasm32")]
#[durable_object(alarm)]
pub struct AbuseLimiter {
    state: State,
}

#[cfg(target_arch = "wasm32")]
impl DurableObject for AbuseLimiter {
    fn new(state: State, _env: Env) -> Self {
        // Idempotent synchronous table creation; safe to run on every cold
        // start of the object.
        let _ = state.storage().sql().exec(CREATE_TABLE_SQL, None);
        Self { state }
    }

    async fn fetch(&self, req: Request) -> WorkerResult<Response> {
        match (req.method(), req.path().as_str()) {
            (Method::Post, "/v1/reserve") => self.handle_reserve(req).await,
            (Method::Post, "/v1/reset") => self.handle_reset(req).await,
            _ => Response::error("not found", 404),
        }
    }

    /// Retention-only cleanup. Deletes state only if it is logically
    /// expired at the time the alarm fires; a delayed or missed alarm never
    /// breaks correctness because `reserve`'s own expiry check is
    /// authoritative regardless of stored state age.
    async fn alarm(&self) -> WorkerResult<Response> {
        let sql = self.state.storage().sql();
        if let Ok(Some(row)) = read_row(&sql) {
            let now_ms = Date::now().as_millis() as i64;
            if let Some((_, window_ms)) = policy_limits(&row.policy) {
                if now_ms - row.window_started_ms >= window_ms {
                    let _ = sql.exec("DELETE FROM limiter_state WHERE singleton = 1", None);
                }
            }
        }
        Response::empty()
    }
}

#[cfg(target_arch = "wasm32")]
impl AbuseLimiter {
    async fn handle_reserve(&self, mut req: Request) -> WorkerResult<Response> {
        let policy = match read_bounded_policy(&mut req).await {
            Ok(p) => p,
            Err(()) => return Response::error("bad request", 400),
        };

        let now_ms = Date::now().as_millis() as i64;
        let sql = self.state.storage().sql();

        // Synchronous read → decide → write; no `.await` in this block.
        let existing = match read_row(&sql) {
            Ok(row) => row,
            Err(_) => return Response::error("storage error", 500),
        };
        let (next_row, outcome) = match transition(existing, &policy, now_ms) {
            Ok(t) => t,
            Err(()) => return Response::error("policy mismatch", 500),
        };
        if write_row(&sql, &next_row).is_err() {
            return Response::error("storage error", 500);
        }

        // Alarm scheduled only after the write has been persisted (H-N3).
        // If scheduling fails, the reservation charge is conservatively
        // retained and the caller sees `Unavailable`, never `Allowed`.
        let window_ms = policy_limits(&policy).map(|(_, w)| w).unwrap_or(0);
        let retention = Duration::from_millis((window_ms + ALARM_RETENTION_MARGIN_MS) as u64);
        if self.state.storage().set_alarm(retention).await.is_err() {
            return Response::error("alarm scheduling failed", 500);
        }

        let body = match outcome {
            TransitionOutcome::Allowed => {
                serde_json::json!({"outcome": "allowed", "retry_after_seconds": 0})
            }
            TransitionOutcome::Blocked {
                retry_after_seconds,
            } => {
                serde_json::json!({"outcome": "blocked", "retry_after_seconds": retry_after_seconds})
            }
        };
        Response::from_json(&body)
    }

    async fn handle_reset(&self, mut req: Request) -> WorkerResult<Response> {
        let policy = match read_bounded_policy(&mut req).await {
            Ok(p) => p,
            Err(()) => return Response::error("bad request", 400),
        };

        let sql = self.state.storage().sql();
        let existing = match read_row(&sql) {
            Ok(row) => row,
            Err(_) => return Response::error("storage error", 500),
        };
        match existing {
            None => {}
            Some(row) if row.policy == policy => {
                if sql
                    .exec("DELETE FROM limiter_state WHERE singleton = 1", None)
                    .is_err()
                {
                    return Response::error("storage error", 500);
                }
            }
            Some(_) => return Response::error("policy mismatch", 500),
        }
        Ok(Response::empty()?.with_status(204))
    }
}

// ── Wire protocol (private, same-Worker only) ───────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtocolRequestBody {
    policy: String,
}

#[cfg(target_arch = "wasm32")]
async fn read_bounded_policy(req: &mut Request) -> Result<String, ()> {
    let content_type = req.headers().get("Content-Type").ok().flatten();
    if !content_type.as_deref().is_some_and(is_json_media_type) {
        return Err(());
    }
    let text = req.text().await.map_err(|_| ())?;
    if text.is_empty() || text.len() > PROTOCOL_MAX_BODY_BYTES {
        return Err(());
    }
    let body: ProtocolRequestBody = serde_json::from_str(&text).map_err(|_| ())?;
    if policy_limits(&body.policy).is_some() {
        Ok(body.policy)
    } else {
        Err(())
    }
}

// ── SQLite state access ──────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct RowDe {
    policy: String,
    window_started_ms: i64,
    count: i64,
}

#[cfg(target_arch = "wasm32")]
fn read_row(sql: &SqlStorage) -> WorkerResult<Option<Row>> {
    let cursor = sql.exec(
        "SELECT policy, window_started_ms, count FROM limiter_state WHERE singleton = 1",
        None,
    )?;
    let rows: Vec<RowDe> = cursor.to_array()?;
    Ok(rows.into_iter().next().map(|r| Row {
        policy: r.policy,
        window_started_ms: r.window_started_ms,
        count: r.count,
    }))
}

#[cfg(target_arch = "wasm32")]
fn write_row(sql: &SqlStorage, row: &Row) -> WorkerResult<()> {
    let bindings = vec![
        SqlStorageValue::String(row.policy.clone()),
        SqlStorageValue::try_from_i64(row.window_started_ms)?,
        SqlStorageValue::try_from_i64(row.count)?,
    ];
    sql.exec(
        "INSERT INTO limiter_state (singleton, policy, window_started_ms, count) \
         VALUES (1, ?1, ?2, ?3) \
         ON CONFLICT(singleton) DO UPDATE SET \
           policy = excluded.policy, \
           window_started_ms = excluded.window_started_ms, \
           count = excluded.count",
        bindings,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
