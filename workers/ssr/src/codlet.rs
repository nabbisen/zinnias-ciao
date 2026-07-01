//! codlet manager construction for each Workers request.
//!
//! Builds `CodeAuth`, `SessionManager`, and `FormTokenManager` from the
//! request environment.  All three share one `Rc<D1Database>` handle.
//! The KV rate-limit store uses `worker_kv::KvStore` directly (the type
//! codlet-worker's `KvRateLimitStore::new` expects — distinct from the
//! re-exported `worker::kv::KvStore`).

use std::rc::Rc;
use std::time::Duration;

use codlet_core::{
    audit::NoopAuditSink,
    auth::{CodeAuth, FormTokenManager, SessionManager},
    clock::SystemClock,
    cookie::CookiePolicy,
    hashing::SecretHasher,
    rng::SystemRandom,
    store::ratelimit::{RateLimitPolicy, RateLimitUnavailable},
    CodePolicy,
};
use codlet_worker::{
    D1CodeStore, D1FormTokenStore, D1SessionStore, D1TableConfig, KvRateLimitStore,
    WorkerKeyProvider, run_d1_migrations,
};
use worker::{Env, Result};

/// Run codlet's D1 schema migrations idempotently.
///
/// Must be called **once per request** at the top of the fetch handler, before
/// any `build()` calls. Separated from `build()` so migrations are not re-run
/// on every `build()` invocation within a single request.
pub async fn run_migrations(env: &Env) -> Result<()> {
    let db = env.d1("DB")?;
    run_d1_migrations(&db).await
}

/// All three codlet managers built for one request lifetime.
///
/// `WorkerKeyProvider` does not implement `Clone`, so each manager gets its
/// own `SecretHasher` wrapping an independent `WorkerKeyProvider` instance
/// built from the same underlying key bytes.
pub struct CodletManagers {
    pub code_auth:   CodeAuth<D1CodeStore, KvRateLimitStore, WorkerKeyProvider, SystemClock, NoopAuditSink>,
    pub session_mgr: SessionManager<D1SessionStore, WorkerKeyProvider, SystemClock, NoopAuditSink>,
    pub token_mgr:   FormTokenManager<D1FormTokenStore, WorkerKeyProvider, SystemClock, NoopAuditSink>,
    pub rng:         SystemRandom,
}

/// Build only the `SessionManager` component, without constructing the full
/// `CodletManagers`. Used by `session::require_auth` so every authenticated
/// request doesn't pay the cost of building `CodeAuth` and `FormTokenManager`.
pub fn build_session_mgr(
    env: &Env,
) -> Result<SessionManager<D1SessionStore, WorkerKeyProvider, SystemClock, NoopAuditSink>> {
    let db = Rc::new(env.d1("DB")?);
    let kp = WorkerKeyProvider::from_env(env, "v1", "CODLET_HMAC_KEY_V1", &[])?;
    let cookie_policy = CookiePolicy::production_strict(
        "ciao_sid",
        Duration::from_secs(30 * 24 * 3600),
    );
    Ok(SessionManager::new(
        D1SessionStore::new(db, D1TableConfig::default()),
        SecretHasher::new(kp),
        SystemClock::new(),
        NoopAuditSink,
        cookie_policy,
    ))
}

