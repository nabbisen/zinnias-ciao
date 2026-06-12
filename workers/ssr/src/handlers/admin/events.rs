//! Admin event handlers — event create, cancel, edit, attendance, hide-note (RFC-009).

use zinnias_ciao_contracts::auth::token_purpose;
use worker::{Env, Request, Response, Result};

use crate::audit;
use crate::authz::require_admin;
use crate::db::{self, event as event_db, event_write, membership as membership_db};
use crate::form_token;
use crate::render;
use crate::session::require_auth;
use crate::handlers::event::classify_day;
use zinnias_ciao_domain::{validate_event, DayInput, EventInput, RecurrenceFreq, expand_recurrence};
use zinnias_ciao_contracts::i18n;
use zinnias_ciao_domain::status::DayTimeState;

fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

// ── GET /c/:cid/admin/events/new ─────────────────────────────────────────

pub async fn get_create_event(
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
    let pp = crate::crypto::pepper(env);
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::CREATE_EVENT, None).await.unwrap_or_default();

    let _community = db::community::find_active(&db, community_id).await?;
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    // RFC-032: pre-fill from template if ?template=TID is present.
    let url = req.url()?;
    let template_id = url.query_pairs().find(|(k,_)| k == "template").map(|(_,v)| v.to_string());
    let err_msg: Option<String> = url.query_pairs().find(|(k,_)| k == "err").map(|(_,v)| v.to_string());
    let (prefill_title, prefill_location) = if let Some(ref tid) = template_id {
        let tmpl = db::event_template::find_active(&db, tid, community_id).await.ok().flatten();
        (
            tmpl.as_ref().map(|t| t.title.clone()),
            tmpl.as_ref().and_then(|t| t.location.clone()),
        )
    } else {
        (None, None)
    };

    let templates_link = format!(
        "<a href=\"/c/{cid}/admin/templates\" \
           style=\"display:block;text-align:center;color:#007AFF;\
           font-size:.875rem;margin-top:1rem;min-height:44px;line-height:44px\">\
           Use a template</a>",
        cid = render::escape_html(community_id),
    );

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">Create Event</h1>\
         <form method=\"post\" action=\"/c/{cid}/admin/events\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {fields}\
           <button type=\"submit\" style=\"width:100%;padding:.875rem;background:#007AFF;\
           color:#fff;border:none;border-radius:14px;font-size:1rem;font-weight:600;\
           min-height:44px;cursor:pointer;margin-top:1rem\">{submit}</button>\
         </form>\
         {tmpl_link}\
         </main>{nav}",
        header    = render::header_with_switcher(i18n::JA_ADMIN_CREATE_EVENT_TITLE, community_id, &_community_pairs),
        cid       = render::escape_html(community_id),
        tok       = render::escape_html(&token),
        fields    = event_form_fields(prefill_title.as_deref(), prefill_location.as_deref(), None, err_msg.as_deref(), None, None, None, true),
        submit    = i18n::JA_ADMIN_CREATE_EVENT_SUBMIT,
        tmpl_link = templates_link,
        nav       = nav,
    );
    render::page(i18n::JA_ADMIN_CREATE_EVENT_TITLE, &body)
}

// ── POST /c/:cid/admin/events ────────────────────────────────────────────

