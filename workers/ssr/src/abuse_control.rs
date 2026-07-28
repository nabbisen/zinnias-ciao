//! Fail-closed abuse-control coordinator client — RFC-078.
//!
//! Owns the caller side of the `AbuseLimiter` Durable Object: trusted
//! direct-edge ingress validation and client-network canonicalization,
//! HMAC-derived subject digesting (reusing RFC-077's validated pepper),
//! the private `/v1/reserve` + `/v1/reset` protocol client, and
//! outcome/telemetry mapping.
//!
//! This module never treats a missing binding, storage failure, or
//! malformed coordinator response as `Allowed`. See `abuse_limiter` for the
//! Durable Object implementation itself.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use worker::{Env, Headers, Method, Request, RequestInit, Response, Result as WorkerResult};

use crate::crypto::hmac_hex;

/// Bound on the private protocol request/response body size. Shared with
/// `abuse_limiter`, which enforces the same bound on the request side.
pub(crate) const PROTOCOL_MAX_BODY_BYTES: usize = 1024;

/// Closed policy scope. Stable labels used for HMAC domain separation and
/// Durable Object naming — never change these strings without a migration
/// note (RFC-078 §Subject derivation and object selection).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    Invite,
    Relink,
    CommunityUser,
    CommunitySession,
    CommunityNetwork,
}

impl Scope {
    pub const fn label(self) -> &'static str {
        match self {
            Scope::Invite => "invite",
            Scope::Relink => "relink",
            Scope::CommunityUser => "community-user",
            Scope::CommunitySession => "community-session",
            Scope::CommunityNetwork => "community-network",
        }
    }

    /// The policy identifier sent to the coordinator. The three community
    /// dimensions share one policy; invite and relink each have their own.
    const fn policy(self) -> &'static str {
        match self {
            Scope::Invite => "invite",
            Scope::Relink => "relink",
            Scope::CommunityUser | Scope::CommunitySession | Scope::CommunityNetwork => "community",
        }
    }

    /// Upper bound used to sanity-check a `Blocked` retry hint from the
    /// coordinator. The coordinator owns the authoritative window; this is a
    /// defensive ceiling only.
    const fn max_window_seconds(self) -> u32 {
        match self {
            Scope::Invite | Scope::Relink => 300,
            Scope::CommunityUser | Scope::CommunitySession | Scope::CommunityNetwork => 86_400,
        }
    }
}

/// Caller-facing coordinator outcome. Handlers may proceed to credential
/// lookup or mutation only on `Allowed`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Allowed,
    Blocked { retry_after_seconds: u32 },
    Unavailable { category: UnavailableCategory },
}

/// Stable, non-sensitive reason category for an `Unavailable` outcome —
/// safe for telemetry (RFC-078 §Observability and Alerting Contract).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnavailableCategory {
    MissingBinding,
    CoordinatorError,
    MalformedResponse,
}

// ── Trusted ingress and client-network identity ────────────────────────────

/// Stable, non-sensitive ingress rejection category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IngressRejection {
    UpstreamWorker,
    Ipv6HeaderPresent,
    InvalidAddress,
    ClassEAddress,
}

/// Pure header-shape classifier — natively unit testable. Takes already
/// extracted header values so it never requires constructing a real
/// `worker::Headers`/`Request` (which needs a JS environment).
///
/// `Headers::get` (the only accessor this module's wasm-boundary glue may
/// use — never `get_all`, see H-B1) already combines any repeated same-name
/// header into one comma-joined value per the Fetch standard, so a single
/// value covers both genuinely repeated headers and comma-joined values.
pub fn classify_ingress(
    cf_worker: Option<&str>,
    cf_connecting_ipv6: Option<&str>,
    cf_connecting_ip: Option<&str>,
) -> Result<String, IngressRejection> {
    if cf_worker.is_some() {
        return Err(IngressRejection::UpstreamWorker);
    }
    if cf_connecting_ipv6.is_some() {
        return Err(IngressRejection::Ipv6HeaderPresent);
    }

    let raw = cf_connecting_ip.ok_or(IngressRejection::InvalidAddress)?;
    if raw.is_empty() || raw.contains(',') || raw.chars().any(|c| c.is_ascii_whitespace()) {
        return Err(IngressRejection::InvalidAddress);
    }

    let addr: IpAddr = raw.parse().map_err(|_| IngressRejection::InvalidAddress)?;
    canonicalize(addr)
}

