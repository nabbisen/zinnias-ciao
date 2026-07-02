//! Community-scoped route dispatcher.

use crate::db::membership as membership_db;
use crate::render;
use crate::session::require_auth;
use worker::{Env, Request, Response, Result};

/// GET /switch?community=:id — no-JS community switcher target.
/// Validates that the authenticated user is an active member of the target
/// community before redirecting (prevents open-redirect / cross-community
/// access). Falls back to the member home on any mismatch.
pub async fn get_switch(req: Request, env: &Env, _rid: &str) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };

    let url = req.url()?;
    let target: Option<String> = url
        .query_pairs()
        .find(|(k, _)| k == "community")
        .map(|(_, v)| v.to_string());
    let next: Option<String> = url
        .query_pairs()
        .find(|(k, _)| k == "next")
        .map(|(_, v)| v.to_string());

    let db = env.d1("DB")?;
    let memberships = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await
        .unwrap_or_default();

    // Only redirect to a community the user actually belongs to.
    let dest = match target {
        Some(ref cid) if memberships.iter().any(|m| &m.community_id == cid) => {
            match next.as_deref() {
                Some("communities") => format!("/c/{cid}/communities"),
                _ => format!("/c/{cid}/home"),
            }
        }
        // Unknown / non-member target: send to their first community, or /join.
        _ => match memberships.first() {
            Some(m) => format!("/c/{}/home", m.community_id),
            None => "/join".to_string(),
        },
    };

    let mut resp = Response::from_html("")?;
    resp.headers_mut().set("Location", &dest)?;
    Ok(resp.with_status(303))
}

