use worker::*;

mod abuse_control;
mod abuse_limiter;
mod audit;
mod authz;
mod codlet;
mod crypto;
mod db;
mod form_token;
mod identity;
mod render;
mod session;

mod handlers;

// Exported at crate root for the generated Worker class name (RFC-078).
// The Durable Object is reachable only through the same-Worker binding; it
// is never added to the public HTTP router in `dispatch_request` below.
// wasm32-only: the `#[durable_object]`-generated glue does not compile
// natively (see `abuse_limiter`'s module doc).
#[cfg(target_arch = "wasm32")]
pub use abuse_limiter::AbuseLimiter;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let request_id = generate_request_id();

    let url = req.url()?;
    let path = url.path().to_owned();
    let method = req.method();

    let security_class = request_security_class(&method, &path);
    let continuation = configuration_guard(
        security_class,
        || crypto::pepper(&env).map(|_| ()),
        || dispatch_request(req, &env, &request_id, method, &path),
    );
    let result = match continuation {
        Ok(continuation) => continuation.await,
        Err(error) => {
            console_error!(
                "event=worker.security_configuration_unavailable request_id={} failure_category={} route_class=request",
                request_id,
                error.category()
            );
            let mut response = render::configuration_unavailable()?;
            attach_security_headers(&mut response, &request_id)?;
            return Ok(response);
        }
    };

    match result {
        Ok(mut resp) => {
            attach_security_headers(&mut resp, &request_id)?;
            Ok(resp)
        }
        Err(error) => {
            if is_not_found_error(&error) {
                let mut resp = render::not_found()?;
                attach_security_headers(&mut resp, &request_id)?;
                return Ok(resp);
            }
            console_error!(
                "event=worker.request_failed request_id={} failure_category=unhandled route_class=request",
                request_id
            );
            let mut resp = render::internal_error()?;
            attach_security_headers(&mut resp, &request_id)?;
            Ok(resp)
        }
    }
}

fn configuration_guard<T>(
    security_class: RequestSecurityClass,
    resolve: impl FnOnce() -> std::result::Result<(), crypto::PepperConfigError>,
    continuation: impl FnOnce() -> T,
) -> std::result::Result<T, crypto::PepperConfigError> {
    if security_class == RequestSecurityClass::Protected {
        resolve()?;
    }
    Ok(continuation())
}