pub async fn post_create_event(
    mut req: Request,
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
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();

    let replay = form_token::consume(&db, &pp, &auth.user_id,
        token_purpose::CREATE_EVENT, &raw_token, None).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/home"));
    }

    let input = EventInput {
        title:       body.get_field("title").unwrap_or_default(),
        location:    Some(body.get_field("location").unwrap_or_default()),
        description: Some(body.get_field("description").unwrap_or_default()),
        days:        vec![DayInput {
            day_date:  body.get_field("day_date").unwrap_or_default(),
            starts_at: body.get_field("starts_at").unwrap_or_default(),
            ends_at:   body.get_field("ends_at").unwrap_or_default(),
        }],
    };

    // RFC-022: recurrence
    let freq_str  = body.get_field("repeat_rule").unwrap_or_default();
    let freq      = RecurrenceFreq::from_str(&freq_str);
    let rep_count = body.get_field("repeat_count")
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);

    let validated = match validate_event(input) {
        Ok(v)  => v,
        Err(e) => {
            let msg = render::escape_html(&e.to_string());
            return redirect(&format!("/c/{community_id}/admin/events/new?err={msg}"));
        }
    };

    // Expand recurrence from the single validated base day.
    let base_day = validated.days[0].clone();
    let expanded = match expand_recurrence(&base_day, freq, rep_count) {
        Ok(v)  => v,
        Err(e) => {
            let msg = render::escape_html(&e.to_string());
            return redirect(&format!("/c/{community_id}/admin/events/new?err={msg}"));
        }
    };

    // Convert community-local "HH:MM" on day_date to true UTC (RFC-018).
    // The community timezone determines the offset for local→UTC conversion.
    // Unknown timezone names are a hard configuration error — we must not
    // silently store wrong UTC times (P1-timezone, architect review v0.29.0).
    let community_tz = db::community::find_active(&db, community_id).await?
        .map(|c| c.timezone)
        .unwrap_or_else(|| "UTC".to_string());
    let off = match zinnias_ciao_contracts::tz::offset_minutes(&community_tz) {
        Some(o) => o,
        None => return render::page(
            "Configuration error",
            "<p style=\"color:#FF3B30\">Community timezone is not configured correctly. Please ask the operator to set a valid timezone.</p>"
        ),
    };
    let days_utc: Vec<(String, String, String)> = expanded.iter().map(|d| {
        let starts = zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.starts_at, off);
        let ends   = zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.ends_at, off);
        (d.day_date.clone(), starts, ends)
    }).collect();

    let repeat_count_stored = if freq.is_recurring() { Some(expanded.len() as u32) } else { None };
    let event_id = event_write::create_event(
        &db, community_id, &membership.membership_id,
        &validated.title,
        validated.location.as_deref(),
        validated.description.as_deref(),
        &days_utc,
        freq.as_str(),
        repeat_count_stored,
    ).await?;

    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "event", Some(&event_id), "created",
        Some(serde_json::json!({ "title": validated.title })),
    ).await;

    redirect(&format!("/c/{community_id}/events/{event_id}"))
}

// ── GET /c/:cid/admin/events/:eid/cancel ─────────────────────────────────

pub async fn get_cancel_event(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::CANCEL_EVENT, Some(event_id)).await.unwrap_or_default();

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };
    let community = db::community::find_active(&db, community_id).await?;
    let _community_name = community.map(|c| c.name).unwrap_or_default();
    let _communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let _community_pairs: Vec<(String,String)> = _communities_for_switcher.iter().map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">Cancel this event?</h1>\
         <p style=\"font-size:.9375rem;color:#6e6e73\"><strong>{title}</strong></p>\
         <p style=\"font-size:.875rem;color:#6e6e73\">Members will still see that it was cancelled.</p>\
         <div style=\"display:flex;gap:.75rem;margin-top:1.5rem\">\
           <a href=\"/c/{cid}/events/{eid}\" \
              style=\"flex:1;padding:.875rem;border:2px solid #e5e5ea;border-radius:14px;\
              text-align:center;text-decoration:none;color:#1D1D1F;font-weight:600\">\
              Keep Event</a>\
           <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/cancel\" style=\"flex:1\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               style=\"width:100%;padding:.875rem;background:#FF3B30;color:#fff;\
               border:none;border-radius:14px;font-weight:600;min-height:44px;cursor:pointer\">\
               Cancel Event</button>\
           </form>\
         </div></main>{nav}",
        header = render::header_with_switcher(i18n::JA_ADMIN_CANCEL_EVENT_TITLE, community_id, &_community_pairs),
        title  = render::escape_html(&event.title),
        cid    = render::escape_html(community_id),
        eid    = render::escape_html(event_id),
        tok    = render::escape_html(&token),
        nav    = nav,
    );
    render::page("Cancel Event", &body)
}

