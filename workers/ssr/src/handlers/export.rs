//! Admin community data export (RFC-027).
//!
//! Routes:
//!   GET  /c/:cid/admin/export       — export landing page
//!   GET  /c/:cid/admin/export/json  — JSON download (bearer via query token)
//!
//! Exports: events, event days, attendance, notes, members.
//! Excludes: session tokens, invite HMACs, HMAC pepper, raw logs.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;

use crate::authz::require_admin;
use crate::db::{self, community as community_db, membership as membership_db};
use crate::render;
use crate::session::require_auth;
use zinnias_ciao_contracts::i18n;

// ── GET /c/:cid/admin/export ──────────────────────────────────────────────

pub async fn get_export_page(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    // Issue a one-time download token bound to this admin.
    let dl_token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::COMMUNITY_EXPORT,
        Some(community_id),
    )
    .await;

    let communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await
        .unwrap_or_default();
    let community_pairs: Vec<(String, String)> = communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();

    let community = community_db::find_active(&db, community_id).await?;
    let community_name = community.map(|c| c.name).unwrap_or_default();

    // Count rows for the summary
    let event_count = count_events(&db, community_id).await.unwrap_or(0);
    let member_count = membership_db::count_active(&db, community_id)
        .await
        .unwrap_or(0);

    let nav = render::bottom_nav(community_id, "home");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">{exp_title}</h1>\
         <p style=\"font-size:.875rem;color:#6E6E73;margin-bottom:1.5rem\">\
           Download a JSON file of your community's events, attendance, and notes.\
           Member names and notes are included. Session tokens and security credentials\
           are not included.\
         </p>\
         <div style=\"background:#F5F5F7;border-radius:12px;padding:1rem;margin-bottom:1.5rem\">\
           <p style=\"font-size:.875rem;margin:0 0 .25rem\"><strong>{name}</strong></p>\
           <p style=\"font-size:.8125rem;color:#6E6E73;margin:0\">\
             {events} events · {members} active members\
           </p>\
         </div>\
         <a href=\"/c/{cid}/admin/export/json?token={token}\" \
            download=\"{slug}-export.json\" \
            style=\"display:flex;align-items:center;justify-content:center;\
            padding:.875rem;background:#007AFF;color:#fff;\
            border-radius:14px;font-size:1rem;font-weight:600;\
            text-decoration:none;min-height:44px;margin-bottom:1rem\">\
            Download JSON\
         </a>\
         <p style=\"font-size:.75rem;color:#6E6E73;text-align:center\">\
           This link is single-use and expires in 5 minutes.\
         </p>\
         </main>{nav}",
        exp_title = i18n::JA_EXPORT_TITLE,
        header =
            render::header_with_switcher(i18n::JA_EXPORT_TITLE, community_id, &community_pairs),
        name = render::escape_html(&community_name),
        events = event_count,
        members = member_count,
        cid = render::escape_html(community_id),
        token = render::escape_html(&dl_token),
        slug = render::escape_html(&slugify(&community_name)),
        nav = nav,
    );
    render::page(i18n::JA_EXPORT_TITLE, &body)
}

// ── GET /c/:cid/admin/export/json?token=… ────────────────────────────────

pub async fn get_export_json(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    // Validate the one-time download token from the query string.
    let url = req.url()?;
    let raw_token = url
        .query_pairs()
        .find(|(k, _)| k == "token")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();

    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::COMMUNITY_EXPORT,
        &raw_token,
        Some(community_id),
    )
    .await?;
    if replay.is_some() {
        // Already used — redirect back to export page for a fresh token.
        return redirect(&format!("/c/{community_id}/admin/export"));
    }
    if raw_token.is_empty() {
        return redirect(&format!("/c/{community_id}/admin/export"));
    }

    // Build the export payload.
    let payload = build_export(&db, community_id).await?;
    let json = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_owned());

    let community = community_db::find_active(&db, community_id).await?;
    let community_name = community.map(|c| c.name).unwrap_or_default();
    let filename = format!("{}-export.json", slugify(&community_name));

    // Audit the export (no content logged).
    let _ = crate::audit::write(
        &db,
        rid,
        Some(community_id),
        Some(&membership.membership_id),
        "community",
        Some(community_id),
        "exported",
        None,
    )
    .await;

    let mut resp = Response::ok(json)?;
    resp.headers_mut()
        .set("Content-Type", "application/json; charset=utf-8")?;
    resp.headers_mut().set(
        "Content-Disposition",
        &format!("attachment; filename=\"{filename}\""),
    )?;
    resp.headers_mut()
        .set("Cache-Control", "no-store, private")?;
    Ok(resp)
}

