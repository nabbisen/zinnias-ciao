//! The local fake issuer's HTTP surface — RFC-080 §10 (Handoff 054 §3).
//!
//! Exists only when the `dev_fake_issuer` cargo feature is enabled. Off by
//! default: `bun run build` (production, staging, and every one of the ten
//! pre-existing smokes) never passes it, so this entire file — its routes,
//! its signing key, and the namespace resolver's branch that recognises
//! `idns_local_fake` (see `identity::resolve_namespace_verification`) — is
//! absent from a production build's compiled wasm, not merely unreachable
//! at runtime. `identity_dev_fake_issuer_absent_from_production_build` in
//! `release_gates.rs` proves this from the artifact itself.
//!
//! Simulates the whole Authorization Code + PKCE S256 round trip with no
//! real user interaction: `GET authorize` auto-approves a single fixed
//! test subject and redirects straight back with a code; `POST token`
//! exchanges it. This is a test double for an external provider, not an
//! application-owned route — RFC-080 §9's no-JS requirement governs
//! application routes (`handlers/identity/`), not a stand-in for a
//! provider's own pages.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use worker::{Env, Request, Response, Result};

use super::jwt::encode_hs256;

pub(crate) const ISSUER: &str = "https://fake-issuer.local.test";
pub(crate) const AUDIENCE: &str = "zinnias-ciao-dev-fake-client";
pub(crate) const KID: &str = "dev-fake-key-1";
/// The one simulated external identity this issuer ever authenticates.
/// Fixed and hardcoded because there is no real login form to choose one —
/// the smoke seeds `user_identities` rows against this subject's digest to
/// set up "known identity" vs. "unknown identity" scenarios.
pub(crate) const FAKE_SUBJECT: &str = "dev-fake-subject-1";

/// Generated once per worker-isolate lifetime via `getrandom`, never
/// committed (Handoff 053 §10 / Handoff 054 §11's evidence constraint).
/// Shared between signing (`post_token` below) and verification
/// (`identity::resolve_namespace_verification`) — both only exist under
/// this same feature, so both see the same key within one running
/// `wrangler dev` process, which is all a local smoke needs.
pub(super) fn shared_key() -> Vec<u8> {
    static KEY: OnceLock<Vec<u8>> = OnceLock::new();
    KEY.get_or_init(|| {
        let mut key = vec![0u8; 32];
        getrandom::fill(&mut key).expect("getrandom failed");
        key
    })
    .clone()
}

struct PendingCode {
    redirect_uri: String,
    code_challenge: String,
    nonce: String,
}

fn code_store() -> &'static Mutex<HashMap<String, PendingCode>> {
    static STORE: OnceLock<Mutex<HashMap<String, PendingCode>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn query_param(url: &worker::Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.into_owned())
}

/// `GET /dev/identity/fake-issuer/authorize` — auto-approves, no form, no
/// interstitial: this issuer has no concept of a login screen. Mints a
/// single-use code bound to this request's `redirect_uri`/`code_challenge`/
/// `nonce`, and 303s straight back with `code` and the caller's own
/// `state` echoed unchanged (this issuer never inspects `state` beyond
/// echoing it — the application's transaction lookup is what validates it
/// on return).
pub(crate) async fn get_authorize(req: Request, _env: &Env, _rid: &str) -> Result<Response> {
    let url = req.url()?;
    let Some(redirect_uri) = query_param(&url, "redirect_uri") else {
        return Response::error("missing redirect_uri", 400);
    };
    let Some(state) = query_param(&url, "state") else {
        return Response::error("missing state", 400);
    };
    let Some(nonce) = query_param(&url, "nonce") else {
        return Response::error("missing nonce", 400);
    };
    let Some(code_challenge) = query_param(&url, "code_challenge") else {
        return Response::error("missing code_challenge", 400);
    };
    if query_param(&url, "code_challenge_method").as_deref() != Some("S256") {
        return Response::error("unsupported code_challenge_method", 400);
    }

    let mut code_bytes = [0u8; 32];
    getrandom::fill(&mut code_bytes).expect("getrandom failed");
    let code = hex::encode(code_bytes);

    code_store().lock().expect("code store poisoned").insert(
        code.clone(),
        PendingCode {
            redirect_uri: redirect_uri.clone(),
            code_challenge,
            nonce,
        },
    );

    let separator = if redirect_uri.contains('?') { '&' } else { '?' };
    let destination = format!(
        "{redirect_uri}{separator}code={code}&state={state}",
        code = urlencode(&code),
        state = urlencode(&state),
    );
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", &destination)?;
    Ok(resp)
}

/// `POST /dev/identity/fake-issuer/token` — validates the code (single-use:
/// removed from the store on first use, so a replayed code is simply
/// unknown the second time), the exact `redirect_uri`, and the PKCE S256
/// challenge, then mints and signs an ID token for `FAKE_SUBJECT`.
pub(crate) async fn post_token(mut req: Request, _env: &Env, _rid: &str) -> Result<Response> {
    let body = req.form_data().await?;
    let (Some(code), Some(redirect_uri), Some(code_verifier)) = (
        body.get_field("code"),
        body.get_field("redirect_uri"),
        body.get_field("code_verifier"),
    ) else {
        return Response::error("missing parameter", 400);
    };

    let pending = code_store()
        .lock()
        .expect("code store poisoned")
        .remove(&code);
    let Some(pending) = pending else {
        return Response::error("unknown or already-used code", 400);
    };
    if pending.redirect_uri != redirect_uri {
        return Response::error("redirect_uri mismatch", 400);
    }

    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let computed_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    if computed_challenge != pending.code_challenge {
        return Response::error("PKCE verification failed", 400);
    }

    let now = (worker::Date::now().as_millis() / 1000) as i64;
    let header = serde_json::json!({ "alg": "HS256", "typ": "JWT", "kid": KID });
    let claims = serde_json::json!({
        "iss": ISSUER,
        "aud": AUDIENCE,
        "sub": FAKE_SUBJECT,
        "nonce": pending.nonce,
        "exp": now + 300,
        "iat": now,
    });
    let id_token = encode_hs256(&header, &claims, &shared_key());

    Response::from_json(&serde_json::json!({
        "id_token": id_token,
        "token_type": "Bearer",
        "expires_in": 300,
    }))
}

fn urlencode(value: &str) -> String {
    // Only the characters actually present in generated code/state values
    // (hex digest / random-token alphabet) need escaping here; kept
    // minimal and explicit rather than pulling in a general percent-
    // encoding dependency for a dev-only route.
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
