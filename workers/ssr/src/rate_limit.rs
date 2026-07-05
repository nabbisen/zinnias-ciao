//! Invite-code rate limiting via Cloudflare KV (RFC-012 §5).
//!
//! Tracks failed redemption attempts per IP using KV with a short TTL.
//! Generic errors are returned — never revealing whether a code was valid.
//!
//! KV unavailability fails open so invite-only onboarding is not blocked by
//! transient rate-limit storage issues.

use worker::Env;

const MAX_FAILURES: u32 = 10;
const WINDOW_SECONDS: u64 = 300; // 5-minute window
pub const COMMUNITY_CREATION_MAX_PER_WINDOW: u32 = 3;
const COMMUNITY_CREATION_WINDOW_SECONDS: u64 = 86_400; // 24 hours

/// Check if the given IP is rate-limited for invite redemption.
/// Returns `true` (blocked) or `false` (allowed).
pub async fn is_rate_limited(env: &Env, ip: &str) -> bool {
    let Ok(kv) = env.kv("RATE_LIMIT") else {
        return false;
    };
    let key = format!("invite_fail:{ip}");
    match kv.get(&key).text().await {
        Ok(Some(val)) => val.trim().parse::<u32>().unwrap_or(0) >= MAX_FAILURES,
        _ => false,
    }
}

/// Record a failed invite attempt for the given IP.
pub async fn record_failure(env: &Env, ip: &str) {
    let Ok(kv) = env.kv("RATE_LIMIT") else { return };
    let key = format!("invite_fail:{ip}");
    let current = match kv.get(&key).text().await {
        Ok(Some(v)) => v.trim().parse::<u32>().unwrap_or(0),
        _ => 0,
    };
    let Ok(put) = kv.put(&key, (current + 1).to_string()) else {
        return;
    };
    let _ = put.expiration_ttl(WINDOW_SECONDS).execute().await;
}

/// Clear the failure counter on successful redemption.
pub async fn clear_failures(env: &Env, ip: &str) {
    let Ok(kv) = env.kv("RATE_LIMIT") else { return };
    let key = format!("invite_fail:{ip}");
    let _ = kv.delete(&key).await;
}

pub async fn is_community_creation_limited(
    env: &Env,
    user_id: &str,
    session_id: &str,
    ip: &str,
) -> bool {
    let Ok(kv) = env.kv("RATE_LIMIT") else {
        return false;
    };

    for key in community_creation_keys(user_id, session_id, ip) {
        match kv.get(&key).text().await {
            Ok(Some(val))
                if val.trim().parse::<u32>().unwrap_or(0) >= COMMUNITY_CREATION_MAX_PER_WINDOW =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

pub async fn record_community_creation(env: &Env, user_id: &str, session_id: &str, ip: &str) {
    let Ok(kv) = env.kv("RATE_LIMIT") else { return };

    for key in community_creation_keys(user_id, session_id, ip) {
        let current = match kv.get(&key).text().await {
            Ok(Some(v)) => v.trim().parse::<u32>().unwrap_or(0),
            _ => 0,
        };
        let Ok(put) = kv.put(&key, (current + 1).to_string()) else {
            continue;
        };
        let _ = put
            .expiration_ttl(COMMUNITY_CREATION_WINDOW_SECONDS)
            .execute()
            .await;
    }
}

fn community_creation_keys(user_id: &str, session_id: &str, ip: &str) -> [String; 3] {
    [
        format!("community_create_user:{user_id}"),
        format!("community_create_session:{session_id}"),
        format!("community_create_ip:{ip}"),
    ]
}

/// Extract the best-effort client IP from request headers.
pub fn client_ip(req: &worker::Request) -> String {
    req.headers()
        .get("CF-Connecting-IP")
        .ok()
        .flatten()
        .or_else(|| {
            req.headers()
                .get("X-Forwarded-For")
                .ok()
                .flatten()
                .map(|v| v.split(',').next().unwrap_or("").trim().to_owned())
        })
        .unwrap_or_else(|| "unknown".to_string())
}