// ── POST /c/:cid/admin/events/:eid/cancel ────────────────────────────────

pub async fn post_cancel_event(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(&db, &pp, &auth.user_id,
        token_purpose::CANCEL_EVENT, &raw_token, Some(event_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    event_write::cancel_event(&db, event_id, &membership.membership_id).await?;
    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "event", Some(event_id), "cancelled", None).await;

    redirect(&format!("/c/{community_id}/events/{event_id}"))
}

// ── GET /c/:cid/admin/events/:eid/edit ───────────────────────────────────

pub async fn get_edit_event(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };
    if event.status == "cancelled" {
        return render::page("Cannot edit",
            "<main style=\"padding:2rem\"><p>Cancelled events cannot be edited.</p>\
             <p><a href=\"javascript:history.back()\">Back</a></p></main>");
    }
    // RFC-018: editing is only allowed while the event is still upcoming (before first day starts).
    let days = event_db::days_for_event(&db, event_id).await?;
    let now_utc = db::now_utc();
    let already_started = days.iter().any(|d| {
        classify_day(&d.starts_at_utc, &d.ends_at_utc, &now_utc) != DayTimeState::Upcoming
    });
    if already_started {
        return render::page("Cannot edit",
            "<main style=\"padding:2rem\"><p>This event has already started and cannot be edited.</p>\
             <p><a href=\"javascript:history.back()\">Back</a></p></main>");
    }

    let pp = crate::crypto::pepper(env);
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::EDIT_EVENT, Some(event_id)).await.unwrap_or_default();

    // Prefill date/time from the existing day, converted UTC → community-local.
    // Only single-day events support time editing; multi-day events edit details only.
    let is_single_day = days.len() == 1;
    let (prefill_date, prefill_start, prefill_end) = if is_single_day {
        let community_tz = db::community::find_active(&db, community_id).await?
            .map(|c| c.timezone)
            .unwrap_or_else(|| "UTC".to_string());
        // Display path: fall back to UTC for unknown zones (shows UTC times rather
        // than wrong local times; correct config should be enforced at write time).
        let off = zinnias_ciao_contracts::tz::offset_minutes_or_utc(&community_tz);
        let d = &days[0];
        let (date, start) = zinnias_ciao_contracts::tz::to_local_parts(&d.starts_at_utc, off);
        let (_, end)      = zinnias_ciao_contracts::tz::to_local_parts(&d.ends_at_utc, off);
        (Some(date), Some(start), Some(end))
    } else {
        (None, None, None)
    };

    let communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let community_pairs: Vec<(String,String)> = communities_for_switcher.iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    // Pull flash / error from query string
    let url = req.url()?;
    let err: Option<String> = url.query_pairs().find(|(k,_)| k == "err").map(|(_,v)| v.to_string());

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.5rem\">Edit Event</h1>\
         <p style=\"font-size:.8125rem;color:{muted};margin-bottom:1rem\">\
           Members will see the updated event details.</p>\
         <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/edit\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {fields}\
           <button type=\"submit\" style=\"width:100%;padding:.875rem;background:{going};\
           color:#fff;border:none;border-radius:14px;font-size:1rem;font-weight:600;\
           min-height:44px;cursor:pointer;margin-top:1rem\">Save Changes</button>\
         </form>\
         <div style=\"margin-top:1.5rem\">\
           <a href=\"/c/{cid}/events/{eid}\" \
              style=\"color:{muted};font-size:.875rem\">Back to event</a>\
         </div>\
         </main>{nav}",
        header = render::header_with_switcher("Edit Event", community_id, &community_pairs),
        cid    = render::escape_html(community_id),
        eid    = render::escape_html(event_id),
        tok    = render::escape_html(&token),
        muted  = "#6E6E73",
        going  = "#007AFF",
        fields = event_form_fields(
            Some(&event.title),
            event.location.as_deref(),
            event.description.as_deref(),
            err.as_deref(),
            prefill_date.as_deref(),
            prefill_start.as_deref(),
            prefill_end.as_deref(),
            false, // edit hides recurrence
        ),
        nav = nav,
    );
    render::page("Edit Event", &body)
}

