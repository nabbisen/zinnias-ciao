use worker::{Env, Response, Result};

pub async fn get_health(env: &Env) -> Result<Response> {
    if crate::crypto::pepper(env).is_ok() {
        Response::from_json(
            &serde_json::json!({"ok": true, "ready": true, "service": "ciao.zinnias"}),
        )
    } else {
        Ok(Response::from_json(
            &serde_json::json!({"ok": false, "ready": false, "service": "ciao.zinnias"}),
        )?
        .with_status(503))
    }
}

pub async fn get_version(env: &Env) -> Result<Response> {
    let version = env
        .var("BUILD_VERSION")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "dev".to_string());
    Response::from_json(&serde_json::json!({"ok": true, "version": version}))
}
