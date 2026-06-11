use worker::{Env, Request, Response, Result};
use crate::render;

pub async fn get_join(_req: Request, _env: &Env, _rid: &str) -> Result<Response> {
    render::placeholder()
}
pub async fn post_join(_req: Request, _env: &Env, _rid: &str) -> Result<Response> {
    render::placeholder()
}
pub async fn get_profile(_req: Request, _env: &Env, _rid: &str) -> Result<Response> {
    render::placeholder()
}
pub async fn post_profile(_req: Request, _env: &Env, _rid: &str) -> Result<Response> {
    render::placeholder()
}
