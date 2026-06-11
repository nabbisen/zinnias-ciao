use worker::{Env, Request, Response, Result};
use crate::render;

pub async fn post_logout(_req: Request, _env: &Env, _rid: &str) -> Result<Response> {
    // M1: validate form token, revoke session, clear cookie, 303 -> /join.
    render::placeholder()
}
