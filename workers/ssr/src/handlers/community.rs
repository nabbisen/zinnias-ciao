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
                Some(next) if next.starts_with("communities:") => {
                    calendar_next_destination(cid, next)
                        .unwrap_or_else(|| format!("/c/{cid}/communities"))
                }
                Some("admin_events_new") => format!("/c/{cid}/admin/events/new"),
                Some(next) if next.starts_with("admin_events_new:") => {
                    admin_events_new_destination(cid, next)
                        .unwrap_or_else(|| format!("/c/{cid}/admin/events/new"))
                }
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

fn calendar_next_destination(cid: &str, next: &str) -> Option<String> {
    let mut parts = next.split(':');
    if parts.next()? != "communities" {
        return None;
    }
    let month = parts.next()?;
    let day = parts.next();
    if parts.next().is_some() || parse_month(month).is_none() {
        return None;
    }
    let mut dest = format!("/c/{cid}/communities?month={month}");
    if let Some(day) = day {
        let (year, month_num) = parse_month(month)?;
        let (day_year, day_month, _) = parse_ymd(day)?;
        if day_year != year || day_month != month_num {
            return None;
        }
        dest.push_str("&day=");
        dest.push_str(day);
    }
    Some(dest)
}

fn admin_events_new_destination(cid: &str, next: &str) -> Option<String> {
    let mut parts = next.split(':');
    if parts.next()? != "admin_events_new" {
        return None;
    }
    let day = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    parse_ymd(day)?;
    Some(format!("/c/{cid}/admin/events/new?day={day}"))
}

fn parse_month(month: &str) -> Option<(i32, i32)> {
    if month.len() != 7 || month.get(4..5)? != "-" {
        return None;
    }
    let year = month.get(..4)?.parse::<i32>().ok()?;
    let month = month.get(5..7)?.parse::<i32>().ok()?;
    if !(1..=12).contains(&month) {
        return None;
    }
    Some((year, month))
}

fn parse_ymd(date: &str) -> Option<(i32, i32, i32)> {
    if date.len() != 10 || date.get(4..5)? != "-" || date.get(7..8)? != "-" {
        return None;
    }
    let year = date.get(..4)?.parse::<i32>().ok()?;
    let month = date.get(5..7)?.parse::<i32>().ok()?;
    let day = date.get(8..10)?.parse::<i32>().ok()?;
    if !(1..=12).contains(&month) || !(1..=days_in_month(year, month)).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
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
                        "recreate" => {
                            super::admin::get_recreate_event(req, env, rid, cid, eid).await
                        }
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

#[cfg(test)]
mod tests;
