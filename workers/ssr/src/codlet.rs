//! codlet integration: manager construction and token helpers.
//!
//! **Wasm32 (production Workers):** full codlet managers backed by D1 and KV.
//! **Non-wasm (native tests):** `issue_token` and `consume_token` fall through
//! directly to the legacy `form_token::issue/consume` functions so tests remain
//! unaffected.
//!
//! The three wasm32-only managers (`CodeAuth`, `SessionManager`,
//! `FormTokenManager`) are built from `build()`, `build_session_mgr()`, and
//! `build_token_mgr()`.  `run_migrations` is called once in `lib.rs::main()`
//! before routing.

#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
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
#[cfg(target_arch = "wasm32")]
use codlet_worker::{
    D1CodeStore, D1FormTokenStore, D1SessionStore, D1TableConfig, KvRateLimitStore,
    WorkerKeyProvider, run_d1_migrations,
};
use worker::{Env, Result};

// ── Wasm32: migration helper ──────────────────────────────────────────────

/// Run codlet's D1 schema migrations idempotently.
///
/// Called once per request in `lib.rs::main()`, before routing.
#[cfg(target_arch = "wasm32")]
pub async fn run_migrations(env: &Env) -> Result<()> {
    let db = env.d1("DB")?;
    run_d1_migrations(&db).await
}

// ── Wasm32: manager types ─────────────────────────────────────────────────

/// All three codlet managers for one request lifetime.
#[cfg(target_arch = "wasm32")]
pub struct CodletManagers {
    pub code_auth:   CodeAuth<D1CodeStore, KvRateLimitStore, WorkerKeyProvider, SystemClock, NoopAuditSink>,
    pub session_mgr: SessionManager<D1SessionStore, WorkerKeyProvider, SystemClock, NoopAuditSink>,
    pub token_mgr:   FormTokenManager<D1FormTokenStore, WorkerKeyProvider, SystemClock, NoopAuditSink>,
    pub rng:         SystemRandom,
}

/// Build all three codlet managers from the Worker `Env`.
///
/// Fails closed if `CODLET_HMAC_KEY_V1` is absent or empty (INV-2).
#[cfg(target_arch = "wasm32")]
pub async fn build(env: &Env) -> Result<CodletManagers> {
    let db     = Rc::new(env.d1("DB")?);
    let tables = D1TableConfig::default();

    let kv_store = worker_kv::KvStore::create("CODLET_RL")
        .map_err(|e| worker::Error::RustError(format!("CODLET_RL KV: {e:?}")))?;
    let rl_store = KvRateLimitStore::new(kv_store);

    const INVITE_TTL_SECS: u64 = 86_400;
    #[allow(deprecated)]
    let code_policy = CodePolicy::six_symbol(Duration::from_secs(INVITE_TTL_SECS))
        .map_err(|e| worker::Error::RustError(format!("codlet policy: {e}")))?;

    let cookie_policy = CookiePolicy::production_strict(
        "ciao_sid",
        Duration::from_secs(30 * 24 * 3600),
    );

    let rl_policy = RateLimitPolicy {
        max_failures: 5,
        window: Duration::from_secs(15 * 60),
        unavailable: RateLimitUnavailable::FailOpen,
    };

    let form_token_ttl = Duration::from_secs(3600);

    // Three independent WorkerKeyProvider instances (not Clone).
    let kp_code    = WorkerKeyProvider::from_env(env, "v1", "CODLET_HMAC_KEY_V1", &[])?;
    let kp_session = WorkerKeyProvider::from_env(env, "v1", "CODLET_HMAC_KEY_V1", &[])?;
    let kp_token   = WorkerKeyProvider::from_env(env, "v1", "CODLET_HMAC_KEY_V1", &[])?;

    let code_store    = D1CodeStore::new(Rc::clone(&db), tables.clone());
    let session_store = D1SessionStore::new(Rc::clone(&db), tables.clone());
    let token_store   = D1FormTokenStore::new(db, tables);

    let code_auth = CodeAuth::new(
        code_store, rl_store, SecretHasher::new(kp_code),
        SystemClock::new(), NoopAuditSink, code_policy, rl_policy,
    );

    let session_mgr = SessionManager::new(
        session_store, SecretHasher::new(kp_session),
        SystemClock::new(), NoopAuditSink, cookie_policy,
    );

    let token_mgr = FormTokenManager::new(
        token_store, SecretHasher::new(kp_token),
        SystemClock::new(), NoopAuditSink, form_token_ttl,
    );

    Ok(CodletManagers {
        code_auth, session_mgr, token_mgr,
        rng: SystemRandom::new(),
    })
}