// ── Export payload builder ────────────────────────────────────────────────

async fn build_export(
    db: &worker::d1::D1Database,
    community_id: &str,
) -> Result<serde_json::Value> {
    // Members (active + removed, for attendance label completeness)
    let members_raw = db
        .prepare(
            "SELECT id, display_name, role, joined_at, removed_at \
             FROM community_memberships \
             WHERE community_id = ?1 \
             ORDER BY joined_at ASC",
        )
        .bind(&[community_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    let members: Vec<serde_json::Value> = members_raw
        .iter()
        .map(|m| {
            serde_json::json!({
                "id":           m.get("id").and_then(|x| x.as_str()).unwrap_or(""),
                "display_name": m.get("display_name").and_then(|x| x.as_str()).unwrap_or(""),
                "role":         m.get("role").and_then(|x| x.as_str()).unwrap_or("member"),
                "joined_at":    m.get("joined_at").and_then(|x| x.as_str()).unwrap_or(""),
                "removed":      m.get("removed_at").and_then(|x| x.as_str()).is_some(),
            })
        })
        .collect();

    // Name map for display
    let name_map: std::collections::HashMap<String, String> = members_raw
        .iter()
        .filter_map(|m| {
            let id = m.get("id")?.as_str()?.to_owned();
            let name = m.get("display_name")?.as_str()?.to_owned();
            Some((id, name))
        })
        .collect();

    // Events with their days, attendance, and notes
    let events_raw = db
        .prepare(
            "SELECT id, title, description, location, status, created_at \
             FROM events \
             WHERE community_id = ?1 \
             ORDER BY created_at ASC",
        )
        .bind(&[community_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    // Collect all event IDs for the batched sub-queries below.
    let event_ids: Vec<&str> = events_raw
        .iter()
        .filter_map(|ev| ev.get("id").and_then(|x| x.as_str()))
        .collect();

    if event_ids.is_empty() {
        let community = community_db::find_active(db, community_id).await?;
        return Ok(serde_json::json!({
            "export_version": 1,
            "exported_at":    db::now_utc(),
            "community": {
                "id":   community_id,
                "name": community.map(|c| c.name).unwrap_or_default(),
            },
            "members": members,
            "events":  [],
        }));
    }

    // Batch 1: all days for all events in one IN query.
    let ev_placeholders = zinnias_ciao_contracts::build_in_placeholders(event_ids.len(), 0);
    let ev_binds: Vec<worker::wasm_bindgen::JsValue> =
        event_ids.iter().map(|id| (*id).into()).collect();
    let all_days_raw = db
        .prepare(&format!(
            "SELECT id, event_id, seq, day_date, starts_at_utc, ends_at_utc \
             FROM event_days WHERE event_id IN ({ev_placeholders}) ORDER BY event_id, seq ASC"
        ))
        .bind(&ev_binds)?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    // Collect all day IDs for the attendance batch.
    let all_day_ids: Vec<String> = all_days_raw
        .iter()
        .filter_map(|d| d.get("id").and_then(|x| x.as_str()).map(|s| s.to_owned()))
        .collect();
    let day_id_refs: Vec<&str> = all_day_ids.iter().map(|s| s.as_str()).collect();

    // Batch 2: all attendance rows for all days in one IN query.
    let mut att_by_day: std::collections::HashMap<String, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    if !all_day_ids.is_empty() {
        let day_placeholders = zinnias_ciao_contracts::build_in_placeholders(all_day_ids.len(), 0);
        let day_binds: Vec<worker::wasm_bindgen::JsValue> =
            day_id_refs.iter().map(|id| (*id).into()).collect();
        let att_raw = db
            .prepare(&format!(
                "SELECT event_day_id, membership_id, status, status_updated_at \
                 FROM attendances WHERE event_day_id IN ({day_placeholders})"
            ))
            .bind(&day_binds)?
            .all()
            .await?
            .results::<serde_json::Value>()?;
        for a in att_raw {
            let did = a
                .get("event_day_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_owned();
            att_by_day.entry(did).or_default().push(a);
        }
    }

    // Batch 3: all visible notes for all events in one IN query.
    let notes_sql = format!(
        "SELECT event_id, membership_id, note, note_updated_at \
         FROM event_notes \
         WHERE event_id IN ({ev_placeholders}) \
           AND note_deleted_at IS NULL \
           AND hidden_by_admin_at IS NULL \
         ORDER BY event_id, note_updated_at ASC"
    );
    let notes_all_raw = db
        .prepare(&notes_sql)
        .bind(&ev_binds)?
        .all()
        .await?
        .results::<serde_json::Value>()?;
    let mut notes_by_event: std::collections::HashMap<String, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    for n in notes_all_raw {
        let eid = n
            .get("event_id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_owned();
        notes_by_event.entry(eid).or_default().push(n);
    }

    // Group days by event_id.
    let mut days_by_event: std::collections::HashMap<&str, Vec<&serde_json::Value>> =
        std::collections::HashMap::new();
    for day in &all_days_raw {
        let eid = day.get("event_id").and_then(|x| x.as_str()).unwrap_or("");
        days_by_event.entry(eid).or_default().push(day);
    }

    // Assemble events without any additional DB queries.
    let mut events_out = Vec::new();
    for ev in &events_raw {
        let event_id = match ev.get("id").and_then(|x| x.as_str()) {
            Some(id) => id,
            None => continue,
        };

        let days_out: Vec<serde_json::Value> = days_by_event.get(event_id)
            .map(|days| days.iter().map(|day| {
                let day_id = day.get("id").and_then(|x| x.as_str()).unwrap_or("");
                let attendance: Vec<serde_json::Value> = att_by_day.get(day_id)
                    .map(|rows| rows.iter().map(|a| {
                        let mid  = a.get("membership_id").and_then(|x| x.as_str()).unwrap_or("");
                        let name = name_map.get(mid).map(|s| s.as_str()).unwrap_or("[removed member]");
                        serde_json::json!({
                            "member":     name,
                            "status":     a.get("status").and_then(|x| x.as_str()).unwrap_or("no_answer"),
                            "updated_at": a.get("status_updated_at").and_then(|x| x.as_str()),
                        })
                    }).collect())
                    .unwrap_or_default();
                serde_json::json!({
                    "seq":        day.get("seq").and_then(|x| x.as_u64()),
                    "date":       day.get("day_date").and_then(|x| x.as_str()),
                    "starts_at":  day.get("starts_at_utc").and_then(|x| x.as_str()),
                    "ends_at":    day.get("ends_at_utc").and_then(|x| x.as_str()),
                    "attendance": attendance,
                })
            }).collect())
            .unwrap_or_default();

        let notes: Vec<serde_json::Value> = notes_by_event
            .get(event_id)
            .map(|ns| {
                ns.iter()
                    .map(|n| {
                        let mid = n
                            .get("membership_id")
                            .and_then(|x| x.as_str())
                            .unwrap_or("");
                        let name = name_map
                            .get(mid)
                            .map(|s| s.as_str())
                            .unwrap_or("[removed member]");
                        serde_json::json!({
                            "member":     name,
                            "note":       n.get("note").and_then(|x| x.as_str()).unwrap_or(""),
                            "updated_at": n.get("note_updated_at").and_then(|x| x.as_str()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        events_out.push(serde_json::json!({
            "id":          event_id,
            "title":       ev.get("title").and_then(|x| x.as_str()),
            "description": ev.get("description").and_then(|x| x.as_str()),
            "location":    ev.get("location").and_then(|x| x.as_str()),
            "status":      ev.get("status").and_then(|x| x.as_str()).unwrap_or("scheduled"),
            "created_at":  ev.get("created_at").and_then(|x| x.as_str()),
            "days":        days_out,
            "notes":       notes,
        }));
    }

    let community = community_db::find_active(db, community_id).await?;

    Ok(serde_json::json!({
        "export_version": 1,
        "exported_at":    db::now_utc(),
        "community": {
            "id":   community_id,
            "name": community.map(|c| c.name).unwrap_or_default(),
        },
        "members": members,
        "events":  events_out,
    }))
}

// ── Helpers ───────────────────────────────────────────────────────────────

async fn count_events(db: &worker::d1::D1Database, community_id: &str) -> Result<u32> {
    let row = db
        .prepare("SELECT COUNT(*) AS n FROM events WHERE community_id = ?1")
        .bind(&[community_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row
        .and_then(|v| v.get("n").and_then(|x| x.as_u64()))
        .unwrap_or(0) as u32)
}

fn slugify(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .map(|c| {
            if c.is_whitespace() {
                '-'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(40)
        .collect()
}

fn redirect(location: &str) -> Result<Response> {
    let mut resp = Response::from_html("")?;
    resp.headers_mut().set("Location", location)?;
    Ok(resp.with_status(303))
}