async fn dispatch_request(
    req: Request,
    env: &Env,
    request_id: &str,
    method: Method,
    path: &str,
) -> Result<Response> {
    match (method, path) {
        // ── Infrastructure ────────────────────────────────────────────────
        (Method::Get, "/healthz") => handlers::health::get_health(env).await,
        (Method::Get, "/version") => handlers::health::get_version(env).await,

        // ── Operator-only recovery ────────────────────────────────────────
        (Method::Post, "/operator/recovery/community-access") => {
            handlers::operator::post_community_access_recovery(req, env, request_id).await
        }

        // ── Static assets + PWA ───────────────────────────────────────────
        (Method::Get, "/manifest.webmanifest") => {
            handlers::static_files::get_manifest(req, env).await
        }
        (Method::Get, "/sw.js") => handlers::static_files::get_sw(req, env).await,
        (Method::Get, "/static/app.css") => handlers::static_files::get_css(req, env).await,
        (Method::Get, "/static/app.js") => handlers::static_files::get_app_js(req, env).await,
        (Method::Get, "/offline") => handlers::static_files::get_offline(req, env).await,

        // ── Join / onboarding ─────────────────────────────────────────────
        (Method::Get, "/join") => handlers::join::get_join(req, env, request_id).await,
        (Method::Post, "/join") => handlers::join::post_join(req, env, request_id).await,
        (Method::Get, "/join/profile") => handlers::join::get_profile(req, env, request_id).await,
        (Method::Post, "/join/profile") => handlers::join::post_profile(req, env, request_id).await,
        (Method::Get, "/relink") => handlers::relink::get_relink(req, env, request_id).await,
        (Method::Post, "/relink") => handlers::relink::post_relink(req, env, request_id).await,

        // ── External identity (RFC-080 §5, Handoff 054) ───────────────────
        (Method::Get, "/identity/start") => {
            handlers::identity::get_start(req, env, request_id).await
        }
        (Method::Get, "/identity/callback") => {
            handlers::identity::get_callback(req, env, request_id).await
        }

        // ── The local fake issuer — dev/smoke builds only (Handoff 054 §3).
        // Absent entirely from a production build; see
        // `identity_dev_fake_issuer_absent_from_production_build` in
        // release_gates.rs for the artifact-level proof.
        #[cfg(feature = "dev_fake_issuer")]
        (Method::Get, "/dev/identity/fake-issuer/authorize") => {
            identity::dev_fake_issuer::get_authorize(req, env, request_id).await
        }
        #[cfg(feature = "dev_fake_issuer")]
        (Method::Post, "/dev/identity/fake-issuer/token") => {
            identity::dev_fake_issuer::post_token(req, env, request_id).await
        }

        // ── Member area ───────────────────────────────────────────────────
        (Method::Get, "/") | (Method::Get, "/c") => {
            handlers::home::redirect_to_home(req, env, request_id).await
        }
        (Method::Get, "/switch") => handlers::community::get_switch(req, env, request_id).await,
        (Method::Get, "/communities/new") => {
            handlers::community_create::get_new_community(req, env, request_id).await
        }
        (Method::Post, "/communities/new") => {
            handlers::community_create::post_new_community(req, env, request_id).await
        }
        (Method::Get, p) if p.starts_with("/c/") => {
            handlers::community::dispatch_get(req, env, request_id, p).await
        }
        (Method::Post, p) if p.starts_with("/c/") => {
            handlers::community::dispatch_post(req, env, request_id, p).await
        }

        // ── Logout ────────────────────────────────────────────────────────
        (Method::Post, "/logout") => handlers::auth::post_logout(req, env, request_id).await,

        _ => render::not_found(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RequestSecurityClass {
    Health,
    Unprotected,
    Protected,
}

fn request_security_class(method: &Method, path: &str) -> RequestSecurityClass {
    if method == &Method::Get && path == "/healthz" {
        return RequestSecurityClass::Health;
    }
    if method == &Method::Get
        && matches!(
            path,
            "/manifest.webmanifest"
                | "/sw.js"
                | "/static/app.css"
                | "/static/app.js"
                | "/offline"
                | "/version"
        )
    {
        return RequestSecurityClass::Unprotected;
    }
    RequestSecurityClass::Protected
}

fn is_not_found_error(e: &Error) -> bool {
    matches!(e, Error::RustError(message) if message == "Not found.")
}

fn generate_request_id() -> String {
    use std::fmt::Write;
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).unwrap_or_default();
    let mut s = String::with_capacity(16);
    for b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn attach_security_headers(resp: &mut Response, request_id: &str) -> Result<()> {
    let h = resp.headers_mut();
    // Content Security Policy.
    // RFC-075 removed every inline `style=` attribute from the SSR templates
    // across seven slices (486 at the peak, 0 now — release_gates.rs's
    // `inline_style_count_is_zero` asserts this stays true, not just that it
    // never increases). `style-src` therefore carries no 'unsafe-inline':
    // every directive here is strict.
    h.set(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self'; style-src 'self'; \
         img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; \
         form-action 'self'; object-src 'none'",
    )?;
    h.set("X-Content-Type-Options", "nosniff")?;
    h.set("X-Frame-Options", "DENY")?;
    // Handlers may set a stricter policy before this hook runs, such as
    // `no-referrer` for bearer URLs. Do not use this to loosen the default.
    if h.get("Referrer-Policy").ok().flatten().is_none() {
        h.set("Referrer-Policy", "same-origin")?;
    }
    h.set(
        "Permissions-Policy",
        "camera=(), microphone=(), geolocation=()",
    )?;
    // Cache-Control: prevent browsers and intermediaries from caching responses.
    // Static asset handlers (CSS, JS, manifest) set public/max-age headers before
    // this function runs; we only set no-store when the handler has not already
    // set a Cache-Control header, preserving intentional caching for static assets.
    if h.get("Cache-Control").ok().flatten().is_none() {
        h.set("Cache-Control", "no-store")?;
    }
    h.set("X-Request-Id", request_id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_classification_has_an_exact_get_allowlist() {
        assert_eq!(
            request_security_class(&Method::Get, "/healthz"),
            RequestSecurityClass::Health
        );
        for path in [
            "/manifest.webmanifest",
            "/sw.js",
            "/static/app.css",
            "/static/app.js",
            "/offline",
            "/version",
        ] {
            assert_eq!(
                request_security_class(&Method::Get, path),
                RequestSecurityClass::Unprotected,
                "unexpected classification for {path}"
            );
            assert_eq!(
                request_security_class(&Method::Post, path),
                RequestSecurityClass::Protected,
                "method mismatch must remain protected for {path}"
            );
        }
    }

    #[test]
    fn every_dynamic_and_unknown_route_is_protected() {
        for (method, path) in [
            (Method::Get, "/"),
            (Method::Get, "/join"),
            (Method::Post, "/join"),
            (Method::Get, "/relink"),
            (Method::Post, "/logout"),
            (Method::Get, "/c/example"),
            (Method::Post, "/operator/recovery/community-access"),
            (Method::Get, "/unknown"),
        ] {
            assert_eq!(
                request_security_class(&method, path),
                RequestSecurityClass::Protected,
                "dynamic route bypassed preflight: {method:?} {path}"
            );
        }
    }

    #[test]
    fn rejected_configuration_never_invokes_binding_continuation() {
        use std::cell::Cell;

        let d1_accesses = Cell::new(0);
        let kv_accesses = Cell::new(0);
        let result = configuration_guard(
            RequestSecurityClass::Protected,
            || Err(crypto::PepperConfigError::Missing),
            || {
                d1_accesses.set(d1_accesses.get() + 1);
                kv_accesses.set(kv_accesses.get() + 1);
            },
        );

        assert_eq!(result, Err(crypto::PepperConfigError::Missing));
        assert_eq!(d1_accesses.get(), 0, "D1 continuation was invoked");
        assert_eq!(kv_accesses.get(), 0, "KV continuation was invoked");
    }

    #[test]
    fn unprotected_routes_skip_resolution_and_invoke_continuation() {
        use std::cell::Cell;

        let resolver_calls = Cell::new(0);
        let continuation_calls = Cell::new(0);
        let result = configuration_guard(
            RequestSecurityClass::Unprotected,
            || {
                resolver_calls.set(resolver_calls.get() + 1);
                Ok(())
            },
            || continuation_calls.set(continuation_calls.get() + 1),
        );

        assert_eq!(result, Ok(()));
        assert_eq!(resolver_calls.get(), 0);
        assert_eq!(continuation_calls.get(), 1);
    }
}