pub async fn dispatch_get(req: Request, env: &Env, rid: &str, path: &str) -> Result<Response> {
    let rest = &path[3..];
    let (cid, tail) = split_once(rest, '/');
    if cid.is_empty() {
        return render::not_found();
    }

    let url = req.url()?;
    let flash: Option<String> = url
        .query_pairs()
        .find(|(k, _)| k == "flash")
        .map(|(_, v)| v.to_string());
    let err: Option<String> = url
        .query_pairs()
        .find(|(k, _)| k == "err")
        .map(|(_, v)| v.to_string());

    match tail {
        "home" | "" | "/" => super::home::get_home(req, env, rid, cid).await,

        t if t.starts_with("events/") => {
            let (eid, sub) = split_once(&t[7..], '/');
            match sub {
                "" => {
                    super::event::get_event_detail(
                        req,
                        env,
                        rid,
                        cid,
                        eid,
                        flash.as_deref(),
                        err.as_deref(),
                    )
                    .await
                }
                "my-note/delete" => {
                    super::event::get_delete_note_confirm(req, env, rid, cid, eid).await
                }
                _ => render::not_found(),
            }
        }

        "communities" => super::communities::get_communities(req, env, rid, cid).await,
        "me" => super::me::get_me(req, env, rid, cid).await,
        "me/calendar" => super::calendar::get_me_calendar(req, env, rid, cid).await,

        // ── Unauthenticated ICS feed ──────────────────────────────────────
        t if t.starts_with("cal/") => {
            let token = &t[4..];
            if token.is_empty() {
                render::not_found()
            } else {
                super::calendar::get_ics_feed(req, env, rid, cid, token).await
            }
        }

        // ── Admin GET routes ─────────────────────────────────────────────
        "admin" | "admin/" => render::not_found(),
        t if t.starts_with("admin/") => {
            let admin_tail = &t[6..];
            match admin_tail {
                "events/new" => super::admin::get_create_event(req, env, rid, cid).await,
                t if t.starts_with("events/") => {
                    let (eid, sub) = split_once(&t[7..], '/');
                    match sub {
                        "cancel" => super::admin::get_cancel_event(req, env, rid, cid, eid).await,
                        "edit" => super::admin::get_edit_event(req, env, rid, cid, eid).await,
                        "attendance" => super::admin::get_attendance(req, env, rid, cid, eid).await,
                        s if s.starts_with("notes/") => {
                            let (mid, action) = split_once(&s[6..], '/');
                            if action == "hide" {
                                super::admin::get_admin_hide_note_confirm(
                                    req, env, rid, cid, eid, mid,
                                )
                                .await
                            } else {
                                render::not_found()
                            }
                        }
                        _ => render::not_found(),
                    }
                }
                "invites" => super::admin::get_invites(req, env, rid, cid).await,
                "members" => super::admin::get_members(req, env, rid, cid).await,
                "export" => super::export::get_export_page(req, env, rid, cid).await,
                "export/json" => super::export::get_export_json(req, env, rid, cid).await,
                "templates" => super::templates::get_templates(req, env, rid, cid).await,
                t if t.starts_with("members/") => {
                    let (mid, sub) = split_once(&t[8..], '/');
                    if sub == "remove" {
                        super::admin::get_remove_member(req, env, rid, cid, mid).await
                    } else {
                        render::not_found()
                    }
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
    if cid.is_empty() {
        return render::not_found();
    }

    match tail {
        t if t.starts_with("events/") => {
            let (eid, sub) = split_once(&t[7..], '/');
            match sub {
                s if s.starts_with("days/") => {
                    let (day_id, action) = split_once(&s[5..], '/');
                    if action == "my-status" {
                        super::event::post_my_status(req, env, rid, cid, eid, day_id).await
                    } else {
                        render::not_found()
                    }
                }
                "my-note" => super::event::post_my_note(req, env, rid, cid, eid).await,
                "my-note/delete" => super::event::delete_my_note(req, env, rid, cid, eid).await,
                _ => render::not_found(),
            }
        }
        // ── Admin POST routes ─────────────────────────────────────────────
        t if t.starts_with("admin/") => {
            let admin_tail = &t[6..];
            match admin_tail {
                "events" => super::admin::post_create_event(req, env, rid, cid).await,
                t if t.starts_with("events/") => {
                    let (eid, sub) = split_once(&t[7..], '/');
                    match sub {
                        "cancel" => super::admin::post_cancel_event(req, env, rid, cid, eid).await,
                        "edit" => super::admin::post_edit_event(req, env, rid, cid, eid).await,
                        "attendance" => {
                            super::admin::post_attendance(req, env, rid, cid, eid).await
                        }
                        s if s.starts_with("notes/") => {
                            let (mid, action) = split_once(&s[6..], '/');
                            if action == "hide" {
                                super::admin::post_admin_hide_note(req, env, rid, cid, eid, mid)
                                    .await
                            } else {
                                render::not_found()
                            }
                        }
                        _ => render::not_found(),
                    }
                }
                "invites" => super::admin::post_generate_invite(req, env, rid, cid).await,
                t if t.starts_with("invites/") => {
                    let (iid, action) = split_once(&t[8..], '/');
                    if action == "revoke" {
                        super::admin::post_revoke_invite(req, env, rid, cid, iid).await
                    } else {
                        render::not_found()
                    }
                }
                t if t.starts_with("members/") => {
                    let (mid, sub) = split_once(&t[8..], '/');
                    if sub == "remove" {
                        super::admin::post_remove_member(req, env, rid, cid, mid).await
                    } else {
                        render::not_found()
                    }
                }
                _ => render::not_found(),
            }
        }
        "me/calendar/regenerate" => {
            super::calendar::post_regenerate_calendar(req, env, rid, cid).await
        }
        "me/calendar/revoke" => super::calendar::post_revoke_calendar(req, env, rid, cid).await,
        t if t.starts_with("admin/templates") => {
            let tail = &t[16..]; // after "admin/templates"
            if tail.is_empty() || tail == "/" {
                super::templates::post_create_template(req, env, rid, cid).await
            } else {
                // "/TID/delete"
                let (tid, action) = split_once(tail.trim_start_matches('/'), '/');
                if action == "delete" {
                    super::templates::post_delete_template(req, env, rid, cid, tid).await
                } else {
                    render::not_found()
                }
            }
        }

        _ => render::not_found(),
    }
}

fn split_once(s: &str, sep: char) -> (&str, &str) {
    match s.find(sep) {
        Some(i) => (&s[..i], &s[i + sep.len_utf8()..]),
        None => (s, ""),
    }
}
