//! External identity sign-in/join routes — RFC-080 §5 (Handoff 054).
//!
//! GET /identity/start    — begins a transaction, redirects to the provider.
//! GET /identity/callback — the nine-step callback contract (RFC-080 §5.1).
//!
//! Every route here is a plain SSR GET (AD-1, no application JavaScript):
//! the authorization redirect is a top-level navigation, never a fetch or
//! form post, so the existing CSP (`form-action 'self'`) is unaffected —
//! nothing here changes it.

use worker::{Env, Headers, Method, Request, RequestInit, Response, Result};
use zinnias_ciao_contracts::i18n;

use crate::db;
use crate::render;

const CALLBACK_PATH: &str = "/identity/callback";
/// A login round trip, not a session — short-lived on purpose.
const TRANSACTION_TTL_SECONDS: u64 = 600;
/// The only namespace this slice can ever start a transaction against —
/// see `identity::resolve_namespace_verification` for why this is a
/// constant rather than a selectable value: no real provider is
/// registered yet, and there is exactly one candidate.
const NAMESPACE_ID: &str = "idns_local_fake";
const CLIENT_ID: &str = "zinnias-ciao-dev-fake-client";

/// RFC-080 §5.2: the only destinations a completed sign-in/join may land
/// on. Never taken from a request parameter — resolved from the
/// transaction row's own `return_to`, and re-validated against this list
/// regardless of what is stored, since a stored value must never be
/// trusted as a redirect target without its own check (defence in depth
/// against a future caller of `db::auth_transaction::insert_required`
/// passing something unsafe).
///
/// `/account` (Handoff 055 §5.4) is the first growth of this list since
/// its introduction: no live call site sets `return_to` to it yet — every
/// `insert_required` call in this file still passes `Some("/")` — a
/// working "re-authenticate, land back on /account" path needs
/// `get_start`'s own already-authenticated short-circuit (below) to change
/// first, which is session-rotation-adjacent territory this package
/// deliberately leaves to Slice 5b (see the review request for the full
/// reasoning). Added now, and exercised by its own test, so 5b extends an
/// already-reviewed allowlist rather than growing it and wiring a live
/// producer in the same, more sensitive package.
const ALLOWED_RETURN_DESTINATIONS: &[&str] = &["/", "/account"];

fn resolve_safe_return(stored: Option<&str>) -> &'static str {
    match stored.and_then(|value| {
        ALLOWED_RETURN_DESTINATIONS
            .iter()
            .find(|destination| **destination == value)
    }) {
        Some(destination) => destination,
        None => "/",
    }
}

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}

fn request_origin(req: &Request) -> Result<String> {
    let url = req.url()?;
    let host = url.host_str().unwrap_or("localhost");
    let host_with_port = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    };
    Ok(format!("{}://{}", url.scheme(), host_with_port))
}

fn urlencode(value: &str) -> String {
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

fn extract_cookie(req: &Request, name: &str) -> Option<String> {
    let header = req.headers().get("Cookie").ok()??;
    for pair in header.split(';') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        if key == name {
            return Some(value.to_owned());
        }
    }
    None
}

/// The non-secret internal invite reference RFC-080 §5 requires — reads
/// the same `__join_ticket` cookie the ordinary `/join` flow already
/// establishes (`ticket|invite_id:community_id`), never a raw invite code.
/// `None` if the cookie is absent or malformed; the caller falls back to
/// `/join` rather than starting a transaction with nothing to claim.
fn resolve_invite_reference(req: &Request) -> Option<String> {
    let raw = extract_cookie(req, "__join_ticket")?;
    let (_ticket, ticket_value) = raw.split_once('|')?;
    let invite_id = ticket_value.split(':').next()?;
    if invite_id.is_empty() {
        return None;
    }
    Some(invite_id.to_owned())
}

