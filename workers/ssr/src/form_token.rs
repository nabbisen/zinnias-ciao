//! Server-issued form tokens — AD-4 / RFC-002.
//!
//! Every state-changing form embeds a single-use token that provides:
//!   1. CSRF protection (session-bound; SameSite=Strict alone is sufficient
//!      on modern browsers, but the token adds defence-in-depth and
//!      Origin-check parity on older browsers — RFC-012).
//!   2. Idempotency (consumed on first success; replay returns a benign no-op).
//!
//! No client-generated mutation_id; no client_mutations table.

use worker::{D1Database, Result};
use zinnias_ciao_contracts::FORM_TOKEN_TTL_SECONDS;

use crate::crypto::{hmac_hex, random_token};
use crate::db::{add_seconds_to_now, now_utc};

/// Issue a new form token, insert it into `form_tokens`, and return
/// the raw secret (to be embedded in the rendered form as a hidden field).
pub async fn issue(
    db: &D1Database,
    pepper: &str,
    user_id: &str,
    purpose: &str,
    bound_resource: Option<&str>,
) -> Result<String> {
    let secret = random_token();
    let token_hmac = hmac_hex(pepper, &secret);
    let now = now_utc();
    let expires_at = add_seconds_to_now(FORM_TOKEN_TTL_SECONDS);

    db.prepare(
        "INSERT INTO form_tokens \
         (token_hmac, user_id, purpose, bound_resource, issued_at, expires_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&[
        token_hmac.as_str().into(),
        user_id.into(),
        purpose.into(),
        bound_resource.unwrap_or("").into(),
        now.as_str().into(),
        expires_at.as_str().into(),
    ])?
    .run()
    .await?;

    Ok(secret)
}

/// Validate and consume a form token submitted with a POST.
///
/// - Checks the HMAC matches a row for this user and purpose.
/// - Checks the token has not expired.
/// - Checks (if provided) that `bound_resource` matches.
/// - On success, marks `consumed_at` atomically.
/// - A previously consumed token returns `Err(TokenConsumed)` — the caller
///   should return the prior result ref if available (idempotency).
pub async fn consume(
    db: &D1Database,
    pepper: &str,
    user_id: &str,
    purpose: &str,
    raw_token: &str,
    bound_resource: Option<&str>,
) -> Result<Option<String>> {
    // Returns the prior result_ref if already consumed (idempotent replay).
    let now = now_utc();
    let token_hmac = hmac_hex(pepper, raw_token);

    // Atomic conditional consume: mark consumed only if currently unconsumed,
    // matching subject + purpose + binding + not expired. The affected-row
    // count tells us whether THIS call won the race. Two concurrent submits
    // cannot both observe changes == 1.
    let bound_match = bound_resource.unwrap_or("");
    let update = db
        .prepare(
            "UPDATE form_tokens SET consumed_at = ?1 \
             WHERE token_hmac = ?2 \
               AND user_id = ?3 \
               AND purpose = ?4 \
               AND expires_at > ?5 \
               AND consumed_at IS NULL \
               AND COALESCE(bound_resource, '') = ?6",
        )
        .bind(&[
            now.as_str().into(),
            token_hmac.as_str().into(),
            user_id.into(),
            purpose.into(),
            now.as_str().into(),
            bound_match.into(),
        ])?
        .run()
        .await?;

    let changed = update
        .meta()
        .ok()
        .flatten()
        .and_then(|m| m.changes)
        .unwrap_or(0);

    // Fast path: won the atomic race.
    use zinnias_ciao_contracts::auth::{TokenConsumeOutcome, classify_token_consume};
    if changed == 1 {
        return Ok(None);
    }

    // changed == 0: classify why via a follow-up SELECT (no race-sensitive write).
    let row = db
        .prepare(
            "SELECT consumed_at, result_ref, bound_resource, expires_at \
             FROM form_tokens \
             WHERE token_hmac = ?1 AND user_id = ?2 AND purpose = ?3 \
             LIMIT 1",
        )
        .bind(&[token_hmac.as_str().into(), user_id.into(), purpose.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let found = row.is_some();
    let already_consumed = row
        .as_ref()
        .and_then(|r| r.get("consumed_at").and_then(|v| v.as_str()))
        .is_some();
    let binding_ok = match (bound_resource, row.as_ref()) {
        (Some(expected), Some(r)) => {
            r.get("bound_resource")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                == expected
        }
        (None, _) => true,
        (Some(_), None) => false,
    };

    let invalid = || {
        worker::Error::RustError(
            "This action could not be completed. Please try again.".to_string(),
        )
    };

    match classify_token_consume(changed, found, already_consumed, binding_ok) {
        TokenConsumeOutcome::Proceed => Ok(None), // unreachable for changed==0, but safe
        TokenConsumeOutcome::Replay => {
            let result_ref = row
                .as_ref()
                .and_then(|r| r.get("result_ref").and_then(|v| v.as_str()))
                .map(|s| s.to_owned());
            Ok(result_ref)
        }
        TokenConsumeOutcome::Invalid => Err(invalid()),
    }
}

/// Store the result ref on a consumed token (for idempotency replay).
///
/// Written as infrastructure for RFC-037 §4 (full idempotency replay: a retry
/// returns the stored result without re-executing the mutation). Not called in
/// the pilot — current handlers redirect on token replay rather than replaying
/// the result. Retained for the integration harness (RFC-044) and future use.
#[allow(dead_code)]
pub async fn set_result(
    db: &D1Database,
    pepper: &str,
    raw_token: &str,
    result_ref: &str,
) -> Result<()> {
    let token_hmac = hmac_hex(pepper, raw_token);
    db.prepare("UPDATE form_tokens SET result_ref = ?1 WHERE token_hmac = ?2")
        .bind(&[result_ref.into(), token_hmac.as_str().into()])?
        .run()
        .await?;
    Ok(())
}