/// Build only the `SessionManager` — for use in `session::require_auth`
/// and `handlers/auth.rs::post_logout` to avoid paying the cost of building
/// `CodeAuth` and `FormTokenManager` on every authenticated request.
#[cfg(target_arch = "wasm32")]
pub fn build_session_mgr(
    env: &Env,
) -> Result<SessionManager<D1SessionStore, WorkerKeyProvider, SystemClock, NoopAuditSink>> {
    let db  = Rc::new(env.d1("DB")?);
    let kp  = WorkerKeyProvider::from_env(env, "v1", "CODLET_HMAC_KEY_V1", &[])?;
    let cookie_policy = CookiePolicy::production_strict(
        "ciao_sid", Duration::from_secs(30 * 24 * 3600),
    );
    Ok(SessionManager::new(
        D1SessionStore::new(db, D1TableConfig::default()),
        SecretHasher::new(kp), SystemClock::new(), NoopAuditSink, cookie_policy,
    ))
}

/// Build the `Set-Cookie` header value that clears the codlet session cookie.
/// Single source of truth for cookie name and attributes at login and logout.
#[cfg(target_arch = "wasm32")]
pub fn session_clear_cookie() -> String {
    CookiePolicy::production_strict("ciao_sid", Duration::from_secs(30 * 24 * 3600))
        .build_clear_cookie()
}

/// Build only the `FormTokenManager`.
#[cfg(target_arch = "wasm32")]
fn build_token_mgr(
    env: &Env,
) -> Result<FormTokenManager<D1FormTokenStore, WorkerKeyProvider, SystemClock, NoopAuditSink>> {
    let db  = Rc::new(env.d1("DB")?);
    let kp  = WorkerKeyProvider::from_env(env, "v1", "CODLET_HMAC_KEY_V1", &[])?;
    Ok(FormTokenManager::new(
        D1FormTokenStore::new(db, D1TableConfig::default()),
        SecretHasher::new(kp), SystemClock::new(), NoopAuditSink,
        Duration::from_secs(3600),
    ))
}

// ── Token helpers — available on ALL targets ──────────────────────────────
//
// On wasm32: delegates to codlet's FormTokenManager.
// On non-wasm (native tests): delegates directly to the legacy form_token module.
// This lets handlers call crate::codlet::issue_token / consume_token without
// conditional compilation at every call site.