fn pkce_challenge(verifier: &str) -> String {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

#[cfg(feature = "dev_fake_issuer")]
fn resolve_authorize_endpoint(namespace_id: &str, origin: &str) -> Option<String> {
    (namespace_id == NAMESPACE_ID).then(|| format!("{origin}/dev/identity/fake-issuer/authorize"))
}

#[cfg(not(feature = "dev_fake_issuer"))]
fn resolve_authorize_endpoint(_namespace_id: &str, _origin: &str) -> Option<String> {
    None
}

#[cfg(feature = "dev_fake_issuer")]
fn resolve_token_endpoint(namespace_id: &str, origin: &str) -> Option<String> {
    (namespace_id == NAMESPACE_ID).then(|| format!("{origin}/dev/identity/fake-issuer/token"))
}

#[cfg(not(feature = "dev_fake_issuer"))]
fn resolve_token_endpoint(_namespace_id: &str, _origin: &str) -> Option<String> {
    None
}

fn sign_in_failed_page() -> Result<Response> {
    let body = format!(
        "<main class=\"cz-anon-main\">\
         <h1 class=\"cz-anon-title\">{title}</h1>\
         <p class=\"cz-anon-error-text\">{body}</p>\
         <div class=\"cz-error-recovery-links\">\
           <a href=\"/identity/start?action=sign_in\" class=\"cz-error-recovery-link\">{retry}</a>\
           <a href=\"/join\" class=\"cz-error-recovery-link\">{cancel}</a>\
         </div></main>",
        title = i18n::JA_IDENTITY_SIGN_IN_FAILED_TITLE,
        body = i18n::JA_IDENTITY_SIGN_IN_FAILED_BODY,
        retry = i18n::JA_IDENTITY_SIGN_IN_RETRY,
        cancel = i18n::JA_IDENTITY_SIGN_IN_CANCEL,
    );
    Ok(render::page(i18n::JA_IDENTITY_SIGN_IN_FAILED_TITLE, &body)?.with_status(200))
}

// ── GET /identity/start ───────────────────────────────────────────────────

pub async fn get_start(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    if crate::session::require_auth(&req, env).await.is_ok() {
        return redirect("/");
    }

    let url = req.url()?;
    let action = url
        .query_pairs()
        .find(|(k, _)| k == "action")
        .map(|(_, v)| v.into_owned());
    let action = match action.as_deref() {
        Some("sign_in") => "sign_in",
        Some("join") => "join",
        _ => return render::not_found(),
    };

    let invite_reference = if action == "join" {
        match resolve_invite_reference(&req) {
            Some(reference) => Some(reference),
            None => return redirect("/join"),
        }
    } else {
        None
    };

    let origin = request_origin(&req)?;
    let Some(authorize_endpoint) = resolve_authorize_endpoint(NAMESPACE_ID, &origin) else {
        return sign_in_failed_page();
    };

    let pepper = crate::crypto::pepper(env)?;
    let db = env.d1("DB")?;

    let state = crate::crypto::random_token();
    let nonce = crate::crypto::random_token();
    let pkce_verifier = crate::crypto::random_token();
    let lookup_key_hmac = crate::crypto::hmac_hex(pepper.as_str(), &state);
    let nonce_hmac = crate::crypto::hmac_hex(pepper.as_str(), &nonce);
    let code_challenge = pkce_challenge(&pkce_verifier);
    let callback_uri = format!("{origin}{CALLBACK_PATH}");
    let transaction_id = crate::crypto::random_token();
    let now = db::now_utc();
    let expires_at = db::add_seconds_to_now(TRANSACTION_TTL_SECONDS);

    db::auth_transaction::insert_required(
        &db,
        &transaction_id,
        &lookup_key_hmac,
        action,
        NAMESPACE_ID,
        &nonce_hmac,
        &pkce_verifier,
        None,
        invite_reference.as_deref(),
        &callback_uri,
        Some("/"),
        &now,
        &expires_at,
    )
    .await?;

    let destination = format!(
        "{authorize_endpoint}?response_type=code&client_id={client_id}&redirect_uri={redirect_uri}\
         &state={state}&nonce={nonce}&code_challenge={code_challenge}&code_challenge_method=S256\
         &scope=openid",
        client_id = urlencode(CLIENT_ID),
        redirect_uri = urlencode(&callback_uri),
        state = urlencode(&state),
        nonce = urlencode(&nonce),
        code_challenge = urlencode(&code_challenge),
    );
    redirect(&destination)
}

// ── GET /identity/callback ─────────────────────────────────────────────────

pub async fn get_callback(req: Request, env: &Env, rid: &str) -> Result<Response> {
    let url = req.url()?;
    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned());
    let state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.into_owned());
    let (Some(code), Some(state)) = (code, state) else {
        return sign_in_failed_page();
    };

    let pepper = crate::crypto::pepper(env)?;
    let db = env.d1("DB")?;
    let now = db::now_utc();

    // Step 2: atomically consume/reserve the transaction against replay,
    // strictly before the code exchange (RFC-080 §5.1's required order —
    // a replay must be rejected before it costs a provider round trip).
    let lookup_key_hmac = crate::crypto::hmac_hex(pepper.as_str(), &state);
    let Some(transaction) =
        db::auth_transaction::find_active_by_lookup_key_hmac(&db, &lookup_key_hmac, &now).await?
    else {
        return sign_in_failed_page();
    };
    if !db::auth_transaction::consume_required(&db, &transaction.id, &now).await? {
        return sign_in_failed_page();
    }

    // Step 1: select only the namespace the transaction itself expects.
    let Some(namespace) =
        crate::identity::resolve_namespace_verification(&transaction.identity_namespace_id)
    else {
        return sign_in_failed_page();
    };

    // Step 3: exchange the code server-to-server, exact redirect URI + PKCE.
    let origin = request_origin(&req)?;
    let Some(id_token) = exchange_code(&transaction, &code, &origin).await? else {
        return sign_in_failed_page();
    };

    // Steps 4-6: verify signature under the namespace-pinned algorithm and
    // key, validate every claim, reject on any mismatch — each with its
    // own reason internally (Handoff 053's matrix), one generic outcome
    // here regardless of which one fired.
    let now_unix = (worker::Date::now().as_millis() / 1000) as i64;
    let Ok(verified) = crate::identity::verify_id_token(
        &id_token,
        &transaction.identity_namespace_id,
        &namespace,
        &transaction.nonce_hmac,
        pepper.as_str(),
        now_unix,
    ) else {
        return sign_in_failed_page();
    };

    // Step 7: only VerifiedExternalIdentity crosses into identity logic —
    // no token, no claims map, no raw subject reaches anything below this
    // line except as the one-way digest.
    let subject_lookup = crate::crypto::subject_lookup(
        pepper.as_str(),
        &verified.identity_namespace_id,
        &verified.subject,
    );

    // Step 8: atomically create-or-link, claim any invite, issue the
    // session, and audit — one outcome per `action`, RFC-080 §7 (no
    // orphan users, no auto-link).
    let outcome = match transaction.action.as_str() {
        "sign_in" => {
            sign_in_outcome(&db, rid, pepper.as_str(), &namespace, &subject_lookup).await?
        }
        "join" => {
            join_outcome(
                &db,
                rid,
                pepper.as_str(),
                &namespace,
                &subject_lookup,
                transaction.invite_reference.as_deref(),
            )
            .await?
        }
        _ => None,
    };

    let Some(session_secret) = outcome else {
        return sign_in_failed_page();
    };

    // Step 9: redirect to a fixed clean local route — provider response
    // parameters (`code`, `state`) never appear in this response's own
    // URL, so they drop out of subsequent history/referrer propagation.
    let destination = resolve_safe_return(transaction.return_to.as_deref());
    let cookie_domain = env
        .var("SESSION_COOKIE_DOMAIN")
        .ok()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());
    let session_cookie =
        crate::session::build_session_cookie(&session_secret, cookie_domain.as_deref());
    let mut resp = redirect(destination)?;
    resp.headers_mut().set("Set-Cookie", &session_cookie)?;
    resp.headers_mut().set("Referrer-Policy", "no-referrer")?;
    Ok(resp)
}