// ── POST /c/:cid/admin/events/:eid/edit ──────────────────────────────────

pub async fn post_edit_event(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(&db, &pp, &auth.user_id,
        token_purpose::EDIT_EVENT, &raw_token, Some(event_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    // Verify event exists and belongs to community
    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };
    if event.status == "cancelled" {
        return render::not_found();
    }
    // RFC-018: reject POST edits if the event has already started.
    let days_check = event_db::days_for_event(&db, event_id).await?;
    let now_check  = db::now_utc();
    if days_check.iter().any(|d| classify_day(&d.starts_at_utc, &d.ends_at_utc, &now_check) != DayTimeState::Upcoming) {
        return render::not_found(); // same generic response — consistent with GET guard
    }

    let input = EventInput {
        title:       body.get_field("title").unwrap_or_default(),
        location:    Some(body.get_field("location").unwrap_or_default()),
        description: Some(body.get_field("description").unwrap_or_default()),
        days:        vec![zinnias_ciao_domain::DayInput {
            day_date:  body.get_field("day_date").unwrap_or_default(),
            starts_at: body.get_field("starts_at").unwrap_or_default(),
            ends_at:   body.get_field("ends_at").unwrap_or_default(),
        }],
    };

    let validated = match validate_event(input) {
        Ok(v)  => v,
        Err(e) => {
            let msg = render::escape_html(&e.to_string());
            return redirect(&format!("/c/{community_id}/admin/events/{event_id}/edit?err={msg}"));
        }
    };

    // Determine whether this is a single-day event. Per-day time editing is
    // only supported for single-day events; multi-day/recurring events edit
    // details only (RFC-040 will define multi-day edit semantics).
    let existing_days = event_db::days_for_event(&db, event_id).await?;
    let day_utc: Option<(String, String, String)> = if existing_days.len() == 1 {
        let community_tz = db::community::find_active(&db, community_id).await?
            .map(|c| c.timezone)
            .unwrap_or_else(|| "UTC".to_string());
        // Write path: unknown timezone is a hard error (P1-timezone).
        let off = match zinnias_ciao_contracts::tz::offset_minutes(&community_tz) {
            Some(o) => o,
            None => return render::page(
                "Configuration error",
                "<p style=\"color:#FF3B30\">Community timezone is not configured correctly. Please ask the operator to set a valid timezone.</p>"
            ),
        };
        let d = &validated.days[0];
        Some((
            d.day_date.clone(),
            zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.starts_at, off),
            zinnias_ciao_contracts::tz::local_to_utc(&d.day_date, &d.ends_at, off),
        ))
    } else {
        None
    };

    event_write::edit_event(
        &db, event_id,
        &validated.title,
        validated.location.as_deref(),
        validated.description.as_deref(),
        day_utc.as_ref().map(|(d, s, e)| (d.as_str(), s.as_str(), e.as_str())),
    ).await?;

    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "event", Some(event_id), "edited",
        Some(serde_json::json!({ "title": validated.title })),
    ).await;

    redirect(&format!("/c/{community_id}/events/{event_id}"))
}

// ── GET /c/:cid/admin/events/:eid/attendance ─────────────────────────────

