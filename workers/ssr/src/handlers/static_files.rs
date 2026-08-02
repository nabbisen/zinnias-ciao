//! Static asset and PWA file handlers.
//!
//! In production these would be served by Cloudflare's asset pipeline;
//! during local `wrangler dev` we serve them directly from the Worker.

use worker::{Env, Request, Response, Result};

pub async fn get_manifest(_req: Request, _env: &Env) -> Result<Response> {
    let body = include_str!("../../static/manifest.webmanifest");
    let mut r = Response::from_body(worker::ResponseBody::Body(body.as_bytes().to_vec()))?;
    r.headers_mut()
        .set("Content-Type", "application/manifest+json")?;
    r.headers_mut()
        .set("Cache-Control", "public, max-age=86400")?;
    Ok(r)
}

pub async fn get_sw(_req: Request, _env: &Env) -> Result<Response> {
    let body = include_str!("../../static/sw.js");
    let mut r = Response::from_body(worker::ResponseBody::Body(body.as_bytes().to_vec()))?;
    r.headers_mut()
        .set("Content-Type", "application/javascript")?;
    // SW must not be cached aggressively — browsers re-check on every navigation
    r.headers_mut().set("Cache-Control", "no-cache")?;
    r.headers_mut().set("Service-Worker-Allowed", "/")?;
    Ok(r)
}

pub async fn get_css(_req: Request, _env: &Env) -> Result<Response> {
    let body = include_str!("../../static/app.css");
    let mut r = Response::from_body(worker::ResponseBody::Body(body.as_bytes().to_vec()))?;
    r.headers_mut().set("Content-Type", "text/css")?;
    r.headers_mut()
        .set("Cache-Control", "public, max-age=3600")?;
    Ok(r)
}

pub async fn get_app_js(_req: Request, _env: &Env) -> Result<Response> {
    let body = include_str!("../../static/app.js");
    let mut r = Response::from_body(worker::ResponseBody::Body(body.as_bytes().to_vec()))?;
    r.headers_mut()
        .set("Content-Type", "application/javascript")?;
    r.headers_mut()
        .set("Cache-Control", "public, max-age=3600")?;
    Ok(r)
}

pub async fn get_offline(_req: Request, _env: &Env) -> Result<Response> {
    let html = r#"<!DOCTYPE html>
<html lang="ja">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>オフライン — ciao.zinnias</title>
  <link rel="stylesheet" href="/static/app.css">
</head>
<body>
  <div id="offline-banner">オフライン — 最後に読み込んだ情報を表示しています</div>
  <main class="cz-anon-main">
    <h1 class="cz-anon-title">オフラインです</h1>
    <p class="cz-anon-subtitle">電波がある場所で再度開いてください。</p>
  </main>
  <script src="/static/app.js?v=0.61.0" defer></script>
</body>
</html>"#;
    let mut r = Response::from_html(html)?;
    r.headers_mut().set("Cache-Control", "no-store")?;
    Ok(r)
}
