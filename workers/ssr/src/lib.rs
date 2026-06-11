use worker::*;

mod audit;
mod authz;
mod crypto;
mod db;
mod errors;
mod form_token;
mod render;
mod session;

mod handlers;

/// Cloudflare Worker entrypoint — routes all requests.
#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let request_id = generate_request_id();

    let url = req.url()?;
    let path = url.path();
    let method = req.method();

    let result: Result<Response> = match (method, path) {
        // ── infrastructure ─────────────────────────────────────────────
        (Method::Get, "/healthz") => handlers::health::get_health(&env).await,
        (Method::Get, "/version") => handlers::health::get_version(&env).await,

        // ── join / onboarding ──────────────────────────────────────────
        (Method::Get, "/join") => handlers::join::get_join(req, &env, &request_id).await,
        (Method::Post, "/join") => handlers::join::post_join(req, &env, &request_id).await,
        (Method::Get, "/join/profile") => handlers::join::get_profile(req, &env, &request_id).await,
        (Method::Post, "/join/profile") => {
            handlers::join::post_profile(req, &env, &request_id).await
        }

        // ── member area ────────────────────────────────────────────────
        (Method::Get, "/") | (Method::Get, "/c") => {
            handlers::home::redirect_to_home(req, &env, &request_id).await
        }
        (Method::Get, p) if p.starts_with("/c/") => {
            handlers::community::dispatch_get(req, &env, &request_id, p).await
        }
        (Method::Post, p) if p.starts_with("/c/") => {
            handlers::community::dispatch_post(req, &env, &request_id, p).await
        }

        // ── logout ─────────────────────────────────────────────────────
        (Method::Post, "/logout") => handlers::auth::post_logout(req, &env, &request_id).await,

        // ── 404 ────────────────────────────────────────────────────────
        _ => render::not_found(),
    };

    match result {
        Ok(mut resp) => {
            attach_security_headers(&mut resp, &request_id)?;
            Ok(resp)
        }
        Err(e) => {
            console_error!("[{}] unhandled error: {:?}", request_id, e);
            let mut resp = render::internal_error()?;
            attach_security_headers(&mut resp, &request_id)?;
            Ok(resp)
        }
    }
}

fn generate_request_id() -> String {
    use std::fmt::Write;
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).unwrap_or_default();
    let mut s = String::with_capacity(16);
    for b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn attach_security_headers(resp: &mut Response, request_id: &str) -> Result<()> {
    let headers = resp.headers_mut();
    headers.set(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
         img-src 'self' data:; frame-ancestors 'none'",
    )?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("X-Frame-Options", "DENY")?;
    headers.set("Referrer-Policy", "strict-origin-when-cross-origin")?;
    headers.set("X-Request-Id", request_id)?;
    Ok(())
}