pub async fn get_attendance(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None    => return render::not_found(),
    };
    // Only allow attendance correction after the event (status=ended or any non-scheduled)
    // For MVP we allow it for any non-cancelled event (the admin controls when to correct).
    if event.status == "cancelled" {
        return render::page("Not available",
            "<main style=\"padding:2rem\"><p>Attendance cannot be corrected for a cancelled event.</p>\
             <p><a href=\"javascript:history.back()\">Back</a></p></main>");
    }

    let days = event_db::days_for_event(&db, event_id).await?;
    let members = membership_db::list_all_active(&db, community_id).await?;

    let pp = crate::crypto::pepper(env);
    // One token per (event, admin) covers the whole batch form.
    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::ATTENDANCE_OVERRIDE, Some(event_id)).await.unwrap_or_default();

    let communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id).await.unwrap_or_default();
    let community_pairs: Vec<(String,String)> = communities_for_switcher.iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    // Build one table per day (MVP events are almost always single-day)
    let mut days_html = String::new();
    for day in &days {
        let attendances = crate::db::attendance::list_for_day(&db, &day.id).await?;
        let att_map: std::collections::HashMap<&str, Option<&str>> = attendances.iter()
            .map(|a| (a.membership_id.as_str(), a.status.as_deref()))
            .collect();

        let day_label = render::escape_html(&day.day_date);
        days_html.push_str(&format!(
            "<h3 style=\"font-size:.9375rem;font-weight:600;margin:1rem 0 .5rem\">{day_label}</h3>"
        ));

        for m in &members {
            let current = att_map.get(m.id.as_str()).copied().flatten();
            let sel = |v: &str| if current == Some(v) { " selected" } else { "" };
            days_html.push_str(&format!(
                "<div style=\"display:flex;align-items:center;gap:.75rem;padding:.5rem 0;\
                 border-bottom:1px solid #F5F5F7\">\
                 <span style=\"flex:1;font-size:.9375rem\">{name}</span>\
                 <select name=\"att_{day_id}_{mid}\" \
                   style=\"font-size:.875rem;padding:.375rem .5rem;border:1px solid #E5E5EA;\
                   border-radius:8px;min-height:44px\" \
                   aria-label=\"Attendance for {name_raw}\">\
                   <option value=\"\"{no_ans}>No answer</option>\
                   <option value=\"going\"{going}>Going</option>\
                   <option value=\"not_going\"{notgoing}>Not going</option>\
                   <option value=\"attended\"{attended}>Attended</option>\
                 </select>\
                 </div>",
                name     = render::escape_html(&m.display_name),
                name_raw = render::escape_html(&m.display_name),
                day_id   = render::escape_html(&day.id),
                mid      = render::escape_html(&m.id),
                no_ans   = if current.is_none() { " selected" } else { "" },
                going    = sel("going"),
                notgoing = sel("not_going"),
                attended = sel("attended"),
            ));
        }
    }

    let flash: Option<String> = req.url()?.query_pairs()
        .find(|(k,_)| k == "flash").map(|(_,v)| v.to_string());
    let flash_html = flash.map(|f| format!(
        "<p role=\"status\" style=\"color:#167A34;font-size:.875rem;margin-bottom:1rem\">{}</p>",
        render::escape_html(&f)
    )).unwrap_or_default();

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.25rem\">Mark Attendance</h1>\
         <p style=\"font-size:.875rem;color:#6E6E73;margin-bottom:1rem\">{title}</p>\
         {flash}\
         <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/attendance\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {days}\
           <button type=\"submit\" \
             style=\"width:100%;padding:.875rem;background:#007AFF;color:#fff;\
             border:none;border-radius:14px;font-size:1rem;font-weight:600;\
             min-height:44px;cursor:pointer;margin-top:1.5rem\">Save Attendance</button>\
         </form>\
         <div style=\"margin-top:1rem\">\
           <a href=\"/c/{cid}/events/{eid}\" style=\"color:#6E6E73;font-size:.875rem\">\
             Back to event</a>\
         </div>\
         </main>{nav}",
        header = render::header_with_switcher(i18n::JA_ADMIN_ATTEND_TITLE, community_id, &community_pairs),
        title  = render::escape_html(&event.title),
        cid    = render::escape_html(community_id),
        eid    = render::escape_html(event_id),
        tok    = render::escape_html(&token),
        days   = days_html,
        flash  = flash_html,
        nav    = nav,
    );
    render::page("Mark Attendance", &body)
}

// ── POST /c/:cid/admin/events/:eid/attendance ────────────────────────────

