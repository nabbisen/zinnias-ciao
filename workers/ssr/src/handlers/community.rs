use worker::{Env, Request, Response, Result};
use crate::render;

pub async fn dispatch_get(
    _req: Request,
    _env: &Env,
    _request_id: &str,
    _path: &str,
) -> Result<Response> {
    // M2+: dispatch to home/detail/communities/me/admin handlers.
    render::placeholder()
}

pub async fn dispatch_post(
    _req: Request,
    _env: &Env,
    _request_id: &str,
    _path: &str,
) -> Result<Response> {
    // M2+: dispatch to status/note/admin POST handlers.
    render::placeholder()
}