/// Issue a single-use CSRF form token.
///
/// Returns the plaintext token to embed in the form, or `""` on error.
/// Subject is always `Authenticated(user_id)`.
pub async fn issue_token(
    env: &Env,
    user_id: &str,
    purpose: &str,
    bound_resource: Option<&str>,
) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        use codlet_core::store::token::TokenSubject;
        use codlet_core::secret::SubjectId;

        if let Ok(mgr) = build_token_mgr(env) {
            let subject = TokenSubject::Authenticated(SubjectId::new(user_id.to_owned().into()));
            let mut rng = SystemRandom::new();
            return mgr.issue(&mut rng, subject, purpose, bound_resource.map(str::to_owned))
                .await
                .map(|s| s.expose().to_owned())
                .unwrap_or_default();
        }
        // Fall through to legacy if codlet not yet configured.
    }

    // Non-wasm path and wasm32 fallback: legacy form_tokens table.
    let pepper = crate::crypto::pepper(env);
    if let Ok(db) = env.d1("DB") {
        crate::form_token::issue(&db, &pepper, user_id, purpose, bound_resource)
            .await
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Validate and consume a single-use CSRF form token.
///
/// Returns:
/// - `Ok(None)`          — first submission; proceed
/// - `Ok(Some(String))`  — replay; redirect
/// - `Err`               — invalid/expired; reject
pub async fn consume_token(
    env: &Env,
    user_id: &str,
    purpose: &str,
    raw_token: &str,
    bound_resource: Option<&str>,
) -> Result<Option<String>> {
    #[cfg(target_arch = "wasm32")]
    {
        use codlet_core::store::token::TokenSubject;
        use codlet_core::secret::SubjectId;

        if let Ok(mgr) = build_token_mgr(env) {
            let subject = TokenSubject::Authenticated(SubjectId::new(user_id.to_owned().into()));
            return mgr.consume(raw_token, &subject, purpose, bound_resource)
                .await
                .map_err(|e| worker::Error::RustError(format!("form token: {e}")));
        }
        // Fall through to legacy if codlet not yet configured.
    }

    // Non-wasm path and wasm32 fallback: legacy form_tokens table.
    let db     = env.d1("DB")?;
    let pepper = crate::crypto::pepper(env);
    crate::form_token::consume(&db, &pepper, user_id, purpose, raw_token, bound_resource).await
}

// ── Unified invite admin helpers ──────────────────────────────────────────

/// Metadata for one active invite code — unified across codlet and legacy tables.
pub struct InviteCodeMeta {
    pub id:          String,
    /// ISO-8601 prefix for display (first 16 chars).
    pub expires_at:  String,
    /// "admin" or "member".
    pub grants_role: String,
}

/// List active invite codes for a community from both `codlet_codes` (wasm32)
/// and the legacy `invite_codes` table. Codlet codes are listed first.
pub async fn list_active_invites(env: &Env, community_id: &str) -> Vec<InviteCodeMeta> {
    let mut result = Vec::new();

    // ── Codlet codes (wasm32 production) ────────────────────────────────────
    #[cfg(target_arch = "wasm32")]
    {
        use codlet_core::{
            admin::{CodeAdminStore, CodeListFilter},
            clock::{Clock, SystemClock},
            secret::ScopeKey,
        };
        use codlet_worker::D1TableConfig;

        if let Ok(db) = env.d1("DB") {
            let store = D1CodeStore::new(Rc::new(db), D1TableConfig::default());
            let now   = SystemClock::new().unix_now();
            let filter = CodeListFilter::active_in_scope(
                ScopeKey::new(community_id.to_string()),
            );
            if let Ok(metas) = store.list_codes(&filter, now).await {
                for m in metas {
                    result.push(InviteCodeMeta {
                        id:          m.id.as_str().to_owned(),
                        expires_at:  unix_secs_to_display(m.expires_at),
                        grants_role: m.grant
                            .as_deref()
                            .and_then(|g| g.strip_prefix("role:"))
                            .unwrap_or("member")
                            .to_owned(),
                    });
                }
            }
        }
    }

    // ── Legacy codes (all paths, grace period) ──────────────────────────────
    if let Ok(db) = env.d1("DB") {
        if let Ok(rows) = crate::db::invite::list_active_for_community(&db, community_id).await {
            for inv in rows {
                result.push(InviteCodeMeta {
                    id:          inv.id,
                    expires_at:  inv.expires_at,
                    grants_role: inv.grants_role,
                });
            }
        }
    }

    result
}

/// Revoke an invite code by ID, trying codlet first then the legacy table.
/// `community_id` is passed as scope to prevent cross-community revocation.
pub async fn revoke_invite(
    env:          &Env,
    invite_id:    &str,
    community_id: &str,
) -> worker::Result<()> {
    // ── Codlet path (wasm32) ────────────────────────────────────────────────
    #[cfg(target_arch = "wasm32")]
    {
        use codlet_core::secret::CodeId;
        if let Ok(mgrs) = build(env).await {
            let code_id = CodeId::new(invite_id.to_owned().into());
            if mgrs.code_auth.revoke_code(&code_id, Some(community_id)).await.is_ok() {
                return Ok(());
            }
            // Not found in codlet_codes — fall through to legacy.
        }
    }

    // ── Legacy path (all targets) ───────────────────────────────────────────
    let db = env.d1("DB")?;
    crate::db::invite::revoke(&db, invite_id, community_id).await
}

/// Format a Unix timestamp (seconds since epoch) as an ISO-8601 display prefix.
/// Returns "YYYY-MM-DDTHH:MM" — the first 16 characters of ISO-8601.
#[cfg(target_arch = "wasm32")]
fn unix_secs_to_display(ts: u64) -> String {
    let s = ts % 60;
    let m = (ts / 60) % 60;
    let h = (ts / 3600) % 24;
    let (year, month, day) = days_since_epoch_to_ymd(ts / 86400);
    let _ = s; // seconds not needed for display
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}")
}

/// Convert days since Unix epoch to (year, month, day) via the
/// Fliegel–Van Flandern proleptic Gregorian algorithm.
#[cfg(target_arch = "wasm32")]
fn days_since_epoch_to_ymd(days: u64) -> (u64, u64, u64) {
    let z   = days + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;
    let y   = yoe + era * 400;
    let doy = doe - (365*yoe + yoe/4 - yoe/100);
    let mp  = (5*doy + 2) / 153;
    let d   = doy - (153*mp + 2)/5 + 1;
    let m   = if mp < 10 { mp + 3 } else { mp - 9 };
    let y   = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
