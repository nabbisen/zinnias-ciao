//! Static asset and PWA file handlers.
//!
//! In production these would be served by Cloudflare's asset pipeline;
//! during local `wrangler dev` we serve them directly from the Worker.

use worker::{Env, Request, Response, Result};

pub async fn get_manifest(_req: Request, _env: &Env) -> Result<Response> {
    let body = include_str!("../../static/manifest.webmanifest");
    let mut r = Response::from_body(worker::ResponseBody::Body(body.as_bytes().to_vec()))?;
    r.headers_mut().set("Content-Type", "application/manifest+json")?;
    r.headers_mut().set("Cache-Control", "public, max-age=86400")?;
    Ok(r)
}

pub async fn get_sw(_req: Request, _env: &Env) -> Result<Response> {
    let body = include_str!("../../static/sw.js");
    let mut r = Response::from_body(worker::ResponseBody::Body(body.as_bytes().to_vec()))?;
    r.headers_mut().set("Content-Type", "application/javascript")?;
    // SW must not be cached aggressively — browsers re-check on every navigation
    r.headers_mut().set("Cache-Control", "no-cache")?;
    r.headers_mut().set("Service-Worker-Allowed", "/")?;
    Ok(r)
}

pub async fn get_css(_req: Request, _env: &Env) -> Result<Response> {
    let body = include_str!("../../static/app.css");
    let mut r = Response::from_body(worker::ResponseBody::Body(body.as_bytes().to_vec()))?;
    r.headers_mut().set("Content-Type", "text/css")?;
    r.headers_mut().set("Cache-Control", "public, max-age=3600")?;
    Ok(r)
}

pub async fn get_app_js(_req: Request, _env: &Env) -> Result<Response> {
    let body = include_str!("../../static/app.js");
    let mut r = Response::from_body(worker::ResponseBody::Body(body.as_bytes().to_vec()))?;
    r.headers_mut().set("Content-Type", "application/javascript")?;
    r.headers_mut().set("Cache-Control", "public, max-age=3600")?;
    Ok(r)
}

pub async fn get_offline(_req: Request, _env: &Env) -> Result<Response> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Offline — ciao.zinnias</title>
  <link rel="stylesheet" href="/static/app.css">
</head>
<body>
  <div id="offline-banner">Offline — showing last loaded</div>
  <main style="padding:2rem;max-width:480px;margin:auto;font-family:system-ui,sans-serif">
    <h1 style="font-size:1.25rem;font-weight:600">You are offline</h1>
    <p style="color:#6e6e73">Open again when connected.</p>
  </main>
  <script src="/static/app.js" defer></script>
</body>
</html>"#;
    let mut r = Response::from_html(html)?;
    r.headers_mut().set("Cache-Control", "no-store")?;
    Ok(r)
}
