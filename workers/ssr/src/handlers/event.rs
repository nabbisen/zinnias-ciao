//! Event Detail, status, and note handlers (RFC-005/006/007).

use zinnias_ciao_contracts::auth::token_purpose;
use worker::{Env, Request, Response, Result};

use crate::audit;
use crate::authz::require_membership;
use zinnias_ciao_contracts::i18n;
use crate::db::{
    self,
    attendance as attendance_db,
    event as event_db,
    event_note as note_db,
    membership as membership_db,
};
use crate::form_token;
use crate::render::{self, ParticipantEntry};
use crate::session::require_auth;
use zinnias_ciao_domain::{
    status::{validate_status_transition, AttendanceStatus, DayTimeState, Role},
    validate_note,
};


fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

// ── GET /c/:cid/events/:eid ──────────────────────────────────────────────

pub async fn get_event_detail(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
    flash: Option<&str>,
    err: Option<&str>,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };

    let days  = event_db::days_for_event(&db, event_id).await?;
    let member_count = membership_db::count_active(&db, community_id).await?;
    let my_note = note_db::find_mine(&db, event_id, &membership.membership_id).await?;
    let all_notes = note_db::list_for_event(&db, event_id).await?;
    let all_members = membership_db::list_all_active(&db, community_id).await?;

    // Fetch community early — needed for timezone display inside the day loop.
    let community_row = db::community::find_active(&db, community_id).await?;
    let community_tz_early = community_row.as_ref().map(|c| c.timezone.as_str()).unwrap_or("UTC");

    // Build a display-name map for participant list
    let name_map: std::collections::HashMap<String, String> = all_members
        .iter()
        .map(|m| (m.id.clone(), m.display_name.clone()))
        .collect();

    let now_utc = db::now_utc();

    // ── Batch-fetch all per-day data before the loop (RFC-029: no N+1) ───
    let day_id_strs: Vec<&str> = days.iter().map(|d| d.id.as_str()).collect();
    let my_statuses = attendance_db::list_mine_for_days(
        &db, &membership.membership_id, &day_id_strs,
    ).await?;
    let day_counts = attendance_db::counts_for_days(&db, &day_id_strs, member_count).await?;
    // Batch-fetch all per-day attendances in one IN query (RFC-029/RFC-044).
    let all_day_attendances = attendance_db::list_for_event_days(&db, &day_id_strs).await?;

    // ── Status form token (RFC-046) ──────────────────────────────────────
    // One token bound to the EVENT, reused for every day's status form on this
    // page. The POST handler validates that the submitted day belongs to this
    // event (and community) before mutating, so a single event-bound token is
    // safe and removes the per-day D1 write that previously scaled with the
    // number of recurring occurrences.
    let status_token = form_token::issue(
        &db, &pp, &auth.user_id,
        token_purpose::SET_STATUS, Some(event_id),
    ).await.unwrap_or_default();

    // ── Days section ─────────────────────────────────────────────────────
    let mut days_html = String::new();
    for day in &days {
        let time_state = classify_day(&day.starts_at_utc, &day.ends_at_utc, &now_utc);
        let current_status = my_statuses.get(&day.id).map(|s| s.as_str());
        let empty_counts = attendance_db::DayCountRow {
            going: 0, not_going: 0, attended: 0, no_answer: member_count
        };
        let counts = day_counts.get(&day.id).unwrap_or(&empty_counts);

        let can_set_attended = membership.is_admin() && time_state == DayTimeState::Ended;
        let attended_reason  = if time_state != DayTimeState::Ended {
            i18n::EN_EVENT_ATTENDED_UNAVAILABLE
        } else if !membership.is_admin() {
            i18n::EN_EVENT_ATTENDED_ADMIN_ONLY
        } else { "" };

        // Day header (status token issued once before the loop, RFC-046)
        let label = format_day_label(&day.day_date, &day.starts_at_utc, &day.ends_at_utc, days.len() > 1, day.seq, community_tz_early);

        let status_form = render::status_form(
            community_id, event_id, &day.id,
            &status_token, current_status,
            can_set_attended, attended_reason,
        );

        let (cg, cng, cna) = (counts.going, counts.not_going, counts.no_answer);
        let counts_html = format!(
            "<p style=\"font-size:.875rem;color:#6e6e73\">Going {cg} · No Go {cng} · No answer {cna}</p>",
        );

        // Participant rows for this day — from the pre-fetched batch, no N+1.
        let mut participants: Vec<ParticipantEntry> = Vec::new();
        let empty_day_att = Vec::new();
        let day_attendances = all_day_attendances.get(&day.id).unwrap_or(&empty_day_att);
        let att_map: std::collections::HashMap<&str, Option<&str>> = day_attendances
            .iter()
            .map(|a| (a.membership_id.as_str(), a.status.as_deref()))
            .collect();
        // All active members, in Going → No Go → No answer order
        let mut ordered = all_members.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|m| {
            match att_map.get(m.id.as_str()).copied().flatten() {
                Some("going")     => 0u8,
                Some("attended")  => 1,
                Some("not_going") => 2,
                _                 => 3,
            }
        });
        for m in &ordered {
            participants.push(ParticipantEntry {
                display_name: &m.display_name,
                status: att_map.get(m.id.as_str()).copied().flatten(),
            });
        }
        let plist = render::participant_list(&participants);

        days_html.push_str(&format!(
            "<div style=\"border:1px solid #e5e5ea;border-radius:16px;padding:1rem;margin-bottom:1rem\">\
             <h3 style=\"font-size:.9375rem;font-weight:600;margin-bottom:.5rem\">{label}</h3>\
             {status_form}{counts_html}\
             <details style=\"margin-top:.75rem\">\
               <summary style=\"font-size:.875rem;color:#007AFF;cursor:pointer\">Who's going?</summary>\
               <div style=\"margin-top:.5rem\">{plist}</div>\
             </details>\
             </div>",
            label       = render::escape_html(&label),
            status_form = status_form,
            counts_html = counts_html,
            plist       = plist,
        ));
    }

    // ── Note section ─────────────────────────────────────────────────────
    let save_token = form_token::issue(
        &db, &pp, &auth.user_id,
        token_purpose::SAVE_NOTE, Some(event_id),
    ).await.unwrap_or_default();

    let note_html = render::note_form(
        community_id, event_id,
        &save_token,
        my_note.as_ref().map(|n| n.note.as_str()),
        flash,
    );

    // ── Other members' notes (admin gets a link to the confirmation page) ────
    let mut others_html = String::new();
    for n in all_notes.iter().filter(|n| n.membership_id != membership.membership_id) {
        let name = name_map.get(&n.membership_id)
            .map(|s| s.as_str()).unwrap_or(i18n::EN_EVENT_MEMBER_FALLBACK);
        // The hide button is now a link to GET …/notes/:mid/hide (RFC-043).
        // Token is issued on that confirmation page — no per-note DB write here.
        let hide_btn = if membership.is_admin() {
            render::admin_note_hide_form(community_id, event_id, &n.membership_id, "")
        } else { String::new() };
        others_html.push_str(&format!(
            "<div style=\"padding:.75rem 0;border-bottom:1px solid #f5f5f7\">\
             <div style=\"display:flex;align-items:baseline;justify-content:space-between\">\
               <span style=\"font-weight:600;font-size:.875rem\">{name}</span>\
               {hide}\
             </div>\
             <p style=\"margin:.25rem 0 0;font-size:.9375rem\">{note}</p>\
             </div>",
            name = render::escape_html(name),
            note = render::escape_html(&n.note),
            hide = hide_btn,
        ));
    }
    let notes_section = if !others_html.is_empty() {
        format!(
            "<section style=\"margin-top:1.5rem\">\
             <h2 style=\"font-size:1.0625rem;font-weight:600;margin-bottom:.5rem\">Notes</h2>\
             {others_html}</section>"
        )
    } else { String::new() };

    let community = community_row;
    let _community_name = community.as_ref().map(|c| c.name.as_str()).unwrap_or_default();
    let _community_tz  = community.as_ref().map(|c| c.timezone.as_str()).unwrap_or("UTC");
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav  = render::bottom_nav(community_id, "home");
    let back = format!(
        "<a href=\"/c/{}/home\" style=\"color:#007AFF;font-size:.9375rem\">\u{2190} Home</a>",
        render::escape_html(community_id)
    );
    let cancelled_banner = if event.status == "cancelled" {
        "<div style=\"background:#FF3B3022;color:#FF3B30;padding:.75rem;border-radius:12px;margin-bottom:1rem\">This event was cancelled.</div>"
    } else { "" };

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           {back}\
           {err_banner}\
           <h1 style=\"font-size:1.25rem;font-weight:600;margin:1rem 0 .25rem\">{title}</h1>\
           {loc}{desc}\
           {cancelled}\
           {days}\
           {note}\
           {notes_section}\
         </main>{nav}",
        header         = render::header_with_switcher(i18n::EN_EVENT_TITLE_HEADER, community_id, &_community_pairs),
        err_banner     = err.map(|e| format!(
            "<p role=\"alert\" style=\"background:#FFF0EF;color:#B42318;padding:.75rem;\
             border-radius:12px;font-size:.9375rem;margin:.5rem 0\">{}</p>",
            render::escape_html(e)
        )).unwrap_or_default(),
        title          = render::escape_html(&event.title),
        loc            = event.location.as_deref().map(|l| format!(
            "<p style=\"color:#6e6e73;font-size:.875rem\">\u{1F4CD} {}</p>",
            render::escape_html(l)
        )).unwrap_or_default(),
        desc           = event.description.as_deref().map(|d| format!(
            "<p style=\"font-size:.9375rem;margin:.5rem 0\">{}</p>",
            render::escape_html(d)
        )).unwrap_or_default(),
        cancelled      = cancelled_banner,
        days           = days_html,
        note           = note_html,
        notes_section  = notes_section,
        nav            = nav,
    );
    render::page(&event.title, &body)
}

