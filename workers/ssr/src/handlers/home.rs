use crate::render;
use worker::{Env, Request, Response, Result};

pub async fn redirect_to_home(_req: Request, _env: &Env, _request_id: &str) -> Result<Response> {
    // M0: render the placeholder. M2 will resolve the selected community
    // from the session and redirect to /c/:cid/home.
    render::placeholder()
}
