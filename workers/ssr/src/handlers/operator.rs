//! Operator-only recovery endpoints — RFC-069.

use serde::Deserialize;
use worker::{Env, Request, Response, Result, console_log};

use crate::crypto::{constant_time_eq, hmac_hex, normalize_invite_code, random_token};
use crate::db::{community as community_db, membership as membership_db, relink as relink_db};
use crate::render;

const OPERATOR_LABEL_MAX_CHARS: usize = 80;

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
    let code_hmac = hmac_hex(&crate::crypto::pepper(env), &normalized);
    let relink_code_id = random_token()[..24].to_owned();
    let audit_id = random_token()[..16].to_owned();
    let now = crate::db::now_utc();
    let expires_at = relink_db::expires_at();
    let metadata = serde_json::json!({
        "operator_label": body.operator_label,
        "relink_code_id": relink_code_id,
        "membership_id": target.id,
        "community_id": target.community_id,
    })
    .to_string();

    let revoke_stmt = db
        .prepare(
            "UPDATE membership_relink_codes \
             SET revoked_at = ?1 \
             WHERE membership_id = ?2 \
               AND used_at IS NULL \
               AND revoked_at IS NULL \
               AND expires_at > ?1",
        )
        .bind(&[now.as_str().into(), target.id.as_str().into()])?;

    let insert_relink_stmt = db
        .prepare(
            "INSERT INTO membership_relink_codes \
             (id, code_hmac, community_id, membership_id, created_by_membership_id, created_at, expires_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(&[
            relink_code_id.as_str().into(),
            code_hmac.as_str().into(),
            target.community_id.as_str().into(),
            target.id.as_str().into(),
            target.id.as_str().into(),
            now.as_str().into(),
            expires_at.as_str().into(),
        ])?;

    let audit_stmt = db
        .prepare(
            "INSERT INTO audit_log \
             (id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at) \
             VALUES (?1, ?2, ?3, 'membership', ?4, 'operator_recovery.admin_relink_created', ?5, ?6)",
        )
        .bind(&[
            audit_id.as_str().into(),
            target.community_id.as_str().into(),
            target.id.as_str().into(),
            target.id.as_str().into(),
            metadata.as_str().into(),
            now.as_str().into(),
        ])?;

    db.batch(vec![revoke_stmt, insert_relink_stmt, audit_stmt])
        .await?;

    console_log!(
        "[{}] audit: action=operator_recovery.admin_relink_created target=membership:{} actor={} community={}",
        rid,
        target.id,
        target.id,
        target.community_id,
    );

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
    let trimmed = label.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= OPERATOR_LABEL_MAX_CHARS
        && trimmed.chars().all(|c| !c.is_control())
}

#[cfg(test)]
mod tests;