// ── POST /c/:cid/events/:eid/days/:dayId/my-status ───────────────────────

pub async fn post_my_status(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
    day_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token  = body.get_field("_token").unwrap_or_default();
    let raw_status = body.get_field("status").unwrap_or_default();

    // Validate and consume form token (CSRF + idempotency, AD-4).
    // The token is bound to the EVENT (RFC-046); the day is identified by the
    // URL path and validated for event/community ownership below.
    let replay = form_token::consume(
        &db, &pp, &auth.user_id,
        token_purpose::SET_STATUS, &raw_token, Some(event_id),
    ).await?;

    if replay.is_some() {
        // Already processed — redirect to detail (idempotent)
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    // Load the day to compute time state
    let days = event_db::days_for_event(&db, event_id).await?;
    let day  = days.iter().find(|d| d.id == day_id)
        .ok_or_else(|| worker::Error::RustError("Day not found".to_string()))?;
    let event = event_db::find_for_community(&db, event_id, community_id).await?
        .ok_or_else(|| worker::Error::RustError("Event not found".to_string()))?;

    let now_utc    = db::now_utc();
    let time_state = classify_day(&day.starts_at_utc, &day.ends_at_utc, &now_utc);
    let role = if membership.is_admin() { Role::Admin } else { Role::Member };
    let is_cancelled = event.status == "cancelled";

    // Parse requested status ("clear" means None = No answer)
    let requested: Option<AttendanceStatus> = match raw_status.as_str() {
        "going"     => Some(AttendanceStatus::Going),
        "not_going" => Some(AttendanceStatus::NotGoing),
        "attended"  => Some(AttendanceStatus::Attended),
        _           => None, // "clear" or anything else
    };

    let current_att = attendance_db::find_mine(&db, day_id, &membership.membership_id).await?;
    let current = current_att.as_ref()
        .and_then(|a| a.status.as_deref())
        .and_then(|s| match s {
            "going"     => Some(AttendanceStatus::Going),
            "not_going" => Some(AttendanceStatus::NotGoing),
            "attended"  => Some(AttendanceStatus::Attended),
            _           => None,
        });

    if let Err(e) = validate_status_transition(role, time_state, is_cancelled, current, requested) {
        // Return to detail with error in query param (simple, no flash cookie needed)
        let msg = render::escape_html(&e.to_string());
        return redirect(&format!("/c/{community_id}/events/{event_id}?err={msg}"));
    }

    // Persist
    let status_str: Option<&str> = match requested {
        Some(AttendanceStatus::Going)    => Some("going"),
        Some(AttendanceStatus::NotGoing) => Some("not_going"),
        Some(AttendanceStatus::Attended) => Some("attended"),
        None                             => None,
    };
    attendance_db::upsert(&db, day_id, &membership.membership_id, status_str).await?;

    // Audit admin attendance correction
    if membership.is_admin() && matches!(requested, Some(AttendanceStatus::Attended)) {
        let _ = audit::write(
            &db, rid, Some(community_id), Some(&membership.membership_id),
            "attendance", Some(day_id), "admin_set_attended",
            Some(serde_json::json!({ "event_id": event_id })),
        ).await;
    }

    redirect(&format!("/c/{community_id}/events/{event_id}"))
}

// ── POST /c/:cid/events/:eid/my-note ────────────────────────────────────

pub async fn post_my_note(
    mut req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let raw_note  = body.get_field("note").unwrap_or_default();

    let replay = form_token::consume(
        &db, &pp, &auth.user_id,
        token_purpose::SAVE_NOTE, &raw_token, Some(event_id),
    ).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}?flash=saved"));
    }

    let note: String = match validate_note(&raw_note) {
        Ok(n)  => n,
        Err(e) => {
            let msg = render::escape_html(&e.to_string());
            return redirect(&format!("/c/{community_id}/events/{event_id}?err={msg}"));
        }
    };

    note_db::upsert(&db, event_id, &membership.membership_id, &note).await?;
    redirect(&format!("/c/{community_id}/events/{event_id}?flash=saved"))
}