/// Return the `Set-Cookie` header value that clears the codlet session cookie.
///
/// Kept here alongside the cookie policy constants so there is one source of
/// truth for the cookie name and attributes used at both login and logout.
pub fn session_clear_cookie() -> String {
    CookiePolicy::production_strict("ciao_sid", Duration::from_secs(30 * 24 * 3600))
        .build_clear_cookie()
}
///
/// Fails closed if `CODLET_HMAC_KEY_V1` is absent or empty (INV-2).
/// `run_d1_migrations` is **not** called here; call it once at the top of the
/// fetch handler before routing so it runs exactly once per request, not once
/// per `build()` invocation.
pub async fn build(env: &Env) -> Result<CodletManagers> {
    // ── Shared D1 handle ────────────────────────────────────────────────────
    let db = Rc::new(env.d1("DB")?);

    // ── Tables ──────────────────────────────────────────────────────────────
    // Option A: use codlet's own table names (codlet_codes / codlet_sessions /
    // codlet_form_tokens). Existing service tables are untouched.
    let tables = D1TableConfig::default();

    // ── KV rate-limit store ─────────────────────────────────────────────────
    // worker_kv::KvStore::create() accesses the binding by name from the
    // global JS context, matching what `wrangler.toml` [[kv_namespaces]] sets.
    let kv_store = worker_kv::KvStore::create("CODLET_RL")
        .map_err(|e| worker::Error::RustError(format!("CODLET_RL KV: {e:?}")))?;
    let rl_store = KvRateLimitStore::new(kv_store);

    // ── Policies ────────────────────────────────────────────────────────────
    // Code policy: 6-symbol codes.
    //
    // `CodePolicy::six_symbol` is deprecated because 6-symbol codes carry only
    // ~29.7 bits of entropy, below the library's 8-symbol secure minimum
    // (~39.6 bits). The `#[allow(deprecated)]` below is an explicit,
    // reviewable acknowledgment that the following three conditions are met for
    // this deployment — as codlet's deprecation note requires:
    //
    //   1. SHORT EXPIRY: codes expire in 24 hours (`INVITE_TTL_SECS`).
    //      This tightly bounds the online-guessing window.
    //
    //   2. SINGLE-USE: codlet's atomic `claim_code` conditional UPDATE
    //      guarantees one-time use even under concurrent submissions (INV-5).
    //
    //   3. RATE LIMITING: `KvRateLimitStore` with 5 failures / 15-minute
    //      window is active. The handoff document (§8f) names this the minimum
    //      policy for 6-symbol codes. `FailOpen` is intentional: a KV outage
    //      should not lock out legitimate users of a small community.
    //
    // Migration path: once all outstanding 6-char codes have expired (i.e.,
    // no admin-generated invites predate this deploy by more than 24 hours),
    // switch to `CodePolicy::default_human(Duration::from_secs(INVITE_TTL_SECS))`
    // and remove this `#[allow(deprecated)]`. Admin-issued codes will then be
    // 8 symbols; users entering old codes will simply see an expired error.
    const INVITE_TTL_SECS: u64 = 86_400; // 24 hours — matches legacy invite_codes.expires_at
    #[allow(deprecated)]
    let code_policy = CodePolicy::six_symbol(Duration::from_secs(INVITE_TTL_SECS))
        .map_err(|e| worker::Error::RustError(format!("codlet policy: {e}")))?;

    let cookie_policy = CookiePolicy::production_strict(
        "ciao_sid",                          // keep existing session cookie name
        Duration::from_secs(30 * 24 * 3600), // 30-day session (SESSION_TTL_SECONDS)
    );

    // 5 failures / 15-minute window — minimum for 6-symbol codes (handoff §8f).
    let rl_policy = RateLimitPolicy {
        max_failures: 5,
        window: Duration::from_secs(15 * 60),
        unavailable: RateLimitUnavailable::FailOpen,
    };

    // Form-token TTL: 1 hour, matching the existing FORM_TOKEN_TTL_SECONDS.
    let form_token_ttl = Duration::from_secs(3600);

    // ── WorkerKeyProvider ────────────────────────────────────────────────────
    // WorkerKeyProvider does not impl Clone. Build three independent instances
    // from the same `Env` — each call reads the same Wrangler secret bytes.
    // Fails closed if the secret is absent or empty (INV-2).
    let kp_code    = WorkerKeyProvider::from_env(env, "v1", "CODLET_HMAC_KEY_V1", &[])?;
    let kp_session = WorkerKeyProvider::from_env(env, "v1", "CODLET_HMAC_KEY_V1", &[])?;
    let kp_token   = WorkerKeyProvider::from_env(env, "v1", "CODLET_HMAC_KEY_V1", &[])?;

    // ── Stores ──────────────────────────────────────────────────────────────
    let code_store    = D1CodeStore::new(Rc::clone(&db), tables.clone());
    let session_store = D1SessionStore::new(Rc::clone(&db), tables.clone());
    let token_store   = D1FormTokenStore::new(db, tables);

    // ── Managers ────────────────────────────────────────────────────────────
    let code_auth = CodeAuth::new(
        code_store,
        rl_store,
        SecretHasher::new(kp_code),
        SystemClock::new(),
        NoopAuditSink, // TODO: wire CiaoCodletAuditSink once stable
        code_policy,
        rl_policy,
    );

    let session_mgr = SessionManager::new(
        session_store,
        SecretHasher::new(kp_session),
        SystemClock::new(),
        NoopAuditSink,
        cookie_policy,
    );

    let token_mgr = FormTokenManager::new(
        token_store,
        SecretHasher::new(kp_token),
        SystemClock::new(),
        NoopAuditSink,
        form_token_ttl,
    );

    Ok(CodletManagers {
        code_auth,
        session_mgr,
        token_mgr,
        rng: SystemRandom::new(),
    })
}