fn canonicalize(addr: IpAddr) -> Result<String, IngressRejection> {
    match addr {
        IpAddr::V4(v4) => reject_class_e(v4).map(|v4| v4.to_string()),
        IpAddr::V6(v6) => {
            // Normalize IPv4-mapped IPv6 to IPv4 *before* the Class E test
            // (H-B1 / design-review N1) — an `::ffff:240.0.0.1` must be
            // rejected, not passed through as an opaque IPv6 literal.
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return reject_class_e(mapped).map(|v4| v4.to_string());
            }
            Ok(zero_low_64_bits(v6).to_string())
        }
    }
}

fn reject_class_e(v4: Ipv4Addr) -> Result<Ipv4Addr, IngressRejection> {
    if v4.octets()[0] >= 240 {
        Err(IngressRejection::ClassEAddress)
    } else {
        Ok(v4)
    }
}

fn zero_low_64_bits(v6: Ipv6Addr) -> Ipv6Addr {
    let s = v6.segments();
    Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0)
}

/// Wasm-boundary glue: reads the trusted headers off a real inbound request
/// and delegates to `classify_ingress`. Exercised by the isolated workerd
/// harness, not natively unit tested — constructing a real `worker::Request`
/// requires a JS environment.
pub fn canonical_client_network(req: &Request) -> Result<String, IngressRejection> {
    let headers = req.headers();
    classify_ingress(
        headers.get("CF-Worker").ok().flatten().as_deref(),
        headers.get("CF-Connecting-IPv6").ok().flatten().as_deref(),
        headers.get("CF-Connecting-IP").ok().flatten().as_deref(),
    )
}

// ── Subject derivation and object selection ─────────────────────────────────

/// `HMAC-SHA256(pepper, "abuse-control:v1:" + scope + ":" + canonical_subject)`.
/// The raw subject is used only long enough to derive this digest; it must
/// never cross the coordinator, telemetry, or evidence boundary.
pub fn subject_digest(pepper: &str, scope: Scope, canonical_subject: &str) -> String {
    hmac_hex(
        pepper,
        &format!("abuse-control:v1:{}:{}", scope.label(), canonical_subject),
    )
}

fn object_name(scope: Scope, digest: &str) -> String {
    format!("v1:{}:{}", scope.label(), digest)
}

// ── Private same-Worker protocol client ─────────────────────────────────────

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ReserveBody {
    outcome: String,
    retry_after_seconds: u32,
}

/// Reserve capacity for one scope/subject. Never returns `Allowed` on any
/// binding, protocol, or storage failure.
pub async fn reserve(env: &Env, pepper: &str, scope: Scope, canonical_subject: &str) -> Outcome {
    match reserve_inner(env, pepper, scope, canonical_subject).await {
        Ok(outcome) => outcome,
        Err(category) => Outcome::Unavailable { category },
    }
}

async fn reserve_inner(
    env: &Env,
    pepper: &str,
    scope: Scope,
    canonical_subject: &str,
) -> Result<Outcome, UnavailableCategory> {
    let mut resp = call_protocol(env, pepper, scope, canonical_subject, "/v1/reserve").await?;
    let status = resp.status_code();
    let text = resp
        .text()
        .await
        .map_err(|_| UnavailableCategory::MalformedResponse)?;
    parse_reserve_outcome(status, &text, scope)
}

/// Pure, natively testable parsing of the `/v1/reserve` response. Strict:
/// wrong status, oversized/empty body, unknown fields, or an unexpected
/// `outcome`/`retry_after_seconds` value all map to `Unavailable`, never to
/// `Allowed`. A `5xx` status is reported as `CoordinatorError` (the Durable
/// Object itself failed) rather than `MalformedResponse` (a parse/shape
/// failure), so incident triage can distinguish the two categories.
fn parse_reserve_outcome(
    status: u16,
    body: &str,
    scope: Scope,
) -> Result<Outcome, UnavailableCategory> {
    if (500..600).contains(&status) {
        return Err(UnavailableCategory::CoordinatorError);
    }
    if status != 200 {
        return Err(UnavailableCategory::MalformedResponse);
    }
    if body.is_empty() || body.len() > PROTOCOL_MAX_BODY_BYTES {
        return Err(UnavailableCategory::MalformedResponse);
    }
    let parsed: ReserveBody =
        serde_json::from_str(body).map_err(|_| UnavailableCategory::MalformedResponse)?;

    match parsed.outcome.as_str() {
        "allowed" if parsed.retry_after_seconds == 0 => Ok(Outcome::Allowed),
        "blocked" => {
            let max = scope.max_window_seconds();
            if parsed.retry_after_seconds == 0 || parsed.retry_after_seconds > max {
                return Err(UnavailableCategory::MalformedResponse);
            }
            Ok(Outcome::Blocked {
                retry_after_seconds: parsed.retry_after_seconds,
            })
        }
        _ => Err(UnavailableCategory::MalformedResponse),
    }
}