async fn sign_in_outcome(
    db: &worker::D1Database,
    rid: &str,
    pepper: &str,
    namespace: &crate::identity::NamespaceVerification,
    subject_lookup: &str,
) -> Result<Option<String>> {
    let _ = namespace;
    let identity =
        db::identity::find_by_subject_lookup(db, "idns_local_fake", subject_lookup).await?;
    let Some(identity) = identity else {
        return Ok(None);
    };
    if !crate::identity::identity_lookup_is_authenticatable(&identity.status) {
        return Ok(None);
    }
    let session_secret = crate::crypto::random_token();
    let session_hmac = crate::crypto::hmac_hex(pepper, &session_secret);
    let session_id = crate::crypto::random_token();
    db::auth_transaction::issue_sign_in_required(
        db,
        rid,
        &identity.id,
        &identity.user_id,
        &session_id,
        &session_hmac,
    )
    .await?;
    Ok(Some(session_secret))
}

async fn join_outcome(
    db: &worker::D1Database,
    rid: &str,
    pepper: &str,
    namespace: &crate::identity::NamespaceVerification,
    subject_lookup: &str,
    invite_reference: Option<&str>,
) -> Result<Option<String>> {
    let _ = namespace;
    // A known identity has no business on the join path — RFC-080 §7 gives
    // it authentication without a new invite, but not through this route,
    // which only knows how to create.
    if db::identity::find_by_subject_lookup(db, "idns_local_fake", subject_lookup)
        .await?
        .is_some()
    {
        return Ok(None);
    }
    let Some(invite_id) = invite_reference else {
        return Ok(None);
    };
    let Some(invite) = db::invite::find_by_id(db, invite_id).await? else {
        return Ok(None);
    };

    let user_id = crate::crypto::random_token();
    let membership_id = crate::crypto::random_token();
    let identity_id = crate::crypto::random_token();
    let session_secret = crate::crypto::random_token();
    let session_hmac = crate::crypto::hmac_hex(pepper, &session_secret);
    let session_id = crate::crypto::random_token();
    // A display name from an external identity is not collected by this
    // slice (no account surface) — use a fixed placeholder the member can
    // change afterward, the same field `/join/profile` lets them set.
    let display_name = i18n::JA_JOIN_PROFILE_LABEL;

    db::auth_transaction::issue_join_required(
        db,
        rid,
        invite_id,
        &invite.community_id,
        &invite.grants_role,
        &user_id,
        &membership_id,
        display_name,
        &identity_id,
        "idns_local_fake",
        subject_lookup,
        &session_id,
        &session_hmac,
    )
    .await?;
    Ok(Some(session_secret))
}

async fn exchange_code(
    transaction: &db::auth_transaction::AuthTransactionRow,
    code: &str,
    origin: &str,
) -> Result<Option<String>> {
    let Some(token_endpoint) = resolve_token_endpoint(&transaction.identity_namespace_id, origin)
    else {
        return Ok(None);
    };

    let body = format!(
        "grant_type=authorization_code&code={code}&redirect_uri={redirect_uri}&code_verifier={verifier}",
        code = urlencode(code),
        redirect_uri = urlencode(&transaction.callback_uri),
        verifier = urlencode(&transaction.pkce_verifier),
    );
    let headers = Headers::new();
    headers.set("Content-Type", "application/x-www-form-urlencoded")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(worker::wasm_bindgen::JsValue::from_str(&body)));
    let request = Request::new_with_init(&token_endpoint, &init)?;
    let mut response = worker::Fetch::Request(request).send().await?;
    if response.status_code() != 200 {
        return Ok(None);
    }
    let json: serde_json::Value = response.json().await?;
    Ok(json
        .get("id_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned()))
}

#[cfg(test)]
mod tests;
