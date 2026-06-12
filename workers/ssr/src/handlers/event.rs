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

        // Issue a status form token
        let status_token = form_token::issue(
            &db, &pp, &auth.user_id,
            token_purpose::SET_STATUS, Some(&day.id),
        ).await.unwrap_or_default();

        // Day header
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

        // Participant rows for this day
        let mut participants: Vec<ParticipantEntry> = Vec::new();
        let day_attendances = attendance_db::list_for_day(&db, &day.id).await?;
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
    let delete_token = if my_note.is_some() {
        Some(form_token::issue(
            &db, &pp, &auth.user_id,
            token_purpose::DELETE_NOTE, Some(event_id),
        ).await.unwrap_or_default())
    } else { None };

    let note_html = render::note_form(
        community_id, event_id,
        &save_token, delete_token.as_deref(),
        my_note.as_ref().map(|n| n.note.as_str()),
        flash,
    );

    // ── Other members' notes (admin gets a hide button per note) ────
    let mut others_html = String::new();
    for n in all_notes.iter().filter(|n| n.membership_id != membership.membership_id) {
        let name = name_map.get(&n.membership_id)
            .map(|s| s.as_str()).unwrap_or(i18n::EN_EVENT_MEMBER_FALLBACK);
        let hide_btn = if membership.is_admin() {
            let hide_tok = form_token::issue(
                &db, &pp, &auth.user_id,
                token_purpose::ADMIN_HIDE_NOTE, Some(event_id),
            ).await.unwrap_or_default();
            format!(
                "<form method=\"post\" \
                  action=\"/c/{cid}/admin/events/{eid}/notes/{mid}/hide\" \
                  style=\"display:inline;margin-left:.5rem\">\
                  <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
                  <button type=\"submit\" \
                    style=\"font-size:.75rem;color:#FF3B30;background:none;border:none;\
                    cursor:pointer;padding:.125rem .25rem;min-height:44px\" \
                    aria-label=\"Hide this note\">\
                    Hide\
                  </button>\
                </form>",
                cid = render::escape_html(community_id),
                eid = render::escape_html(event_id),
                mid = render::escape_html(&n.membership_id),
                tok = render::escape_html(&hide_tok),
            )
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

    // Validate and consume form token (CSRF + idempotency, AD-4)
    let replay = form_token::consume(
        &db, &pp, &auth.user_id,
        token_purpose::SET_STATUS, &raw_token, Some(day_id),
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
    let display_date = if local_date.is_empty() { day_date } else { &local_date };
    if multi {
        format!("Day {seq} — {display_date} {start_hm}–{end_hm}")
    } else {
        format!("{display_date} {start_hm}–{end_hm}")
    }
}
