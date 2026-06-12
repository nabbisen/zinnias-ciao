//! Community-scoped route dispatcher — parses /c/:cid/... paths.

use worker::{Env, Request, Response, Result};
use crate::render;

/// Parse /c/:cid/... and dispatch to the right handler.
pub async fn dispatch_get(
    req: Request,
    env: &Env,
    rid: &str,
    path: &str,
) -> Result<Response> {
    // path starts with "/c/"
    let rest = &path[3..]; // strip "/c/"
    let (cid, tail) = split_once(rest, '/');
    if cid.is_empty() {
        return render::not_found();
    }

    // Parse query string for flash/error
    let url = req.url()?;
    let flash: Option<String> = url.query_pairs()
        .find(|(k, _)| k == "flash")
        .map(|(_, v)| v.to_string());

    match tail {
        "home" | "" | "/" => {
            super::home::get_home(req, env, rid, cid).await
        }
        t if t.starts_with("events/") => {
            let event_rest = &t[7..]; // strip "events/"
            let (eid, sub) = split_once(event_rest, '/');
            if sub.is_empty() {
                super::event::get_event_detail(
                    req, env, rid, cid, eid,
                    flash.as_deref(),
                ).await
            } else {
                render::not_found()
            }
        }
        "communities" => render::placeholder(),
        "me"          => render::placeholder(),
        _             => render::not_found(),
    }
}

pub async fn dispatch_post(
    req: Request,
    env: &Env,
    rid: &str,
    path: &str,
) -> Result<Response> {
    let rest = &path[3..];
    let (cid, tail) = split_once(rest, '/');
    if cid.is_empty() {
        return render::not_found();
    }

    match tail {
        t if t.starts_with("events/") => {
            let event_rest = &t[7..];
            let (eid, sub) = split_once(event_rest, '/');
            match sub {
                // POST /c/:cid/events/:eid/days/:dayId/my-status
                s if s.starts_with("days/") => {
                    let day_rest = &s[5..];
                    let (day_id, action) = split_once(day_rest, '/');
                    if action == "my-status" {
                        super::event::post_my_status(req, env, rid, cid, eid, day_id).await
                    } else {
                        render::not_found()
                    }
                }
                "my-note"        => super::event::post_my_note(req, env, rid, cid, eid).await,
                "my-note/delete" => super::event::delete_my_note(req, env, rid, cid, eid).await,
                _ => render::not_found(),
            }
        }
        "select" => {
            // POST /c/:cid/select — community switcher (stub for M3)
            let mut r = worker::Response::empty()?;
            r.headers_mut().set("Location", &format!("/c/{cid}/home"))?;
            Ok(r.with_status(303))
        }
        _ => render::not_found(),
    }
}

/// Split a string at the first occurrence of `sep`, returning (before, after).
/// If sep is not found, returns (whole, "").
fn split_once(s: &str, sep: char) -> (&str, &str) {
    match s.find(sep) {
        Some(i) => (&s[..i], &s[i + sep.len_utf8()..]),
        None    => (s, ""),
    }
}