// ── GET /c/:cid/events/:eid/my-note/delete ───────────────────────────────
// No-JS confirmation page (RFC-043). The delete button in Event Detail links
// here; the page renders a server-issued token form; POST proceeds to delete.

pub async fn get_delete_note_confirm(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    // Issue a fresh DELETE_NOTE token (the Event Detail page's token is not
    // carried here — this is a new page with its own server-issued token).
    let token = form_token::issue(
        &db, &pp, &auth.user_id,
        token_purpose::DELETE_NOTE, Some(event_id),
    ).await.unwrap_or_default();

    // Only show the confirmation if the member actually has a note.
    let my_note = note_db::find_mine(&db, event_id, &membership.membership_id).await?;
    if my_note.is_none() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    let communities = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await.unwrap_or_default();
    let pairs: Vec<(String, String)> = communities.iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">Delete note?</h1>\
           <p style=\"font-size:.9375rem;color:#6E6E73;margin-bottom:1.5rem\">\
             Your note will be removed. This cannot be undone.</p>\
           <div style=\"display:flex;gap:.75rem\">\
             <a href=\"/c/{cid}/events/{eid}\" \
                style=\"flex:1;padding:.875rem;border:2px solid #e5e5ea;border-radius:14px;\
                text-align:center;text-decoration:none;color:#1D1D1F;font-weight:600;min-height:44px;\
                display:flex;align-items:center;justify-content:center\">Keep note</a>\
             <form method=\"post\" action=\"/c/{cid}/events/{eid}/my-note/delete\" style=\"flex:1\">\
               <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
               <button type=\"submit\" \
                 style=\"width:100%;padding:.875rem;background:#FF3B30;color:#fff;\
                 border:none;border-radius:14px;font-weight:600;min-height:44px;cursor:pointer\">\
                 Delete note</button>\
             </form>\
           </div>\
         </main>{nav}",
        header = render::header_with_switcher("Delete note", community_id, &pairs),
        cid    = render::escape_html(community_id),
        eid    = render::escape_html(event_id),
        tok    = render::escape_html(&token),
        nav    = nav,
    );
    render::page("Delete note", &body)
}