/// Attempt to reset a scope/subject's window after a valid credential
/// result. Best-effort: failure emits `abuse_control.reset_failed` and does
/// not affect the already-successful application operation (the preceding
/// reservation is what keeps this fail-safe rather than fail-open).
pub async fn reset(env: &Env, rid: &str, pepper: &str, scope: Scope, canonical_subject: &str) {
    let result = async {
        let resp = call_protocol(env, pepper, scope, canonical_subject, "/v1/reset").await?;
        if resp.status_code() == 204 {
            Ok(())
        } else {
            Err(UnavailableCategory::MalformedResponse)
        }
    }
    .await;
    if result.is_err() {
        worker::console_log!(
            "event=abuse_control.reset_failed request_id={} scope={}",
            rid,
            scope.label()
        );
    }
}

async fn call_protocol(
    env: &Env,
    pepper: &str,
    scope: Scope,
    canonical_subject: &str,
    path: &str,
) -> Result<Response, UnavailableCategory> {
    let digest = subject_digest(pepper, scope, canonical_subject);
    let name = object_name(scope, &digest);

    let namespace = env
        .durable_object("ABUSE_LIMITER")
        .map_err(|_| UnavailableCategory::MissingBinding)?;
    let stub = namespace
        .get_by_name(&name)
        .map_err(|_| UnavailableCategory::MissingBinding)?;

    let body = serde_json::json!({ "policy": scope.policy() }).to_string();
    let req = protocol_request(path, &body).map_err(|_| UnavailableCategory::CoordinatorError)?;

    stub.fetch_with_request(req)
        .await
        .map_err(|_| UnavailableCategory::CoordinatorError)
}

fn protocol_request(path: &str, json_body: &str) -> WorkerResult<Request> {
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_headers(headers);
    init.with_body(Some(worker::wasm_bindgen::JsValue::from_str(json_body)));
    Request::new_with_init(&format!("https://abuse-limiter.internal{path}"), &init)
}

// ── Response helpers ─────────────────────────────────────────────────────────

/// Apply the fixed `429` status and bounded `Retry-After` header to an
/// already-rendered form response. `Cache-Control: no-store` and security
/// headers are applied globally by the Worker's outer response hook.
pub fn apply_blocked(resp: Response, retry_after_seconds: u32) -> WorkerResult<Response> {
    let mut resp = resp.with_status(429);
    resp.headers_mut()
        .set("Retry-After", &retry_after_seconds.to_string())?;
    Ok(resp)
}

// ── Telemetry ────────────────────────────────────────────────────────────────
//
// Bounded, privacy-safe fields only: request ID, route/surface, subject
// dimension category, outcome/reason category. Never the raw or canonical
// client address, IPv6 prefix, digest, object name, user/session ID,
// credential, token, cookie, request body, SQL, or protocol body.

pub fn log_blocked(rid: &str, route: &str, scope: Scope) {
    worker::console_log!(
        "event=abuse_control.blocked request_id={} route={} scope={}",
        rid,
        route,
        scope.label()
    );
}

pub fn log_unavailable(rid: &str, route: &str, scope: Scope, category: UnavailableCategory) {
    worker::console_log!(
        "event=abuse_control.unavailable request_id={} route={} scope={} category={:?}",
        rid,
        route,
        scope.label(),
        category
    );
}

pub fn log_ingress_rejected(rid: &str, route: &str, rejection: IngressRejection) {
    worker::console_log!(
        "event=abuse_control.unavailable request_id={} route={} category=ingress_rejected reason={:?}",
        rid,
        route,
        rejection
    );
}

#[cfg(test)]
mod tests;