pub async fn post_attendance(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let form = req.form_data().await?;
    let raw_token = form.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(&db, &pp, &auth.user_id,
        token_purpose::ATTENDANCE_OVERRIDE, &raw_token, Some(event_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    // Verify event is in scope
    if event_db::find_for_community(&db, event_id, community_id).await?.is_none() {
        return render::not_found();
    }

    let days = event_db::days_for_event(&db, event_id).await?;
    let members = membership_db::list_all_active(&db, community_id).await?;

    let mut changes: u32 = 0;
    for day in &days {
        for m in &members {
            let field_name = format!("att_{}_{}", day.id, m.id);
            let value = form.get_field(&field_name).unwrap_or_default();
            let status: Option<&str> = match value.as_str() {
                "going"     => Some("going"),
                "not_going" => Some("not_going"),
                "attended"  => Some("attended"),
                _           => None, // "" → clear to No answer
            };
            crate::db::attendance::upsert(&db, &day.id, &m.id, status).await?;
            changes += 1;
        }
    }

    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "attendance", Some(event_id), "admin_override",
        Some(serde_json::json!({ "changes": changes })),
    ).await;

    redirect(&format!("/c/{community_id}/admin/events/{event_id}/attendance?flash=Saved"))
}

// ── GET /c/:cid/admin/events/:eid/notes/:mid/hide ────────────────────────
// No-JS confirmation page for admin note removal (RFC-043).

pub async fn get_admin_hide_note_confirm(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let token = form_token::issue(&db, &pp, &auth.user_id,
        token_purpose::ADMIN_HIDE_NOTE, Some(event_id)).await.unwrap_or_default();

    // Resolve the target member's display name for the confirmation copy.
    let all = membership_db::list_all_active(&db, community_id).await?;
    let target_name = all.iter()
        .find(|m| m.id == target_membership_id)
        .map(|m| m.display_name.as_str())
        .unwrap_or("this member");

    let communities = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await.unwrap_or_default();
    let pairs: Vec<(String, String)> = communities.iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone())).collect();
    let nav = render::bottom_nav(community_id, "home");

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:1rem\">Remove note?</h1>\
           <p style=\"font-size:.9375rem;color:#6E6E73;margin-bottom:1.5rem\">\
             Remove the note from {name}? This cannot be undone.</p>\
           <div style=\"display:flex;gap:.75rem\">\
             <a href=\"/c/{cid}/events/{eid}\" \
                style=\"flex:1;padding:.875rem;border:2px solid #e5e5ea;border-radius:14px;\
                text-align:center;text-decoration:none;color:#1D1D1F;font-weight:600;min-height:44px;\
                display:flex;align-items:center;justify-content:center\">Keep note</a>\
             <form method=\"post\" \
                   action=\"/c/{cid}/admin/events/{eid}/notes/{mid}/hide\" style=\"flex:1\">\
               <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
               <button type=\"submit\" \
                 style=\"width:100%;padding:.875rem;background:#FF3B30;color:#fff;\
                 border:none;border-radius:14px;font-weight:600;min-height:44px;cursor:pointer\">\
                 Remove note</button>\
             </form>\
           </div>\
         </main>{nav}",
        header = render::header_with_switcher("Remove note", community_id, &pairs),
        name   = render::escape_html(target_name),
        cid    = render::escape_html(community_id),
        eid    = render::escape_html(event_id),
        mid    = render::escape_html(target_membership_id),
        tok    = render::escape_html(&token),
        nav    = nav,
    );
    render::page("Remove note", &body)
}

// ── POST /c/:cid/admin/events/:eid/notes/:mid/hide ───────────────────────

pub async fn post_admin_hide_note(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, &env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(&env, &auth, community_id).await?;
    let db = env.d1("DB")?;
    let pp = crate::crypto::pepper(env);

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = form_token::consume(&db, &pp, &auth.user_id,
        token_purpose::ADMIN_HIDE_NOTE, &raw_token, Some(event_id)).await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    // Verify event belongs to this community
    if event_db::find_for_community(&db, event_id, community_id).await?.is_none() {
        return render::not_found();
    }

    crate::db::event_note::admin_hide(&db, event_id, target_membership_id).await?;

    // Audit without note body content (RFC-014)
    let _ = audit::write(&db, rid, Some(community_id), Some(&membership.membership_id),
        "event_note", Some(event_id), "admin_hidden",
        Some(serde_json::json!({ "target_membership_id": target_membership_id })),
    ).await;

    redirect(&format!("/c/{community_id}/events/{event_id}?flash=Note+removed"))
}

