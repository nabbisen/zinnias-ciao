//! Operator-only recovery endpoints — RFC-069.

use serde::Deserialize;
use worker::{Env, Request, Response, Result};

use crate::crypto::{constant_time_eq, hmac_hex, normalize_invite_code, random_token};
use crate::db::{community as community_db, membership as membership_db, relink as relink_db};
use crate::render;

const OPERATOR_LABEL_MAX_BYTES: usize = 32;

#[derive(Deserialize)]
struct CommunityAccessRecoveryRequest {
    community_id: String,
    admin_membership_id: String,
    operator_label: String,
}

pub async fn post_community_access_recovery(
    mut req: Request,
    env: &Env,
    rid: &str,
) -> Result<Response> {
    if !operator_recovery_enabled(env) || !authorized(&req, env) {
        return render::not_found();
    }

    let body = match req.json::<CommunityAccessRecoveryRequest>().await {
        Ok(body) => body,
        Err(_) => return render::not_found(),
    };
    if !valid_operator_label(&body.operator_label) {
        return render::not_found();
    }

    let db = env.d1("DB")?;
    if community_db::find_active(&db, &body.community_id)
        .await?
        .is_none()
    {
        return render::not_found();
    }

    let target =
        match membership_db::find_active_by_id(&db, &body.admin_membership_id, &body.community_id)
            .await?
        {
            Some(target) if target.role == "admin" => target,
            _ => return render::not_found(),
        };

    let code = random_token()[..16].to_ascii_uppercase();
    let normalized = normalize_invite_code(&code);
    let pepper = crate::crypto::pepper(env)?;
    let code_hmac = hmac_hex(pepper.as_str(), &normalized);
    let relink_code_id = random_token()[..24].to_owned();
    let expires_at = relink_db::expires_at();
    if !relink_db::issue_required(
        &db,
        rid,
        &relink_code_id,
        &code_hmac,
        &target.community_id,
        &target.id,
        &target.id,
        &expires_at,
        Some(body.operator_label),
    )
    .await?
    {
        return render::not_found();
    }

    let mut resp = Response::from_json(&serde_json::json!({
        "ok": true,
        "community_id": target.community_id,
        "admin_membership_id": target.id,
        "expires_at": expires_at,
        "relink_code": code,
    }))?;
    resp.headers_mut()
        .set("Cache-Control", "no-store, private")?;
    Ok(resp)
}

fn operator_recovery_enabled(env: &Env) -> bool {
    env.var("COMMUNITY_RECOVERY_ENABLED")
        .ok()
        .map(|v| v.to_string() == "true")
        .unwrap_or(false)
}

fn authorized(req: &Request, env: &Env) -> bool {
    let Ok(secret) = env.secret("COMMUNITY_RECOVERY_TOKEN") else {
        return false;
    };
    let secret = secret.to_string();
    if secret.is_empty() {
        return false;
    }

    let Some(token) = bearer_token(req) else {
        return false;
    };
    constant_time_eq(&token, &secret)
}

fn bearer_token(req: &Request) -> Option<String> {
    req.headers()
        .get("Authorization")
        .ok()
        .flatten()
        .and_then(|value| value.strip_prefix("Bearer ").map(str::to_owned))
        .filter(|token| !token.is_empty())
}

fn valid_operator_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    (1..=OPERATOR_LABEL_MAX_BYTES).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(byte))
}

#[cfg(test)]
mod tests;
