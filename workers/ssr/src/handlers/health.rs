use serde::Serialize;
use worker::{Env, Response, Result};

#[cfg(target_arch = "wasm32")]
use worker::WorkerVersionMetadata;

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

// Strict, pinned schema (RFC-050 S1). `worker_version_id`/`worker_version_tag`
// are the Cloudflare-assigned immutable version identity, not a secret or
// binding name — they are the exact-candidate identity a hosted evidence
// campaign checks this route for. They are `None` wherever the
// `version_metadata` binding is absent, such as native tests or an
// unconfigured local dev run.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct VersionResponse {
    ok: bool,
    version: String,
    worker_version_id: Option<String>,
    worker_version_tag: Option<String>,
}

impl VersionResponse {
    fn new(
        version: String,
        worker_version_id: Option<String>,
        worker_version_tag: Option<String>,
    ) -> Self {
        Self {
            ok: true,
            version,
            worker_version_id,
            worker_version_tag,
        }
    }
}

pub async fn get_version(env: &Env) -> Result<Response> {
    let version = env
        .var("BUILD_VERSION")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "dev".to_string());
    let (worker_version_id, worker_version_tag) = read_worker_version_metadata(env);
    Response::from_json(&VersionResponse::new(
        version,
        worker_version_id,
        worker_version_tag,
    ))
}

#[cfg(target_arch = "wasm32")]
fn read_worker_version_metadata(env: &Env) -> (Option<String>, Option<String>) {
    match env.get_binding::<WorkerVersionMetadata>("CF_VERSION_METADATA") {
        Ok(meta) => (Some(meta.id()), Some(meta.tag())),
        Err(_) => (None, None),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_worker_version_metadata(_env: &Env) -> (Option<String>, Option<String>) {
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_response_schema_is_pinned_with_metadata_present() {
        let resp = VersionResponse::new(
            "dev".to_string(),
            Some("abcd1234-ef56-7890-abcd-1234ef567890".to_string()),
            Some("blue".to_string()),
        );
        let value = serde_json::to_value(&resp).expect("serializes");
        assert_eq!(
            value,
            serde_json::json!({
                "ok": true,
                "version": "dev",
                "worker_version_id": "abcd1234-ef56-7890-abcd-1234ef567890",
                "worker_version_tag": "blue",
            })
        );
    }

    #[test]
    fn version_response_schema_is_pinned_with_metadata_absent() {
        let resp = VersionResponse::new("dev".to_string(), None, None);
        let value = serde_json::to_value(&resp).expect("serializes");
        assert_eq!(
            value,
            serde_json::json!({
                "ok": true,
                "version": "dev",
                "worker_version_id": null,
                "worker_version_tag": null,
            })
        );
    }

    #[test]
    fn version_response_never_grows_an_undeclared_field() {
        let resp = VersionResponse::new("dev".to_string(), None, None);
        let value = serde_json::to_value(&resp).expect("serializes");
        let obj = value.as_object().expect("object");
        assert_eq!(
            obj.len(),
            4,
            "the /version schema gained or lost a field: {obj:?}"
        );
    }

    #[test]
    fn version_response_retains_the_existing_version_field_contract() {
        // `scripts/runtime-smoke.mjs` asserts `json.version === expectedVersion`;
        // the field name and its independence from the new metadata fields must
        // not change.
        let resp = VersionResponse::new("staging".to_string(), None, None);
        assert_eq!(resp.version, "staging");
    }
}