// ── POST /c/:cid/events/:eid/my-note/delete ──────────────────────────────

pub async fn delete_my_note(
    mut req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_membership(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();

    let replay = form_token::consume(
        &db, &pp, &auth.user_id,
        token_purpose::DELETE_NOTE, &raw_token, Some(event_id),
    ).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    note_db::soft_delete(&db, event_id, &membership.membership_id).await?;
    redirect(&format!("/c/{community_id}/events/{event_id}"))
}

// ── Helpers ───────────────────────────────────────────────────────────────

pub fn classify_day(starts: &str, ends: &str, now: &str) -> DayTimeState {
    if now < starts {
        DayTimeState::Upcoming
    } else if now < ends {
        DayTimeState::Started
    } else {
        DayTimeState::Ended
    }
}

fn format_day_label(day_date: &str, starts: &str, ends: &str, multi: bool, seq: u32, tz: &str) -> String {
    let offset = render::tz_offset_minutes_pub(tz);
    let (local_date, start_hm) = render::utc_to_local_parts_pub(starts, offset);
    let end_hm = render::apply_offset_time_pub(ends, offset);
    // Japan-first deployment: render the calendar date in Japanese convention,
    // e.g. "6月14日（土）", instead of an English month abbreviation (RFC-047).
    // The source date string is the local "YYYY-MM-DD" produced above.
    let date_src = if local_date.is_empty() { day_date } else { &local_date };
    let display_date = zinnias_ciao_contracts::tz::date_label_ja(date_src);
    if multi {
        format!("Day {seq} — {display_date} {start_hm}–{end_hm}")
    } else {
        format!("{display_date} {start_hm}–{end_hm}")
    }
}
