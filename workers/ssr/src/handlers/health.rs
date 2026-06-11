use worker::{Env, Response, Result};

pub async fn get_health(_env: &Env) -> Result<Response> {
    Response::from_json(&serde_json::json!({"ok": true, "service": "ciao.zinnias"}))
}

pub async fn get_version(env: &Env) -> Result<Response> {
    let version = env
        .var("BUILD_VERSION")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "dev".to_string());
    Response::from_json(&serde_json::json!({"ok": true, "version": version}))
}