fn event_form_fields(
    title: Option<&str>,
    location: Option<&str>,
    description: Option<&str>,
    error: Option<&str>,
    day_date: Option<&str>,
    starts_at: Option<&str>,
    ends_at: Option<&str>,
    show_recurrence: bool,
) -> String {
    let err_html = error.map(|e| format!(
        "<p role=\"alert\" style=\"color:#FF3B30;font-size:.875rem\">{}</p>",
        render::escape_html(e)
    )).unwrap_or_default();

    let field = |label: &str, name: &str, ftype: &str, val: &str, required: bool| {
        let req_attr = if required { " required" } else { "" };
        format!(
            "<label style=\"display:block;margin-bottom:1rem\">\
             <span style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">{label}</span>\
             <input type=\"{ftype}\" name=\"{name}\" value=\"{val}\" \
               style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
               border-radius:12px;font-size:1rem\"{req_attr}>\
             </label>",
            label = label,
            ftype = ftype,
            name  = name,
            val   = render::escape_html(val),
        )
    };

    // RFC-022: repeat fields (create only — edit hides recurrence).
    let repeat_html = if show_recurrence {
        format!(
        "<div style=\"margin-bottom:1rem\">\
         <label style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">{repeat_lbl}</label>\
         <div style=\"display:flex;gap:.75rem;align-items:center\">\
           <select name=\"repeat_rule\" style=\"padding:.625rem;border:1px solid #e5e5ea;\
             border-radius:12px;font-size:1rem;flex:1\">\
             <option value=\"none\">{opt_none}</option>\
             <option value=\"weekly\">{opt_weekly}</option>\
             <option value=\"biweekly\">{opt_biweekly}</option>\
             <option value=\"monthly\">{opt_monthly}</option>\
           </select>\
           <input type=\"number\" name=\"repeat_count\" value=\"8\" min=\"1\" max=\"52\"\
             style=\"width:5rem;padding:.625rem;border:1px solid #e5e5ea;\
             border-radius:12px;font-size:1rem\">\
           <span style=\"font-size:.875rem;color:#6e6e73\">{unit}</span>\
         </div>\
         <p style=\"font-size:.75rem;color:#6e6e73;margin:.25rem 0 0\">{hint}</p>\
         </div>",
        repeat_lbl = i18n::JA_REPEAT_LABEL,
        opt_none   = i18n::JA_REPEAT_NONE,
        opt_weekly = i18n::JA_REPEAT_WEEKLY,
        opt_biweekly = i18n::JA_REPEAT_BIWEEKLY,
        opt_monthly  = i18n::JA_REPEAT_MONTHLY,
        unit       = i18n::JA_REPEAT_COUNT_UNIT,
        hint       = i18n::JA_REPEAT_COUNT_HINT,
        )
    } else {
        String::new()
    };

    format!(
        "{err}\
         {title}\
         {date}\
         {start}\
         {end}\
         {loc}\
         {repeat}\
         {desc}",
        err    = err_html,
        title  = field("Title", "title", "text", title.unwrap_or(""), true),
        date   = field("Date", "day_date", "date", day_date.unwrap_or(""), true),
        start  = field("Start time", "starts_at", "time", starts_at.unwrap_or(""), true),
        end    = field("End time", "ends_at", "time", ends_at.unwrap_or(""), true),
        repeat = repeat_html,
        loc    = field("Location (optional)", "location", "text",
                      location.unwrap_or(""), false),
        desc  = {
            let dval = render::escape_html(description.unwrap_or(""));
            format!(
                "<label style=\"display:block;margin-bottom:1rem\">\
                 <span style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">\
                 Description (optional)</span>\
                 <textarea name=\"description\" rows=\"3\" \
                   style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
                   border-radius:12px;font-size:1rem\">{dval}</textarea>\
                 </label>"
            )
        },
    )
}
