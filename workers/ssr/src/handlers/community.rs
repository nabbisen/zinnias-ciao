//! Community-scoped route dispatcher.

use worker::{Env, Request, Response, Result};
use crate::render;

pub async fn dispatch_get(req: Request, env: &Env, rid: &str, path: &str) -> Result<Response> {
    let rest = &path[3..];
    let (cid, tail) = split_once(rest, '/');
    if cid.is_empty() { return render::not_found(); }

    let url = req.url()?;
    let flash: Option<String> = url.query_pairs()
        .find(|(k, _)| k == "flash").map(|(_, v)| v.to_string());

    match tail {
        "home" | "" | "/" => super::home::get_home(req, env, rid, cid).await,

        t if t.starts_with("events/") => {
            let (eid, sub) = split_once(&t[7..], '/');
            if sub.is_empty() {
                super::event::get_event_detail(req, env, rid, cid, eid, flash.as_deref()).await
            } else { render::not_found() }
        }

        "communities" => super::communities::get_communities(req, env, rid, cid).await,
        "me"          => super::me::get_me(req, env, rid, cid).await,

        // ── Admin GET routes ─────────────────────────────────────────────
        "admin" | "admin/" => render::not_found(),
        t if t.starts_with("admin/") => {
            let admin_tail = &t[6..];
            match admin_tail {
                "events/new" =>
                    super::admin::get_create_event(req, env, rid, cid).await,
                t if t.starts_with("events/") => {
                    let (eid, sub) = split_once(&t[7..], '/');
                    match sub {
                        "cancel" => super::admin::get_cancel_event(req, env, rid, cid, eid).await,
                        _ => render::not_found(),
                    }
                }
                "invites" => super::admin::get_invites(req, env, rid, cid).await,
                "members" => super::admin::get_members(req, env, rid, cid).await,
                t if t.starts_with("members/") => {
                    let (mid, sub) = split_once(&t[8..], '/');
                    if sub == "remove" {
                        super::admin::get_remove_member(req, env, rid, cid, mid).await
                    } else { render::not_found() }
                }
                _ => render::not_found(),
            }
        }

        _ => render::not_found(),
    }
}

pub async fn dispatch_post(req: Request, env: &Env, rid: &str, path: &str) -> Result<Response> {
    let rest = &path[3..];
    let (cid, tail) = split_once(rest, '/');
    if cid.is_empty() { return render::not_found(); }

    match tail {
        t if t.starts_with("events/") => {
            let (eid, sub) = split_once(&t[7..], '/');
            match sub {
                s if s.starts_with("days/") => {
                    let (day_id, action) = split_once(&s[5..], '/');
                    if action == "my-status" {
                        super::event::post_my_status(req, env, rid, cid, eid, day_id).await
                    } else { render::not_found() }
                }
                "my-note"        => super::event::post_my_note(req, env, rid, cid, eid).await,
                "my-note/delete" => super::event::delete_my_note(req, env, rid, cid, eid).await,
                _ => render::not_found(),
            }
        }
        "select" => {
            let mut r = worker::Response::empty()?;
            r.headers_mut().set("Location", &format!("/c/{cid}/home"))?;
            Ok(r.with_status(303))
        }
        // ── Admin POST routes ─────────────────────────────────────────────
        t if t.starts_with("admin/") => {
            let admin_tail = &t[6..];
            match admin_tail {
                "events" =>
                    super::admin::post_create_event(req, env, rid, cid).await,
                t if t.starts_with("events/") => {
                    let (eid, sub) = split_once(&t[7..], '/');
                    match sub {
                        "cancel" => super::admin::post_cancel_event(req, env, rid, cid, eid).await,
                        _ => render::not_found(),
                    }
                }
                "invites" => super::admin::post_generate_invite(req, env, rid, cid).await,
                t if t.starts_with("members/") => {
                    let (mid, sub) = split_once(&t[8..], '/');
                    if sub == "remove" {
                        super::admin::post_remove_member(req, env, rid, cid, mid).await
                    } else { render::not_found() }
                }
                _ => render::not_found(),
            }
        }
        _ => render::not_found(),
    }
}

fn split_once(s: &str, sep: char) -> (&str, &str) {
    match s.find(sep) {
        Some(i) => (&s[..i], &s[i + sep.len_utf8()..]),
        None    => (s, ""),
    }
}
